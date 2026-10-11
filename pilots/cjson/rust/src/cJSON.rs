//! Idiomatic port (Stage 2) of `cJSON.c` / `cJSON.h` @ 6d9f244, cJSON 1.7.19.
//!
//! One [`Store`] owns every node; a [`NodeId`] is a generational `Copy` handle (RULEBOOK 6.1). Allocation
//! sites report to the process-global [`AllocHooks`] seam with the C sizes in the C order (RULEBOOK 6.2, D1).
//! Strings are bytes (`&[u8]`, read up to the first NUL as C `strlen`), never `&str` (RULEBOOK 1).

use std::cell::Cell;
use std::sync::{Arc, PoisonError, RwLock};

/// `CJSON_VERSION_MAJOR` .. `CJSON_VERSION_PATCH` rendered as C's `cJSON_Version()`.
pub const CJSON_VERSION: &[u8] = b"1.7.19";
/// `CJSON_NESTING_LIMIT` (`cJSON.h:134-140`): parse and print depth guard.
pub const CJSON_NESTING_LIMIT: usize = 1000;
/// `CJSON_CIRCULAR_LIMIT` (`cJSON.h:142-144`): `duplicate` depth guard.
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

/// `sizeof(cJSON)` on LP64 (`cJSON.c:243`): the size of every node allocation event.
const NODE_SIZE: usize = 64;
/// Largest `int` (`INT_MAX`), the cap of the print buffer (Q6).
const INT_MAX: usize = i32::MAX as usize;

/// The bits of the C `type` word (`cJSON.h:96-115`).
pub mod flags {
    pub const INVALID: u32 = 0;
    pub const FALSE: u32 = 1 << 0;
    pub const TRUE: u32 = 1 << 1;
    pub const NULL: u32 = 1 << 2;
    pub const NUMBER: u32 = 1 << 3;
    pub const STRING: u32 = 1 << 4;
    pub const ARRAY: u32 = 1 << 5;
    pub const OBJECT: u32 = 1 << 6;
    pub const RAW: u32 = 1 << 7;
    pub const IS_REFERENCE: u32 = 256;
    pub const STRING_IS_CONST: u32 = 512;
}
use flags::{ARRAY, FALSE, IS_REFERENCE, NULL, NUMBER, OBJECT, RAW, STRING, STRING_IS_CONST, TRUE};

// ---- allocation hooks (RULEBOOK 6.2, D1) ---------------------------------------------------------------------

/// The seam that replaces `cJSON_Hooks`: it decides whether a C allocation site succeeds and accounts for it.
pub trait AllocHooks: Send + Sync {
    /// Called at each C allocation site with C's size; `false` = this allocation fails.
    fn allocate(&self, size: usize) -> bool;
    /// Called at each C free site of a live allocation with the size given to `allocate`.
    fn deallocate(&self, size: usize);
}

static HOOKS: RwLock<Option<Arc<dyn AllocHooks>>> = RwLock::new(None);

/// `cJSON_InitHooks`: `None` restores the defaults (no events). Process-global, as `cJSON.c:186`.
// DEVIATION(port): D1
pub fn init_hooks(hooks: Option<Arc<dyn AllocHooks>>) {
    *HOOKS.write().unwrap_or_else(PoisonError::into_inner) = hooks;
}

fn current_hooks() -> Option<Arc<dyn AllocHooks>> {
    HOOKS.read().unwrap_or_else(PoisonError::into_inner).clone()
}

/// One C `hooks->allocate(size)` call; `false` = the allocation failed (C returned NULL).
pub(crate) fn alloc_event(size: usize) -> bool {
    match current_hooks() {
        Some(hooks) => hooks.allocate(size),
        None => true,
    }
}

/// One C `hooks->deallocate(ptr)` call of a non-NULL pointer.
pub(crate) fn free_event(size: usize) {
    if let Some(hooks) = current_hooks() {
        hooks.deallocate(size);
    }
}

// ---- parse error position (RULEBOOK 6.3, D9, Q26) -------------------------------------------------------------

thread_local! {
    static ERROR_OFFSET: Cell<Option<usize>> = const { Cell::new(None) };
}

/// `cJSON_GetErrorPtr()` as an offset into the last parsed input (`None` = NULL).
// DEVIATION(port): D9
pub fn get_error_offset() -> Option<usize> {
    ERROR_OFFSET.with(Cell::get)
}

/// A successful parse: the root and the parse end (`*return_parse_end`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parsed {
    pub root: NodeId,
    pub end: usize,
}

/// A failed parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// The input was NULL: nothing is written, not even the parse end.
    NoInput,
    /// The error offset, also the parse end (`cJSON.c:1209-1212`).
    At(usize),
}

// ---- bytes helpers ---------------------------------------------------------------------------------------------

/// `strlen`: the bytes before the first NUL (the whole slice when there is none).
fn c_len(bytes: &[u8]) -> usize {
    bytes.iter().position(|&c| c == 0).unwrap_or(bytes.len())
}

/// The C string held by `bytes`.
pub(crate) fn c_str(bytes: &[u8]) -> &[u8] {
    &bytes[..c_len(bytes)]
}

/// `cJSON_strdup` (`cJSON.c:192-210`): `strlen + 1` bytes, one allocation event, `None` on failure or NULL input.
pub(crate) fn strdup(string: Option<&[u8]>) -> Option<Vec<u8>> {
    let string = c_str(string?);
    if !alloc_event(string.len() + 1) {
        return None;
    }
    let mut copy = Vec::with_capacity(string.len() + 1);
    copy.extend_from_slice(string);
    copy.push(0);
    Some(copy)
}

// ---- nodes and the store (RULEBOOK 6.1) --------------------------------------------------------------------

/// A generational handle of a node in a [`Store`]; equal iff the same node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    index: u32,
    generation: u32,
}

/// A string field of a node: `valuestring` or the key `string`.
#[derive(Clone, Debug)]
pub(crate) enum Text {
    /// NULL.
    None,
    /// A library allocation; its length is the size given to `allocate` (the value ends at the first NUL).
    Owned(Vec<u8>),
    /// The caller's string (`create_string_reference`, `add_item_to_object_cs`), never copied or freed.
    Static(&'static [u8]),
    /// The `valuestring` of another node that a reference shares (`create_reference`, `cJSON.c:2020`).
    Shared(NodeId),
}

/// The C fields of `struct cJSON`.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) next: Option<NodeId>,
    pub(crate) prev: Option<NodeId>,
    pub(crate) child: Option<NodeId>,
    pub(crate) type_word: u32,
    pub(crate) valuestring: Text,
    pub(crate) valueint: i32,
    pub(crate) valuedouble: f64,
    pub(crate) string: Text,
}

impl Node {
    pub(crate) fn zeroed() -> Node {
        Node {
            next: None,
            prev: None,
            child: None,
            type_word: 0,
            valuestring: Text::None,
            valueint: 0,
            valuedouble: 0.0,
            string: Text::None,
        }
    }
}

struct Slot {
    generation: u32,
    node: Option<Node>,
}

/// Owner of every node. Nodes move between trees of one store; a handle of a deleted node is inert.
#[derive(Default)]
pub struct Store {
    slots: Vec<Slot>,
    free: Vec<u32>,
}

/// A string returned by a print function (or by `find_pointer_from_object_to`): dropping it is the caller's
/// `cJSON_free`, one deallocation event (RULEBOOK 6.2).
#[derive(Debug)]
pub struct Printed {
    data: Vec<u8>,
}

impl Printed {
    pub(crate) fn from_vec(data: Vec<u8>) -> Printed {
        Printed { data }
    }

    /// The text: the bytes before the first NUL.
    pub fn bytes(&self) -> &[u8] {
        c_str(&self.data)
    }

    /// Writable view of the text (D21); a NUL written inside ends the text there.
    // DEVIATION(port): D21
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        let n = c_len(&self.data);
        &mut self.data[..n]
    }
}

impl Drop for Printed {
    fn drop(&mut self) {
        free_event(self.data.len());
    }
}

/// Key argument of `add_item_to_object`: copied (`cJSON_AddItemToObject`) or kept (`...CS`).
enum KeyArg<'a> {
    Copy(&'a [u8]),
    Const(&'static [u8]),
}

fn is_hex_digit_value(c: u8) -> Option<u32> {
    match c {
        b'0'..=b'9' => Some(u32::from(c - b'0')),
        b'A'..=b'F' => Some(10 + u32::from(c - b'A')),
        b'a'..=b'f' => Some(10 + u32::from(c - b'a')),
        _ => None,
    }
}

impl Store {
    /// An empty store.
    pub const fn new() -> Store {
        Store {
            slots: Vec::new(),
            free: Vec::new(),
        }
    }

    // ---- slot plumbing ---------------------------------------------------------------------------------------

    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        let slot = self.slots.get(id.index as usize)?;
        if slot.generation == id.generation {
            slot.node.as_ref()
        } else {
            None
        }
    }

    pub(crate) fn node_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation == id.generation {
            slot.node.as_mut()
        } else {
            None
        }
    }

    fn n(&self, id: Option<NodeId>) -> Option<&Node> {
        id.and_then(|i| self.node(i))
    }

    fn insert(&mut self, node: Node) -> NodeId {
        if let Some(index) = self.free.pop() {
            if let Some(slot) = self.slots.get_mut(index as usize) {
                slot.node = Some(node);
                return NodeId {
                    index,
                    generation: slot.generation,
                };
            }
        }
        // 2^32 live nodes would need 64 GiB of slots: the index cannot overflow in any run.
        let index = self.slots.len() as u32;
        self.slots.push(Slot {
            generation: 0,
            node: Some(node),
        });
        NodeId {
            index,
            generation: 0,
        }
    }

    /// Remove a node without touching its strings or children (a C `free(node)`); no event.
    fn remove(&mut self, id: NodeId) -> Option<Node> {
        let slot = self.slots.get_mut(id.index as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let node = slot.node.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.index);
        Some(node)
    }

    /// `cJSON_New_Item` (`cJSON.c:240-252`): allocation event of 64 bytes, a zeroed node.
    pub(crate) fn new_item(&mut self) -> Option<NodeId> {
        if !alloc_event(NODE_SIZE) {
            return None;
        }
        Some(self.insert(Node::zeroed()))
    }

    /// `cJSON_free(node)` of a node whose strings and children were moved elsewhere (`overwrite_item` users).
    pub(crate) fn free_node_only(&mut self, id: NodeId) {
        if self.remove(id).is_some() {
            free_event(NODE_SIZE);
        }
    }

    /// A copy of the node's fields (the C struct copy used by `overwrite_item`).
    pub(crate) fn node_copy(&self, id: NodeId) -> Option<Node> {
        self.node(id).cloned()
    }

    /// `memcpy(root, &replacement, sizeof(cJSON))`.
    pub(crate) fn overwrite_fields(&mut self, id: NodeId, node: Node) {
        if let Some(slot) = self.node_mut(id) {
            *slot = node;
        }
    }

    pub(crate) fn next_of(&self, id: Option<NodeId>) -> Option<NodeId> {
        self.n(id).and_then(|n| n.next)
    }

    pub(crate) fn prev_of(&self, id: Option<NodeId>) -> Option<NodeId> {
        self.n(id).and_then(|n| n.prev)
    }

    pub(crate) fn child_of(&self, id: Option<NodeId>) -> Option<NodeId> {
        self.n(id).and_then(|n| n.child)
    }

    pub(crate) fn set_next(&mut self, id: NodeId, value: Option<NodeId>) {
        if let Some(n) = self.node_mut(id) {
            n.next = value;
        }
    }

    pub(crate) fn set_prev(&mut self, id: NodeId, value: Option<NodeId>) {
        if let Some(n) = self.node_mut(id) {
            n.prev = value;
        }
    }

    pub(crate) fn set_child(&mut self, id: NodeId, value: Option<NodeId>) {
        if let Some(n) = self.node_mut(id) {
            n.child = value;
        }
    }

    /// The type word, `None` for NULL or a deleted handle.
    pub(crate) fn type_word(&self, id: Option<NodeId>) -> Option<u32> {
        self.n(id).map(|n| n.type_word)
    }

    pub(crate) fn set_type_word(&mut self, id: NodeId, value: u32) {
        if let Some(n) = self.node_mut(id) {
            n.type_word = value;
        }
    }

    /// The bytes behind a string field (a reference resolves to the shared buffer), whole buffer.
    fn backing<'a>(&'a self, text: &'a Text) -> Option<&'a [u8]> {
        match text {
            Text::None => None,
            Text::Owned(v) => Some(v),
            Text::Static(s) => Some(s),
            Text::Shared(other) => match self.node(*other).map(|n| &n.valuestring) {
                Some(Text::Owned(v)) => Some(v),
                Some(Text::Static(s)) => Some(s),
                _ => None,
            },
        }
    }

    /// `valuestring` as a C string.
    pub(crate) fn value_bytes(&self, id: Option<NodeId>) -> Option<&[u8]> {
        let node = self.n(id)?;
        self.backing(&node.valuestring).map(c_str)
    }

    /// The key (`string`) as a C string.
    pub(crate) fn key_bytes(&self, id: Option<NodeId>) -> Option<&[u8]> {
        let node = self.n(id)?;
        self.backing(&node.string).map(c_str)
    }

    // ---- delete (D10: no recursion) ---------------------------------------------------------------------------

    /// `cJSON_Delete`: the node and every node after it in its list, with their children (`cJSON.c:259-282`).
    pub fn delete(&mut self, item: Option<NodeId>) {
        if let Some(id) = item {
            self.delete_list(id);
        }
    }

    // DEVIATION(port): D10
    fn delete_list(&mut self, start: NodeId) {
        enum Work {
            List(NodeId),
            Finish(NodeId),
        }
        let mut stack = vec![Work::List(start)];
        while let Some(work) = stack.pop() {
            match work {
                Work::List(id) => {
                    // A stale handle: C would read freed memory; here nothing happens.
                    let Some(node) = self.node(id) else { continue };
                    let (next, child) = (node.next, node.child);
                    let is_reference = node.type_word & IS_REFERENCE != 0;
                    if let Some(next) = next {
                        stack.push(Work::List(next));
                    }
                    stack.push(Work::Finish(id));
                    if !is_reference {
                        if let Some(child) = child {
                            stack.push(Work::List(child));
                        }
                    }
                }
                Work::Finish(id) => {
                    let Some(node) = self.remove(id) else {
                        continue;
                    };
                    // DEVIATION(port): D12
                    if node.type_word & IS_REFERENCE == 0 {
                        if let Text::Owned(v) = &node.valuestring {
                            free_event(v.len());
                        }
                    }
                    if node.type_word & STRING_IS_CONST == 0 {
                        if let Text::Owned(v) = &node.string {
                            free_event(v.len());
                        }
                    }
                    free_event(NODE_SIZE);
                }
            }
        }
    }

    // ---- type predicates and field access -------------------------------------------------------------------------

    fn is_kind(&self, item: Option<NodeId>, kind: u32) -> bool {
        self.n(item).is_some_and(|n| n.type_word & 0xFF == kind)
    }

    pub fn is_invalid(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, flags::INVALID)
    }
    pub fn is_false(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, FALSE)
    }
    pub fn is_true(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, TRUE)
    }
    pub fn is_bool(&self, item: Option<NodeId>) -> bool {
        self.n(item)
            .is_some_and(|n| n.type_word & (TRUE | FALSE) != 0)
    }
    pub fn is_null(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, NULL)
    }
    pub fn is_number(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, NUMBER)
    }
    pub fn is_string(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, STRING)
    }
    pub fn is_array(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, ARRAY)
    }
    pub fn is_object(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, OBJECT)
    }
    pub fn is_raw(&self, item: Option<NodeId>) -> bool {
        self.is_kind(item, RAW)
    }

    /// The full type word (kind bit plus `IS_REFERENCE` / `STRING_IS_CONST`), `None` for a deleted handle.
    pub fn node_type(&self, item: NodeId) -> Option<u32> {
        self.type_word(Some(item))
    }

    pub fn value_int(&self, item: NodeId) -> Option<i32> {
        self.node(item).map(|n| n.valueint)
    }

    pub fn value_double(&self, item: NodeId) -> Option<f64> {
        self.node(item).map(|n| n.valuedouble)
    }

    /// The key (`string` member), `None` when NULL.
    pub fn key(&self, item: NodeId) -> Option<&[u8]> {
        self.key_bytes(Some(item))
    }

    /// The `valuestring` member (also for `Raw`), `None` when NULL.
    pub fn value_string(&self, item: NodeId) -> Option<&[u8]> {
        self.value_bytes(Some(item))
    }

    /// Address of the value buffer (0 for NULL): a reference keeps the caller's buffer (cJSON.c:455-473).
    pub fn value_string_addr(&self, item: NodeId) -> usize {
        let Some(node) = self.node(item) else {
            return 0;
        };
        self.backing(&node.valuestring)
            .map_or(0, |b| b.as_ptr() as usize)
    }

    /// Write the `valueint` field only (Q3).
    pub fn set_value_int(&mut self, item: NodeId, value: i32) {
        if let Some(n) = self.node_mut(item) {
            n.valueint = value;
        }
    }

    /// Write the `valuedouble` field only (Q3).
    pub fn set_value_double(&mut self, item: NodeId, value: f64) {
        if let Some(n) = self.node_mut(item) {
            n.valuedouble = value;
        }
    }

    /// `cJSON_GetStringValue` as a writable view (D21): the bytes before the NUL of a string's buffer. `None`
    /// for a non-string, no value, or a reference (the caller's buffer, D13).
    // DEVIATION(port): D21
    pub fn string_value_mut(&mut self, item: Option<NodeId>) -> Option<&mut [u8]> {
        let node = self.node_mut(item?)?;
        // DEVIATION(port): D13
        if node.type_word & 0xFF != STRING || node.type_word & IS_REFERENCE != 0 {
            return None;
        }
        match &mut node.valuestring {
            Text::Owned(v) => {
                let n = c_len(v);
                Some(&mut v[..n])
            }
            _ => None,
        }
    }

    /// `child` link.
    pub fn child(&self, item: NodeId) -> Option<NodeId> {
        self.child_of(Some(item))
    }

    /// `next` link.
    pub fn next(&self, item: NodeId) -> Option<NodeId> {
        self.next_of(Some(item))
    }

    /// `prev` link: the first sibling's `prev` is the last sibling, a node in no list has none.
    pub fn prev(&self, item: NodeId) -> Option<NodeId> {
        self.prev_of(Some(item))
    }

    /// `cJSON_ArrayForEach`: the children of `item` in order.
    pub fn children(&self, item: Option<NodeId>) -> Vec<NodeId> {
        let mut out = Vec::new();
        let mut cur = self.child_of(item);
        while let Some(c) = cur {
            out.push(c);
            cur = self.next_of(Some(c));
        }
        out
    }

    /// `cJSON_GetStringValue`.
    pub fn get_string_value(&self, item: Option<NodeId>) -> Option<&[u8]> {
        if !self.is_string(item) {
            return None;
        }
        self.value_bytes(item)
    }

    /// `cJSON_GetNumberValue`: NaN for a non-number.
    // QUIRK(port): Q24
    pub fn get_number_value(&self, item: Option<NodeId>) -> f64 {
        if !self.is_number(item) {
            return f64::NAN;
        }
        self.n(item).map_or(f64::NAN, |n| n.valuedouble)
    }

    /// `cJSON_GetArraySize`.
    // DEVIATION(port): D17
    pub fn get_array_size(&self, array: Option<NodeId>) -> usize {
        let mut size = 0;
        let mut child = self.child_of(array);
        while let Some(c) = child {
            size += 1;
            child = self.next_of(Some(c));
        }
        size
    }

    /// `get_array_item` (`cJSON.c:1932-1948`).
    pub fn get_array_item(&self, array: Option<NodeId>, index: usize) -> Option<NodeId> {
        self.n(array)?;
        let mut current = self.child_of(array);
        let mut index = index;
        while let Some(c) = current {
            if index == 0 {
                break;
            }
            index -= 1;
            current = self.next_of(Some(c));
        }
        current
    }

    /// `get_object_item` (`cJSON.c:1951-1981`).
    // QUIRK(port): Q1
    // QUIRK(port): Q2
    fn get_object_item_impl(
        &self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        case_sensitive: bool,
    ) -> Option<NodeId> {
        self.n(object)?;
        let name = c_str(name?);
        let mut current = self.child_of(object);
        loop {
            let c = current?;
            let node = self.node(c)?;
            match self.key_bytes(Some(c)) {
                None => {
                    if case_sensitive {
                        return None;
                    }
                }
                Some(key) => {
                    // DEVIATION(port): D19
                    let equal = if case_sensitive {
                        key == name
                    } else {
                        key.eq_ignore_ascii_case(name)
                    };
                    if equal {
                        return Some(c);
                    }
                }
            }
            current = node.next;
        }
    }

    /// `cJSON_GetObjectItem` (case-insensitive).
    pub fn get_object_item(&self, object: Option<NodeId>, name: Option<&[u8]>) -> Option<NodeId> {
        self.get_object_item_impl(object, name, false)
    }

    /// `cJSON_GetObjectItemCaseSensitive`.
    pub fn get_object_item_case_sensitive(
        &self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        self.get_object_item_impl(object, name, true)
    }

    /// `cJSON_HasObjectItem`.
    pub fn has_object_item(&self, object: Option<NodeId>, name: Option<&[u8]>) -> bool {
        self.get_object_item(object, name).is_some()
    }

    // ---- create ------------------------------------------------------------------------------------------------

    fn create_kind(&mut self, type_word: u32) -> Option<NodeId> {
        let id = self.new_item()?;
        self.set_type_word(id, type_word);
        Some(id)
    }

    pub fn create_null(&mut self) -> Option<NodeId> {
        self.create_kind(NULL)
    }

    pub fn create_true(&mut self) -> Option<NodeId> {
        self.create_kind(TRUE)
    }

    pub fn create_false(&mut self) -> Option<NodeId> {
        self.create_kind(FALSE)
    }

    pub fn create_bool(&mut self, boolean: bool) -> Option<NodeId> {
        self.create_kind(if boolean { TRUE } else { FALSE })
    }

    pub fn create_array(&mut self) -> Option<NodeId> {
        self.create_kind(ARRAY)
    }

    pub fn create_object(&mut self) -> Option<NodeId> {
        self.create_kind(OBJECT)
    }

    /// `valueint` of a number: cJSON's explicit saturation, then the cast (`cJSON.c:389-400`).
    // QUIRK(port): Q5
    // DEVIATION(port): D6
    pub(crate) fn saturate(number: f64) -> i32 {
        if number >= f64::from(i32::MAX) {
            i32::MAX
        } else if number <= f64::from(i32::MIN) {
            i32::MIN
        } else {
            number as i32
        }
    }

    pub fn create_number(&mut self, number: f64) -> Option<NodeId> {
        let id = self.create_kind(NUMBER)?;
        if let Some(n) = self.node_mut(id) {
            n.valuedouble = number;
            n.valueint = Store::saturate(number);
        }
        Some(id)
    }

    /// A node with `type_word` and an owned copy of `string`; on a failed or NULL copy the node is deleted.
    fn create_with_text(&mut self, type_word: u32, string: Option<&[u8]>) -> Option<NodeId> {
        let id = self.create_kind(type_word)?;
        match strdup(string) {
            Some(copy) => {
                if let Some(n) = self.node_mut(id) {
                    n.valuestring = Text::Owned(copy);
                }
                Some(id)
            }
            None => {
                self.delete(Some(id));
                None
            }
        }
    }

    pub fn create_string(&mut self, string: Option<&[u8]>) -> Option<NodeId> {
        self.create_with_text(STRING, string)
    }

    pub fn create_raw(&mut self, raw: Option<&[u8]>) -> Option<NodeId> {
        self.create_with_text(RAW, raw)
    }

    /// `cJSON_CreateStringReference`: the caller's bytes are kept without a copy.
    // DEVIATION(port): D13
    pub fn create_string_reference(&mut self, string: Option<&'static [u8]>) -> Option<NodeId> {
        let id = self.create_kind(STRING | IS_REFERENCE)?;
        if let (Some(n), Some(s)) = (self.node_mut(id), string) {
            n.valuestring = Text::Static(c_str(s));
        }
        Some(id)
    }

    /// `cJSON_CreateObjectReference`: `child` is shared, not copied (D12).
    // DEVIATION(port): D12
    pub fn create_object_reference(&mut self, child: Option<NodeId>) -> Option<NodeId> {
        let id = self.create_kind(OBJECT | IS_REFERENCE)?;
        self.set_child(id, child);
        Some(id)
    }

    // DEVIATION(port): D12
    pub fn create_array_reference(&mut self, child: Option<NodeId>) -> Option<NodeId> {
        let id = self.create_kind(ARRAY | IS_REFERENCE)?;
        self.set_child(id, child);
        Some(id)
    }

    /// The shared tail of the four `cJSON_Create*Array` functions (`cJSON.c:2610-2780`): the array node, then
    /// each element created by `make`; a failed element deletes the array.
    fn create_array_of(
        &mut self,
        count: usize,
        mut make: impl FnMut(&mut Store, usize) -> Option<NodeId>,
    ) -> Option<NodeId> {
        let array = self.create_array()?;
        let mut previous: Option<NodeId> = None;
        let mut last: Option<NodeId> = None;
        for i in 0..count {
            let Some(n) = make(self, i) else {
                self.delete(Some(array));
                return None;
            };
            match previous {
                None => self.set_child(array, Some(n)),
                Some(p) => {
                    self.set_next(p, Some(n));
                    self.set_prev(n, Some(p));
                }
            }
            previous = Some(n);
            last = Some(n);
        }
        if let Some(first) = self.child_of(Some(array)) {
            self.set_prev(first, last);
        }
        Some(array)
    }

    pub fn create_int_array(&mut self, numbers: &[i32]) -> Option<NodeId> {
        self.create_array_of(numbers.len(), |s, i| s.create_number(f64::from(numbers[i])))
    }

    pub fn create_float_array(&mut self, numbers: &[f32]) -> Option<NodeId> {
        self.create_array_of(numbers.len(), |s, i| s.create_number(f64::from(numbers[i])))
    }

    pub fn create_double_array(&mut self, numbers: &[f64]) -> Option<NodeId> {
        self.create_array_of(numbers.len(), |s, i| s.create_number(numbers[i]))
    }

    pub fn create_string_array(&mut self, strings: &[&[u8]]) -> Option<NodeId> {
        self.create_array_of(strings.len(), |s, i| s.create_string(Some(strings[i])))
    }

    // ---- add / detach / insert / replace --------------------------------------------------------------------------

    /// `create_reference` (`cJSON.c:2008-2024`): a new node that shares `item`'s child and value buffer.
    fn create_reference(&mut self, item: Option<NodeId>) -> Option<NodeId> {
        let source = self.n(item)?.clone();
        let reference = self.new_item()?;
        // DEVIATION(port): D12
        let valuestring = match (&source.valuestring, item) {
            (Text::Owned(_), Some(owner)) => Text::Shared(owner),
            (other, _) => other.clone(),
        };
        if let Some(n) = self.node_mut(reference) {
            *n = Node {
                next: None,
                prev: None,
                child: source.child,
                type_word: source.type_word | IS_REFERENCE,
                valuestring,
                valueint: source.valueint,
                valuedouble: source.valuedouble,
                string: Text::None,
            };
        }
        Some(reference)
    }

    /// `add_item_to_array` (`cJSON.c:2027-2058`).
    pub(crate) fn add_item_to_array_impl(
        &mut self,
        array: Option<NodeId>,
        item: Option<NodeId>,
    ) -> bool {
        let (Some(array), Some(item)) = (array, item) else {
            return false;
        };
        if array == item || self.node(array).is_none() || self.node(item).is_none() {
            return false;
        }
        match self.child_of(Some(array)) {
            None => {
                self.set_child(array, Some(item));
                self.set_prev(item, Some(item));
                self.set_next(item, None);
            }
            Some(first) => {
                // DEVIATION(port): D14
                if let Some(last) = self.prev_of(Some(first)) {
                    self.set_next(last, Some(item));
                    self.set_prev(item, Some(last));
                    self.set_prev(first, Some(item));
                }
            }
        }
        true
    }

    /// `cJSON_AddItemToArray`.
    pub fn add_item_to_array(&mut self, array: Option<NodeId>, item: Option<NodeId>) -> bool {
        self.add_item_to_array_impl(array, item)
    }

    /// `add_item_to_object` (`cJSON.c:2079-2121`).
    fn add_item_to_object_impl(
        &mut self,
        object: Option<NodeId>,
        key: Option<KeyArg<'_>>,
        item: Option<NodeId>,
    ) -> bool {
        let (Some(object), Some(key), Some(item)) = (object, key, item) else {
            return false;
        };
        if object == item || self.node(object).is_none() {
            return false;
        }
        let Some(item_type) = self.node(item).map(|n| n.type_word) else {
            return false;
        };
        let (new_key, new_type) = match key {
            KeyArg::Const(s) => (Text::Static(c_str(s)), item_type | STRING_IS_CONST),
            KeyArg::Copy(s) => match strdup(Some(s)) {
                Some(copy) => (Text::Owned(copy), item_type & !STRING_IS_CONST),
                None => return false,
            },
        };
        if let Some(n) = self.node_mut(item) {
            if item_type & STRING_IS_CONST == 0 {
                if let Text::Owned(old) = &n.string {
                    free_event(old.len());
                }
            }
            n.string = new_key;
            n.type_word = new_type;
        }
        self.add_item_to_array_impl(Some(object), Some(item))
    }

    /// `cJSON_AddItemToObject`: the key is copied.
    pub fn add_item_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        item: Option<NodeId>,
    ) -> bool {
        self.add_item_to_object_impl(object, name.map(KeyArg::Copy), item)
    }

    /// `cJSON_AddItemToObjectCS`: the key is kept by reference.
    // DEVIATION(port): D13
    pub fn add_item_to_object_cs(
        &mut self,
        object: Option<NodeId>,
        name: Option<&'static [u8]>,
        item: Option<NodeId>,
    ) -> bool {
        self.add_item_to_object_impl(object, name.map(KeyArg::Const), item)
    }

    /// `cJSON_AddItemReferenceToArray`.
    pub fn add_item_reference_to_array(
        &mut self,
        array: Option<NodeId>,
        item: Option<NodeId>,
    ) -> bool {
        if array.is_none() {
            return false;
        }
        let reference = self.create_reference(item);
        self.add_item_to_array_impl(array, reference)
    }

    /// `cJSON_AddItemReferenceToObject`.
    pub fn add_item_reference_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        item: Option<NodeId>,
    ) -> bool {
        if object.is_none() || name.is_none() {
            return false;
        }
        let reference = self.create_reference(item);
        self.add_item_to_object_impl(object, name.map(KeyArg::Copy), reference)
    }

    /// Shared tail of the `cJSON_Add*ToObject` helpers (`cJSON.c:2123-2260`): a failed add deletes the item.
    fn add_created(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        created: Option<NodeId>,
    ) -> Option<NodeId> {
        if self.add_item_to_object_impl(object, name.map(KeyArg::Copy), created) {
            return created;
        }
        self.delete(created);
        None
    }

    pub fn add_null_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_null();
        self.add_created(object, name, item)
    }

    pub fn add_true_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_true();
        self.add_created(object, name, item)
    }

    pub fn add_false_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_false();
        self.add_created(object, name, item)
    }

    pub fn add_bool_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        boolean: bool,
    ) -> Option<NodeId> {
        let item = self.create_bool(boolean);
        self.add_created(object, name, item)
    }

    pub fn add_number_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        number: f64,
    ) -> Option<NodeId> {
        let item = self.create_number(number);
        self.add_created(object, name, item)
    }

    pub fn add_string_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        string: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_string(string);
        self.add_created(object, name, item)
    }

    pub fn add_raw_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        raw: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_raw(raw);
        self.add_created(object, name, item)
    }

    pub fn add_object_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_object();
        self.add_created(object, name, item)
    }

    pub fn add_array_to_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let item = self.create_array();
        self.add_created(object, name, item)
    }

    /// `cJSON_DetachItemViaPointer` (`cJSON.c:2256-2292`).
    pub fn detach_item_via_pointer(
        &mut self,
        parent: Option<NodeId>,
        item: Option<NodeId>,
    ) -> Option<NodeId> {
        let (parent, item) = (parent?, item?);
        let first = self.child_of(Some(parent));
        let node = self.node(item)?;
        let (item_next, item_prev) = (node.next, node.prev);
        self.node(parent)?;
        let is_first = first == Some(item);
        // DEVIATION(port): D14
        if !is_first && item_prev.is_none() {
            return None;
        }
        if !is_first {
            if let Some(p) = item_prev {
                self.set_next(p, item_next);
            }
        }
        if let Some(n) = item_next {
            self.set_prev(n, item_prev);
        }
        if is_first {
            self.set_child(parent, item_next);
        } else if item_next.is_none() {
            if let Some(f) = first {
                self.set_prev(f, item_prev);
            }
        }
        self.set_prev(item, None);
        self.set_next(item, None);
        Some(item)
    }

    pub fn detach_item_from_array(
        &mut self,
        array: Option<NodeId>,
        which: usize,
    ) -> Option<NodeId> {
        let item = self.get_array_item(array, which);
        self.detach_item_via_pointer(array, item)
    }

    pub fn delete_item_from_array(&mut self, array: Option<NodeId>, which: usize) {
        let detached = self.detach_item_from_array(array, which);
        self.delete(detached);
    }

    pub fn detach_item_from_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let found = self.get_object_item(object, name);
        self.detach_item_via_pointer(object, found)
    }

    pub fn detach_item_from_object_case_sensitive(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) -> Option<NodeId> {
        let found = self.get_object_item_case_sensitive(object, name);
        self.detach_item_via_pointer(object, found)
    }

    pub fn delete_item_from_object(&mut self, object: Option<NodeId>, name: Option<&[u8]>) {
        let detached = self.detach_item_from_object(object, name);
        self.delete(detached);
    }

    pub fn delete_item_from_object_case_sensitive(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
    ) {
        let detached = self.detach_item_from_object_case_sensitive(object, name);
        self.delete(detached);
    }

    /// `cJSON_InsertItemInArray` (`cJSON.c:2334-2367`).
    pub fn insert_item_in_array(
        &mut self,
        array: Option<NodeId>,
        which: usize,
        new_item: Option<NodeId>,
    ) -> bool {
        let Some(new_item) = new_item else {
            return false;
        };
        let Some(after) = self.get_array_item(array, which) else {
            return self.add_item_to_array_impl(array, Some(new_item));
        };
        let Some(array) = array else { return false };
        let first = self.child_of(Some(array));
        let Some(after_prev) = self.prev_of(Some(after)) else {
            // DEVIATION(port): D14
            if first != Some(after) {
                return false;
            }
            return self.insert_before(array, after, new_item, None);
        };
        self.insert_before(array, after, new_item, Some(after_prev))
    }

    fn insert_before(
        &mut self,
        array: NodeId,
        after: NodeId,
        new_item: NodeId,
        after_prev: Option<NodeId>,
    ) -> bool {
        if self.node(new_item).is_none() {
            return false;
        }
        self.set_next(new_item, Some(after));
        self.set_prev(new_item, after_prev);
        self.set_prev(after, Some(new_item));
        if self.child_of(Some(array)) == Some(after) {
            self.set_child(array, Some(new_item));
        } else if let Some(p) = self.prev_of(Some(new_item)) {
            self.set_next(p, Some(new_item));
        }
        true
    }

    /// `cJSON_ReplaceItemViaPointer` (`cJSON.c:2370-2416`).
    pub fn replace_item_via_pointer(
        &mut self,
        parent: Option<NodeId>,
        item: Option<NodeId>,
        replacement: Option<NodeId>,
    ) -> bool {
        let (Some(parent), Some(item), Some(replacement)) = (parent, item, replacement) else {
            return false;
        };
        let Some(first) = self.child_of(Some(parent)) else {
            return false;
        };
        if replacement == item {
            return true;
        }
        let (Some(item_node), Some(_)) = (self.node(item), self.node(replacement)) else {
            return false;
        };
        let (item_next, item_prev) = (item_node.next, item_node.prev);
        self.set_next(replacement, item_next);
        self.set_prev(replacement, item_prev);
        if let Some(n) = item_next {
            self.set_prev(n, Some(replacement));
        }
        if first == item {
            if self.prev_of(Some(first)) == Some(first) {
                self.set_prev(replacement, Some(replacement));
            }
            self.set_child(parent, Some(replacement));
        } else {
            if let Some(p) = self.prev_of(Some(replacement)) {
                self.set_next(p, Some(replacement));
            }
            if self.next_of(Some(replacement)).is_none() {
                self.set_prev(first, Some(replacement));
            }
        }
        self.set_next(item, None);
        self.set_prev(item, None);
        self.delete(Some(item));
        true
    }

    pub fn replace_item_in_array(
        &mut self,
        array: Option<NodeId>,
        which: usize,
        new_item: Option<NodeId>,
    ) -> bool {
        let item = self.get_array_item(array, which);
        self.replace_item_via_pointer(array, item, new_item)
    }

    /// `replace_item_in_object` (`cJSON.c:2418-2448`).
    // QUIRK(port): Q16
    fn replace_item_in_object_impl(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        replacement: Option<NodeId>,
        case_sensitive: bool,
    ) -> bool {
        let (Some(replacement), Some(name)) = (replacement, name) else {
            return false;
        };
        let Some(node) = self.node(replacement) else {
            return false;
        };
        if node.type_word & STRING_IS_CONST == 0 {
            if let Text::Owned(old) = &node.string {
                free_event(old.len());
            }
        }
        let copy = strdup(Some(name));
        let failed = copy.is_none();
        if let Some(n) = self.node_mut(replacement) {
            n.string = copy.map_or(Text::None, Text::Owned);
        }
        if failed {
            return false;
        }
        if let Some(n) = self.node_mut(replacement) {
            n.type_word &= !STRING_IS_CONST;
        }
        let target = self.get_object_item_impl(object, Some(name), case_sensitive);
        self.replace_item_via_pointer(object, target, Some(replacement))
    }

    pub fn replace_item_in_object(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        new_item: Option<NodeId>,
    ) -> bool {
        self.replace_item_in_object_impl(object, name, new_item, false)
    }

    pub fn replace_item_in_object_case_sensitive(
        &mut self,
        object: Option<NodeId>,
        name: Option<&[u8]>,
        new_item: Option<NodeId>,
    ) -> bool {
        self.replace_item_in_object_impl(object, name, new_item, true)
    }

    // ---- setters (the header macros, M2) --------------------------------------------------------------------

    /// `cJSON_SetNumberHelper`: NaN for NULL.
    // QUIRK(port): Q24
    pub fn set_number_helper(&mut self, object: Option<NodeId>, number: f64) -> f64 {
        let Some(id) = object else { return f64::NAN };
        let Some(n) = self.node_mut(id) else {
            return f64::NAN;
        };
        n.valueint = Store::saturate(number);
        n.valuedouble = number;
        n.valuedouble
    }

    /// The `cJSON_SetNumberValue` macro: NULL returns `number`.
    pub fn set_number_value(&mut self, object: Option<NodeId>, number: f64) -> f64 {
        if object.is_none() {
            return number;
        }
        self.set_number_helper(object, number)
    }

    /// The `cJSON_SetIntValue` macro (`cJSON.h:281`): both fields, no saturation.
    // DEVIATION(port): D6
    pub fn set_int_value(&mut self, object: Option<NodeId>, number: f64) -> f64 {
        let Some(n) = object.and_then(|id| self.node_mut(id)) else {
            return number;
        };
        n.valuedouble = number;
        n.valueint = number as i32;
        f64::from(n.valueint)
    }

    /// The `cJSON_SetBoolValue` macro: the new type word, `INVALID` (0) when the item is not a bool.
    pub fn set_bool_value(&mut self, object: Option<NodeId>, boolean: bool) -> u32 {
        let Some(n) = object.and_then(|id| self.node_mut(id)) else {
            return flags::INVALID;
        };
        if n.type_word & (FALSE | TRUE) != 0 {
            n.type_word = (n.type_word & !(FALSE | TRUE)) | if boolean { TRUE } else { FALSE };
            n.type_word
        } else {
            flags::INVALID
        }
    }

    /// `cJSON_SetValuestring` (`cJSON.c:445-473`): the new value, `None` on failure.
    pub fn set_valuestring(
        &mut self,
        object: Option<NodeId>,
        valuestring: Option<&[u8]>,
    ) -> Option<&[u8]> {
        let id = object?;
        let node = self.node(id)?;
        if node.type_word & STRING == 0 || node.type_word & IS_REFERENCE != 0 {
            return None;
        }
        let new_value = c_str(valuestring?);
        let old_len = match &node.valuestring {
            Text::Owned(v) => c_len(v),
            _ => return None,
        };
        if new_value.len() <= old_len {
            // DEVIATION(port): D7
            if let Some(Node {
                valuestring: Text::Owned(v),
                ..
            }) = self.node_mut(id)
            {
                let n = new_value.len();
                if let Some(dst) = v.get_mut(..n) {
                    dst.copy_from_slice(new_value);
                }
                if let Some(terminator) = v.get_mut(n) {
                    *terminator = 0;
                }
            }
            return self.value_bytes(Some(id));
        }
        let copy = strdup(Some(new_value))?;
        if let Some(n) = self.node_mut(id) {
            if let Text::Owned(old) = &n.valuestring {
                free_event(old.len());
            }
            n.valuestring = Text::Owned(copy);
        }
        self.value_bytes(Some(id))
    }

    // ---- duplicate (D10: iterative; D11) ---------------------------------------------------------------------

    /// The per-node part of `cJSON_Duplicate_rec` (`cJSON.c:2800-2850`): node, value string, key.
    fn duplicate_node(&mut self, source: Option<NodeId>) -> Option<NodeId> {
        let src = self.n(source)?.clone();
        let value = self.backing(&src.valuestring).map(|b| c_str(b).to_vec());
        let key = self.backing(&src.string).map(|b| c_str(b).to_vec());
        let new_item = self.new_item()?;
        if let Some(n) = self.node_mut(new_item) {
            n.type_word = src.type_word & !IS_REFERENCE;
            n.valueint = src.valueint;
            n.valuedouble = src.valuedouble;
        }
        if let Some(value) = value {
            match strdup(Some(&value)) {
                Some(copy) => {
                    if let Some(n) = self.node_mut(new_item) {
                        n.valuestring = Text::Owned(copy);
                    }
                }
                None => {
                    self.delete(Some(new_item));
                    return None;
                }
            }
        }
        if key.is_some() {
            let text = if src.type_word & STRING_IS_CONST != 0 {
                src.string.clone()
            } else {
                match strdup(key.as_deref()) {
                    Some(copy) => Text::Owned(copy),
                    None => {
                        self.delete(Some(new_item));
                        return None;
                    }
                }
            };
            if let Some(n) = self.node_mut(new_item) {
                n.string = text;
            }
        }
        Some(new_item)
    }

    /// `cJSON_Duplicate`.
    // DEVIATION(port): D10
    // DEVIATION(port): D11
    pub fn duplicate(&mut self, item: Option<NodeId>, recurse: bool) -> Option<NodeId> {
        struct Frame {
            new_item: NodeId,
            cursor: Option<NodeId>,
            last: Option<NodeId>,
            depth: usize,
        }
        enum Step {
            Descend(Option<NodeId>, usize),
            Advance,
            Returned(Option<NodeId>),
        }
        let mut stack: Vec<Frame> = Vec::new();
        let mut step = Step::Descend(item, 0);
        loop {
            step = match step {
                Step::Descend(source, depth) => match self.duplicate_node(source) {
                    None => Step::Returned(None),
                    Some(new_item) if !recurse => Step::Returned(Some(new_item)),
                    Some(new_item) => {
                        stack.push(Frame {
                            new_item,
                            cursor: self.child_of(source),
                            last: None,
                            depth,
                        });
                        Step::Advance
                    }
                },
                Step::Advance => {
                    let top = stack.last_mut()?;
                    match top.cursor {
                        Some(child) => {
                            if top.depth >= CJSON_CIRCULAR_LIMIT {
                                let new_item = top.new_item;
                                stack.pop();
                                self.delete(Some(new_item));
                                Step::Returned(None)
                            } else {
                                Step::Descend(Some(child), top.depth + 1)
                            }
                        }
                        None => {
                            let (new_item, last) = (top.new_item, top.last);
                            stack.pop();
                            if let Some(first) = self.child_of(Some(new_item)) {
                                self.set_prev(first, last);
                            }
                            Step::Returned(Some(new_item))
                        }
                    }
                }
                Step::Returned(result) => {
                    let Some(top) = stack.last_mut() else {
                        return result;
                    };
                    match result {
                        None => {
                            let new_item = top.new_item;
                            stack.pop();
                            self.delete(Some(new_item));
                            Step::Returned(None)
                        }
                        Some(new_child) => {
                            let (parent, last, cursor) = (top.new_item, top.last, top.cursor);
                            top.last = Some(new_child);
                            top.cursor = self.next_of(cursor);
                            match last {
                                Some(l) => {
                                    self.set_next(l, Some(new_child));
                                    self.set_prev(new_child, Some(l));
                                }
                                None => self.set_child(parent, Some(new_child)),
                            }
                            Step::Advance
                        }
                    }
                }
            };
        }
    }

    // ---- compare ---------------------------------------------------------------------------------------------

    /// `cJSON_Compare` (`cJSON.c:3066-3190`).
    // QUIRK(port): Q17
    // QUIRK(port): Q25
    pub fn compare(&self, a: Option<NodeId>, b: Option<NodeId>, case_sensitive: bool) -> bool {
        let (Some(na), Some(nb)) = (self.n(a), self.n(b)) else {
            return false;
        };
        let kind = na.type_word & 0xFF;
        if kind != nb.type_word & 0xFF {
            return false;
        }
        if !matches!(
            kind,
            FALSE | TRUE | NULL | NUMBER | STRING | RAW | ARRAY | OBJECT
        ) {
            return false;
        }
        if a == b {
            return true;
        }
        match kind {
            FALSE | TRUE | NULL => true,
            NUMBER => compare_double(na.valuedouble, nb.valuedouble),
            STRING | RAW => match (self.value_bytes(a), self.value_bytes(b)) {
                (Some(x), Some(y)) => x == y,
                _ => false,
            },
            ARRAY => {
                let (mut ae, mut be) = (na.child, nb.child);
                while let (Some(x), Some(y)) = (ae, be) {
                    if !self.compare(Some(x), Some(y), case_sensitive) {
                        return false;
                    }
                    ae = self.next_of(Some(x));
                    be = self.next_of(Some(y));
                }
                ae == be
            }
            OBJECT => {
                // QUIRK(port): Q1
                for (from, to) in [(a, b), (b, a)] {
                    let mut element = self.child_of(from);
                    while let Some(e) = element {
                        let Some(other) =
                            self.get_object_item_impl(to, self.key_bytes(Some(e)), case_sensitive)
                        else {
                            return false;
                        };
                        if !self.compare(Some(e), Some(other), case_sensitive) {
                            return false;
                        }
                        element = self.next_of(Some(e));
                    }
                }
                true
            }
            _ => false,
        }
    }

    // ---- parse (RULEBOOK 6.3) -------------------------------------------------------------------------------

    /// `cJSON_Parse`: NULL input leaves the error position unchanged (Q26).
    pub fn parse(&mut self, value: Option<&[u8]>) -> Result<Parsed, ParseError> {
        self.parse_with_opts(value, false)
    }

    /// `cJSON_ParseWithOpts`: `value` is the caller's buffer including its terminating NUL; the length is
    /// `strlen + 1` (Q13). NULL input leaves the error position unchanged (Q26).
    // QUIRK(port): Q13
    // QUIRK(port): Q26
    pub fn parse_with_opts(
        &mut self,
        value: Option<&[u8]>,
        require_null_terminated: bool,
    ) -> Result<Parsed, ParseError> {
        let Some(buffer) = value else {
            return Err(ParseError::NoInput);
        };
        let length = c_len(buffer) + 1;
        self.parse_with_length_opts(value, length, require_null_terminated)
    }

    /// `cJSON_ParseWithLength`.
    pub fn parse_with_length(
        &mut self,
        value: Option<&[u8]>,
        buffer_length: usize,
    ) -> Result<Parsed, ParseError> {
        self.parse_with_length_opts(value, buffer_length, false)
    }

    /// `cJSON_ParseWithLengthOpts` (`cJSON.c:1146-1223`). A length beyond `value` is cut to it.
    // QUIRK(port): Q12
    // DEVIATION(port): D8
    pub fn parse_with_length_opts(
        &mut self,
        value: Option<&[u8]>,
        buffer_length: usize,
        require_null_terminated: bool,
    ) -> Result<Parsed, ParseError> {
        ERROR_OFFSET.with(|e| e.set(None));
        let Some(content) = value else {
            return Err(ParseError::NoInput);
        };
        let length = buffer_length.min(content.len());
        let mut parser = Parser {
            store: self,
            content,
            length,
            offset: 0,
            depth: 0,
        };
        let mut root: Option<NodeId> = None;
        let ok = 'run: {
            if length == 0 {
                break 'run false;
            }
            root = parser.store.new_item();
            let Some(item) = root else { break 'run false };
            parser.skip_utf8_bom();
            parser.skip_whitespace();
            if !parser.parse_value(item) {
                break 'run false;
            }
            if require_null_terminated {
                parser.skip_whitespace();
                if parser.offset >= parser.length || parser.byte(parser.offset) != 0 {
                    break 'run false;
                }
            }
            true
        };
        let offset = parser.offset;
        match root {
            Some(item) if ok => Ok(Parsed {
                root: item,
                end: offset,
            }),
            _ => {
                self.delete(root);
                let position = if offset < length {
                    offset
                } else {
                    length.saturating_sub(1)
                };
                ERROR_OFFSET.with(|e| e.set(Some(position)));
                Err(ParseError::At(position))
            }
        }
    }

    // ---- print (RULEBOOK 6.2, Q6) ---------------------------------------------------------------------------

    /// `print` (`cJSON.c:1239-1304`): 256-byte buffer, shrunk to `offset + 1` at the end.
    // QUIRK(port): Q6
    fn print_item(&self, item: Option<NodeId>, format: bool) -> Option<Printed> {
        const DEFAULT_BUFFER_SIZE: usize = 256;
        if !alloc_event(DEFAULT_BUFFER_SIZE) {
            return None;
        }
        let mut buffer = PrintBuffer {
            buffer: Some(vec![0; DEFAULT_BUFFER_SIZE]),
            length: DEFAULT_BUFFER_SIZE,
            offset: 0,
            depth: 0,
            noalloc: false,
            format,
        };
        if !self.print_value(item, &mut buffer) {
            if buffer.buffer.is_some() {
                free_event(buffer.length);
            }
            return None;
        }
        buffer.update_offset();
        let size = buffer.offset + 1;
        if !alloc_event(size) {
            free_event(buffer.length);
            return None;
        }
        let mut printed = buffer.buffer.take().unwrap_or_default();
        printed.resize(size, 0);
        printed[size - 1] = 0;
        free_event(buffer.length);
        Some(Printed::from_vec(printed))
    }

    /// `cJSON_Print`.
    pub fn print(&self, item: Option<NodeId>) -> Option<Printed> {
        self.print_item(item, true)
    }

    /// `cJSON_PrintUnformatted`.
    pub fn print_unformatted(&self, item: Option<NodeId>) -> Option<Printed> {
        self.print_item(item, false)
    }

    /// `cJSON_PrintBuffered`: the buffer is returned as it is, not shrunk (Q6).
    pub fn print_buffered(
        &self,
        item: Option<NodeId>,
        prebuffer: usize,
        format: bool,
    ) -> Option<Printed> {
        if !alloc_event(prebuffer) {
            return None;
        }
        let mut buffer = PrintBuffer {
            buffer: Some(vec![0; prebuffer]),
            length: prebuffer,
            offset: 0,
            depth: 0,
            noalloc: false,
            format,
        };
        if !self.print_value(item, &mut buffer) {
            if buffer.buffer.is_some() {
                free_event(buffer.length);
            }
            return None;
        }
        buffer.buffer.map(Printed::from_vec)
    }

    /// `cJSON_PrintPreallocated`: `buffer` is the caller's buffer of `length` bytes; the bytes written so far
    /// stay in it after a failure (Q6).
    pub fn print_preallocated(
        &self,
        item: Option<NodeId>,
        buffer: &mut [u8],
        format: bool,
    ) -> bool {
        let mut work = PrintBuffer {
            buffer: Some(buffer.to_vec()),
            length: buffer.len(),
            offset: 0,
            depth: 0,
            noalloc: true,
            format,
        };
        let ok = self.print_value(item, &mut work);
        if let Some(written) = &work.buffer {
            buffer.copy_from_slice(written);
        }
        ok
    }

    /// `print_value` (`cJSON.c:1431-1549`).
    fn print_value(&self, item: Option<NodeId>, out: &mut PrintBuffer) -> bool {
        let Some(node) = self.n(item) else {
            return false;
        };
        match node.type_word & 0xFF {
            NULL => out.put_text(5, b"null\0"),
            FALSE => out.put_text(6, b"false\0"),
            TRUE => out.put_text(5, b"true\0"),
            NUMBER => print_number(node.valueint, node.valuedouble, out),
            RAW => {
                let Some(raw) = self.value_bytes(item) else {
                    return false;
                };
                let Some(at) = out.ensure(raw.len() + 1) else {
                    return false;
                };
                out.write(at, raw);
                out.write(at + raw.len(), &[0]);
                true
            }
            STRING => print_string_ptr(self.value_bytes(item), out),
            ARRAY => self.print_array(node, out),
            OBJECT => self.print_object(node, out),
            _ => false,
        }
    }

    /// `print_array` (`cJSON.c:1595-1659`).
    // QUIRK(port): Q14
    fn print_array(&self, node: &Node, out: &mut PrintBuffer) -> bool {
        if out.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        let Some(at) = out.ensure(1) else {
            return false;
        };
        out.write(at, b"[");
        out.offset += 1;
        out.depth += 1;
        let mut current = node.child;
        while let Some(element) = current {
            if !self.print_value(Some(element), out) {
                return false;
            }
            out.update_offset();
            let next = self.next_of(Some(element));
            if next.is_some() {
                let length = if out.format { 2 } else { 1 };
                let Some(at) = out.ensure(length + 1) else {
                    return false;
                };
                if out.format {
                    out.write(at, b", \0");
                } else {
                    out.write(at, b",\0");
                }
                out.offset += length;
            }
            current = next;
        }
        let Some(at) = out.ensure(2) else {
            return false;
        };
        out.write(at, b"]\0");
        out.depth -= 1;
        true
    }

    /// `print_object` (`cJSON.c:1780-1896`).
    // QUIRK(port): Q14
    fn print_object(&self, node: &Node, out: &mut PrintBuffer) -> bool {
        if out.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        let length = if out.format { 2 } else { 1 };
        let Some(at) = out.ensure(length + 1) else {
            return false;
        };
        out.write(at, b"{");
        out.depth += 1;
        if out.format {
            out.write(at + 1, b"\n");
        }
        out.offset += length;
        let mut current = node.child;
        while let Some(element) = current {
            if out.format {
                let Some(at) = out.ensure(out.depth) else {
                    return false;
                };
                let tabs = vec![b'\t'; out.depth];
                out.write(at, &tabs);
                out.offset += out.depth;
            }
            if !print_string_ptr(self.key_bytes(Some(element)), out) {
                return false;
            }
            out.update_offset();
            let length = if out.format { 2 } else { 1 };
            let Some(at) = out.ensure(length) else {
                return false;
            };
            out.write(at, b":");
            if out.format {
                out.write(at + 1, b"\t");
            }
            out.offset += length;
            if !self.print_value(Some(element), out) {
                return false;
            }
            out.update_offset();
            let next = self.next_of(Some(element));
            let length = usize::from(out.format) + usize::from(next.is_some());
            let Some(at) = out.ensure(length + 1) else {
                return false;
            };
            let mut tail = Vec::with_capacity(3);
            if next.is_some() {
                tail.push(b',');
            }
            if out.format {
                tail.push(b'\n');
            }
            tail.push(0);
            out.write(at, &tail);
            out.offset += length;
            current = next;
        }
        let needed = if out.format { out.depth + 1 } else { 2 };
        let Some(at) = out.ensure(needed) else {
            return false;
        };
        let mut tail = Vec::new();
        if out.format {
            tail.resize(out.depth - 1, b'\t');
        }
        tail.extend_from_slice(b"}\0");
        out.write(at, &tail);
        out.depth -= 1;
        true
    }
}

/// `cJSON_Version`.
pub fn version() -> &'static [u8] {
    CJSON_VERSION
}

/// `compare_double` (`cJSON.c:589-593`): relative `DBL_EPSILON`, not exact.
// QUIRK(port): Q3
pub(crate) fn compare_double(a: f64, b: f64) -> bool {
    let max_val = a.abs().max(b.abs());
    (a - b).abs() <= max_val * f64::EPSILON
}

// ---- print buffer ------------------------------------------------------------------------------------------------

/// The C `printbuffer`. `buffer` is `None` after a failed growth (C: NULL).
struct PrintBuffer {
    buffer: Option<Vec<u8>>,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: bool,
    format: bool,
}

impl PrintBuffer {
    /// `ensure` (`cJSON.c:490-573`): the write position for `needed` more bytes, growing as Q6 says.
    // QUIRK(port): Q6
    fn ensure(&mut self, needed: usize) -> Option<usize> {
        self.buffer.as_ref()?;
        if self.length > 0 && self.offset >= self.length {
            return None;
        }
        if needed > INT_MAX {
            return None;
        }
        let needed = needed + self.offset + 1;
        if needed <= self.length {
            return Some(self.offset);
        }
        if self.noalloc {
            return None;
        }
        let new_size = if needed > INT_MAX / 2 {
            if needed <= INT_MAX {
                INT_MAX
            } else {
                return None;
            }
        } else {
            needed * 2
        };
        // With hooks installed C has no `reallocate`: allocate(new), copy, deallocate(old) (cJSON.c:555-568).
        if !alloc_event(new_size) {
            free_event(self.length);
            self.length = 0;
            self.buffer = None;
            return None;
        }
        if let Some(buffer) = self.buffer.as_mut() {
            buffer.resize(new_size, 0);
        }
        free_event(self.length);
        self.length = new_size;
        Some(self.offset)
    }

    fn write(&mut self, at: usize, bytes: &[u8]) {
        if let Some(dst) = self
            .buffer
            .as_mut()
            .and_then(|b| b.get_mut(at..at + bytes.len()))
        {
            dst.copy_from_slice(bytes);
        }
    }

    /// `ensure(needed)` then write `text` (it carries its NUL) at the offset, without advancing it.
    fn put_text(&mut self, needed: usize, text: &[u8]) -> bool {
        let Some(at) = self.ensure(needed) else {
            return false;
        };
        self.write(at, text);
        true
    }

    /// `update_offset` (`cJSON.c:577-587`): advance by the `strlen` of what was written at the offset.
    fn update_offset(&mut self) {
        let Some(buffer) = self.buffer.as_ref() else {
            return;
        };
        let tail = buffer.get(self.offset..).unwrap_or(&[]);
        self.offset += c_len(tail);
    }
}

/// `print_number` (`cJSON.c:596-663`).
// QUIRK(port): Q3
// QUIRK(port): Q4
fn print_number(valueint: i32, d: f64, out: &mut PrintBuffer) -> bool {
    let decimal_point = get_decimal_point();
    let mut text: Vec<u8> = if d.is_nan() || d.is_infinite() {
        b"null".to_vec()
    } else if d == f64::from(valueint) {
        valueint.to_string().into_bytes()
    } else {
        // DEVIATION(port): D4
        let short = format_g(d, 15);
        // DEVIATION(port): D5
        let round_trips = short
            .parse::<f64>()
            .is_ok_and(|test| compare_double(test, d));
        if round_trips {
            short.into_bytes()
        } else {
            format_g(d, 17).into_bytes()
        }
    };
    // A 26-byte number buffer in C: more than 25 characters is an error.
    if text.len() > 25 {
        return false;
    }
    let Some(at) = out.ensure(text.len() + 1) else {
        return false;
    };
    for byte in &mut text {
        if *byte == decimal_point {
            *byte = b'.';
        }
    }
    text.push(0);
    let length = text.len() - 1;
    out.write(at, &text);
    out.offset += length;
    true
}

/// `get_decimal_point` (`cJSON.c:281-292`).
// DEVIATION(port): D3
fn get_decimal_point() -> u8 {
    #[cfg(feature = "locales")]
    {
        b'.'
    }
    #[cfg(not(feature = "locales"))]
    {
        b'.'
    }
}

/// C's `printf("%1.<precision>g", d)` in the C locale for a finite `d` (RULEBOOK I7, D4): `precision`
/// significant digits from the correctly rounded `{:e}`, exponent form when the exponent is below -4 or at
/// least `precision`, trailing zeros removed, at least two exponent digits.
fn format_g(d: f64, precision: usize) -> String {
    let sign = if d.is_sign_negative() { "-" } else { "" };
    if d == 0.0 {
        return format!("{sign}0");
    }
    let a = d.abs();
    let scientific = format!("{:.*e}", precision - 1, a);
    let Some((mantissa, exponent)) = scientific.split_once('e') else {
        return scientific;
    };
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let strip = |s: &str| -> String {
        if s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else {
            s.to_string()
        }
    };
    if exponent < -4 || exponent >= precision as i32 {
        let sign_char = if exponent < 0 { '-' } else { '+' };
        format!("{sign}{}e{sign_char}{:02}", strip(mantissa), exponent.abs())
    } else {
        let decimals = (precision as i32 - 1 - exponent).max(0) as usize;
        format!("{sign}{}", strip(&format!("{a:.decimals$}")))
    }
}

/// `print_string_ptr` (`cJSON.c:954-1073`).
// QUIRK(port): Q11
fn print_string_ptr(input: Option<&[u8]>, out: &mut PrintBuffer) -> bool {
    let Some(input) = input else {
        return out.put_text(3, b"\"\"\0");
    };
    let input = c_str(input);
    let escapes: usize = input
        .iter()
        .map(|&c| match c {
            b'"' | b'\\' | 8 | 12 | b'\n' | b'\r' | b'\t' => 1,
            c if c < 32 => 5,
            _ => 0,
        })
        .sum();
    let output_length = input.len() + escapes;
    let Some(at) = out.ensure(output_length + 3) else {
        return false;
    };
    let mut text = Vec::with_capacity(output_length + 3);
    text.push(b'"');
    if escapes == 0 {
        text.extend_from_slice(input);
    } else {
        for &c in input {
            if c > 31 && c != b'"' && c != b'\\' {
                text.push(c);
                continue;
            }
            text.push(b'\\');
            match c {
                b'\\' => text.push(b'\\'),
                b'"' => text.push(b'"'),
                8 => text.push(b'b'),
                12 => text.push(b'f'),
                b'\n' => text.push(b'n'),
                b'\r' => text.push(b'r'),
                b'\t' => text.push(b't'),
                _ => {
                    // "u%04x" of a byte below 32.
                    const HEX: &[u8; 16] = b"0123456789abcdef";
                    text.extend_from_slice(b"u00");
                    text.push(HEX[usize::from(c >> 4)]);
                    text.push(HEX[usize::from(c & 15)]);
                }
            }
        }
    }
    text.extend_from_slice(b"\"\0");
    out.write(at, &text);
    true
}

// ---- parser --------------------------------------------------------------------------------------------------------

/// The C `parse_buffer` plus the store it fills.
struct Parser<'a> {
    store: &'a mut Store,
    content: &'a [u8],
    length: usize,
    offset: usize,
    depth: usize,
}

impl Parser<'_> {
    /// `content[index]`; every C read is guarded by `length <= content.len()`, 0 is a safe stand-in.
    fn byte(&self, index: usize) -> u8 {
        self.content.get(index).copied().unwrap_or(0)
    }

    fn can_read(&self, size: usize) -> bool {
        self.offset + size <= self.length
    }

    fn can_access(&self, index: usize) -> bool {
        self.offset + index < self.length
    }

    /// `buffer_skip_whitespace` (`cJSON.c:1090-1113`).
    // QUIRK(port): Q7
    fn skip_whitespace(&mut self) {
        if !self.can_access(0) {
            return;
        }
        while self.can_access(0) && self.byte(self.offset) <= 32 {
            self.offset += 1;
        }
        if self.offset == self.length {
            self.offset -= 1;
        }
    }

    /// `skip_utf8_bom` (`cJSON.c:1116-1129`).
    // QUIRK(port): Q8
    fn skip_utf8_bom(&mut self) {
        if self.offset != 0 {
            return;
        }
        if self.can_access(4) && self.content.get(..3) == Some(b"\xEF\xBB\xBF".as_slice()) {
            self.offset += 3;
        }
    }

    /// `parse_number` (`cJSON.c:307-408`).
    // QUIRK(port): Q9
    // QUIRK(port): Q5
    fn parse_number(&mut self, item: NodeId) -> bool {
        let decimal_point = get_decimal_point();
        let mut number_string_length = 0;
        let mut has_decimal_point = false;
        while self.offset + number_string_length < self.length {
            match self.byte(self.offset + number_string_length) {
                b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {}
                b'.' => has_decimal_point = true,
                _ => break,
            }
            number_string_length += 1;
        }
        // The scanned copy is a C allocation of `length + 1` bytes.
        if !alloc_event(number_string_length + 1) {
            return false;
        }
        let mut copy: Vec<u8> =
            self.content[self.offset..self.offset + number_string_length].to_vec();
        if has_decimal_point {
            for byte in &mut copy {
                if *byte == b'.' {
                    *byte = decimal_point;
                }
            }
        }
        // DEVIATION(port): D5
        let Some((number, consumed)) = strtod_prefix(&copy) else {
            free_event(number_string_length + 1);
            return false;
        };
        if let Some(n) = self.store.node_mut(item) {
            n.valuedouble = number;
            n.valueint = Store::saturate(number);
            n.type_word = NUMBER;
        }
        self.offset += consumed;
        free_event(number_string_length + 1);
        true
    }

    /// `parse_string` (`cJSON.c:823-951`).
    // QUIRK(port): Q10
    // QUIRK(port): Q11
    // QUIRK(port): Q12
    fn parse_string(&mut self, item: NodeId) -> bool {
        let start = self.offset;
        let mut input_pointer = start + 1;
        let mut input_end = start + 1;
        let mut allocated: Option<usize> = None;
        let mut output: Vec<u8> = Vec::new();
        let ok = 'run: {
            if self.byte(start) != b'"' {
                break 'run false;
            }
            let mut skipped_bytes = 0;
            while input_end < self.length && self.byte(input_end) != b'"' {
                if self.byte(input_end) == b'\\' {
                    if input_end + 1 >= self.length {
                        break 'run false;
                    }
                    skipped_bytes += 1;
                    input_end += 1;
                }
                input_end += 1;
            }
            if input_end >= self.length || self.byte(input_end) != b'"' {
                break 'run false;
            }
            let allocation_length = (input_end - start) - skipped_bytes;
            if !alloc_event(allocation_length + 1) {
                break 'run false;
            }
            allocated = Some(allocation_length + 1);
            output.reserve(allocation_length + 1);
            while input_pointer < input_end {
                if self.byte(input_pointer) != b'\\' {
                    output.push(self.byte(input_pointer));
                    input_pointer += 1;
                    continue;
                }
                let mut sequence_length = 2;
                match self.byte(input_pointer + 1) {
                    b'b' => output.push(8),
                    b'f' => output.push(12),
                    b'n' => output.push(b'\n'),
                    b'r' => output.push(b'\r'),
                    b't' => output.push(b'\t'),
                    b'"' | b'\\' | b'/' => output.push(self.byte(input_pointer + 1)),
                    b'u' => {
                        sequence_length = utf16_literal_to_utf8(
                            self.content,
                            input_pointer,
                            input_end,
                            &mut output,
                        );
                        if sequence_length == 0 {
                            break 'run false;
                        }
                    }
                    _ => break 'run false,
                }
                input_pointer += sequence_length;
            }
            true
        };
        if ok {
            let size = allocated.unwrap_or(0);
            output.push(0);
            output.resize(size, 0);
            if let Some(n) = self.store.node_mut(item) {
                n.type_word = STRING;
                n.valuestring = Text::Owned(output);
            }
            self.offset = input_end + 1;
            return true;
        }
        if let Some(size) = allocated {
            free_event(size);
        }
        self.offset = input_pointer;
        false
    }

    /// `parse_value` (`cJSON.c:1384-1429`).
    fn parse_value(&mut self, item: NodeId) -> bool {
        let at = self.offset;
        let rest = self
            .content
            .get(at..self.length.min(self.content.len()))
            .unwrap_or(&[]);
        if self.can_read(4) && rest.starts_with(b"null") {
            self.set_kind(item, NULL, 0);
            self.offset += 4;
            return true;
        }
        if self.can_read(5) && rest.starts_with(b"false") {
            self.set_kind(item, FALSE, 0);
            self.offset += 5;
            return true;
        }
        if self.can_read(4) && rest.starts_with(b"true") {
            // QUIRK(port): Q5
            self.set_kind(item, TRUE, 1);
            self.offset += 4;
            return true;
        }
        if !self.can_access(0) {
            return false;
        }
        match self.byte(at) {
            b'"' => self.parse_string(item),
            b'-' | b'0'..=b'9' => self.parse_number(item),
            b'[' => self.parse_array(item),
            b'{' => self.parse_object(item),
            _ => false,
        }
    }

    fn set_kind(&mut self, item: NodeId, kind: u32, valueint: i32) {
        if let Some(n) = self.store.node_mut(item) {
            n.type_word = kind;
            if kind == TRUE {
                n.valueint = valueint;
            }
        }
    }

    /// Append `new_item` to the list being built (`cJSON.c:1536-1546`).
    fn link(&mut self, head: &mut Option<NodeId>, current: &mut Option<NodeId>, new_item: NodeId) {
        match *current {
            None => *head = Some(new_item),
            Some(c) => {
                self.store.set_next(c, Some(new_item));
                self.store.set_prev(new_item, Some(c));
            }
        }
        *current = Some(new_item);
    }

    /// The `success:` / `fail:` tail shared by `parse_array` and `parse_object`.
    fn finish_container(
        &mut self,
        item: NodeId,
        kind: u32,
        ok: bool,
        head: Option<NodeId>,
        current: Option<NodeId>,
    ) -> bool {
        if !ok {
            if let Some(h) = head {
                self.store.delete_list(h);
            }
            return false;
        }
        self.depth -= 1;
        if let (Some(h), Some(c)) = (head, current) {
            self.store.set_prev(h, Some(c));
        }
        if let Some(n) = self.store.node_mut(item) {
            n.type_word = kind;
            n.child = head;
        }
        self.offset += 1;
        true
    }

    /// `parse_array` (`cJSON.c:1502-1593`).
    fn parse_array(&mut self, item: NodeId) -> bool {
        if self.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        self.depth += 1;
        let mut head: Option<NodeId> = None;
        let mut current: Option<NodeId> = None;
        let ok = 'run: {
            if self.byte(self.offset) != b'[' {
                break 'run false;
            }
            self.offset += 1;
            self.skip_whitespace();
            if self.can_access(0) && self.byte(self.offset) == b']' {
                break 'run true;
            }
            if !self.can_access(0) {
                self.offset -= 1;
                break 'run false;
            }
            self.offset -= 1;
            loop {
                let Some(new_item) = self.store.new_item() else {
                    break 'run false;
                };
                self.link(&mut head, &mut current, new_item);
                self.offset += 1;
                self.skip_whitespace();
                if !self.parse_value(new_item) {
                    break 'run false;
                }
                self.skip_whitespace();
                if !(self.can_access(0) && self.byte(self.offset) == b',') {
                    break;
                }
            }
            self.can_access(0) && self.byte(self.offset) == b']'
        };
        self.finish_container(item, ARRAY, ok, head, current)
    }

    /// `parse_object` (`cJSON.c:1667-1778`).
    fn parse_object(&mut self, item: NodeId) -> bool {
        if self.depth >= CJSON_NESTING_LIMIT {
            return false;
        }
        self.depth += 1;
        let mut head: Option<NodeId> = None;
        let mut current: Option<NodeId> = None;
        let ok = 'run: {
            if !self.can_access(0) || self.byte(self.offset) != b'{' {
                break 'run false;
            }
            self.offset += 1;
            self.skip_whitespace();
            if self.can_access(0) && self.byte(self.offset) == b'}' {
                break 'run true;
            }
            if !self.can_access(0) {
                self.offset -= 1;
                break 'run false;
            }
            self.offset -= 1;
            loop {
                let Some(new_item) = self.store.new_item() else {
                    break 'run false;
                };
                self.link(&mut head, &mut current, new_item);
                if !self.can_access(1) {
                    break 'run false;
                }
                self.offset += 1;
                self.skip_whitespace();
                if !self.parse_string(new_item) {
                    break 'run false;
                }
                self.skip_whitespace();
                // The parsed string is the key: swap valuestring and string.
                if let Some(n) = self.store.node_mut(new_item) {
                    n.string = std::mem::replace(&mut n.valuestring, Text::None);
                }
                if !self.can_access(0) || self.byte(self.offset) != b':' {
                    break 'run false;
                }
                self.offset += 1;
                self.skip_whitespace();
                if !self.parse_value(new_item) {
                    break 'run false;
                }
                self.skip_whitespace();
                if !(self.can_access(0) && self.byte(self.offset) == b',') {
                    break;
                }
            }
            self.can_access(0) && self.byte(self.offset) == b'}'
        };
        self.finish_container(item, OBJECT, ok, head, current)
    }
}

/// `parse_hex4` (`cJSON.c:666-699`): 4 hex digits, 0 on a non-hex digit.
// QUIRK(port): Q10
fn parse_hex4(input: &[u8]) -> u32 {
    let mut h: u32 = 0;
    for i in 0..4 {
        let Some(digit) = input.get(i).copied().and_then(is_hex_digit_value) else {
            return 0;
        };
        h += digit;
        if i < 3 {
            h <<= 4;
        }
    }
    h
}

/// `utf16_literal_to_utf8` (`cJSON.c:703-821`): the length of the consumed `\uXXXX` sequence(s), 0 on failure.
// QUIRK(port): Q10
fn utf16_literal_to_utf8(content: &[u8], first: usize, end: usize, out: &mut Vec<u8>) -> usize {
    if end - first < 6 {
        return 0;
    }
    let hex_at = |at: usize| parse_hex4(content.get(at..at + 4).unwrap_or(&[]));
    let first_code = hex_at(first + 2);
    if (0xdc00..=0xdfff).contains(&first_code) {
        return 0;
    }
    let (sequence_length, codepoint) = if (0xd800..=0xdbff).contains(&first_code) {
        let second = first + 6;
        if end - second < 6 {
            return 0;
        }
        if content.get(second) != Some(&b'\\') || content.get(second + 1) != Some(&b'u') {
            return 0;
        }
        let second_code = hex_at(second + 2);
        if !(0xdc00..=0xdfff).contains(&second_code) {
            return 0;
        }
        (
            12,
            0x10000 + (((first_code & 0x3ff) << 10) | (second_code & 0x3ff)),
        )
    } else {
        (6, first_code)
    };
    let (utf8_length, first_byte_mark): (usize, u32) = if codepoint < 0x80 {
        (1, 0)
    } else if codepoint < 0x800 {
        (2, 0xc0)
    } else if codepoint < 0x10000 {
        (3, 0xe0)
    } else if codepoint <= 0x10ffff {
        (4, 0xf0)
    } else {
        return 0;
    };
    let mut bytes = [0u8; 4];
    let mut cp = codepoint;
    for position in (1..utf8_length).rev() {
        bytes[position] = ((cp | 0x80) & 0xbf) as u8;
        cp >>= 6;
    }
    bytes[0] = if utf8_length > 1 {
        ((cp | first_byte_mark) & 0xff) as u8
    } else {
        (cp & 0x7f) as u8
    };
    out.extend_from_slice(&bytes[..utf8_length]);
    sequence_length
}

/// The accepted prefix of C `strtod` on the scanned copy (Q9) and its value, `None` when nothing converts
/// (RULEBOOK I8, D5). The copy holds only `0-9 + - e E .`, so no `inf`, `nan` or hex form can occur.
// QUIRK(port): Q9
fn strtod_prefix(text: &[u8]) -> Option<(f64, usize)> {
    let digits = |from: usize| {
        text[from..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count()
    };
    let mut i = 0;
    if matches!(text.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let integer = digits(i);
    i += integer;
    let mut fraction = 0;
    if text.get(i) == Some(&b'.') {
        fraction = digits(i + 1);
        if integer + fraction > 0 {
            i += 1 + fraction;
        }
    }
    if integer + fraction == 0 {
        return None;
    }
    if matches!(text.get(i), Some(b'e' | b'E')) {
        let mut j = i + 1;
        if matches!(text.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exponent = digits(j);
        if exponent > 0 {
            i = j + exponent;
        }
    }
    let value = std::str::from_utf8(&text[..i]).ok()?.parse::<f64>().ok()?;
    Some((value, i))
}

/// `cJSON_Minify` (`cJSON.c:2875-2970`) on a NUL-terminated buffer, in place; the end of the slice also ends the
/// string.
// QUIRK(port): Q15
// BUG(port): Q15
pub fn minify(json: &mut [u8]) {
    fn at(json: &[u8], i: usize) -> u8 {
        json.get(i).copied().unwrap_or(0)
    }
    fn put(json: &mut [u8], i: usize, value: u8) {
        if let Some(b) = json.get_mut(i) {
            *b = value;
        }
    }
    let mut i = 0;
    let mut into = 0;
    while at(json, i) != 0 {
        match at(json, i) {
            b' ' | b'\t' | b'\r' | b'\n' => i += 1,
            b'/' => {
                if at(json, i + 1) == b'/' {
                    i += 2;
                    while at(json, i) != 0 {
                        if at(json, i) == b'\n' {
                            i += 1;
                            break;
                        }
                        i += 1;
                    }
                } else if at(json, i + 1) == b'*' {
                    i += 2;
                    while at(json, i) != 0 {
                        if at(json, i) == b'*' && at(json, i + 1) == b'/' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                } else {
                    i += 1;
                }
            }
            b'"' => {
                put(json, into, b'"');
                i += 1;
                into += 1;
                while at(json, i) != 0 {
                    let c = at(json, i);
                    put(json, into, c);
                    if c == b'"' {
                        i += 1;
                        into += 1;
                        break;
                    } else if c == b'\\' && at(json, i + 1) == b'"' {
                        put(json, into + 1, b'"');
                        i += 1;
                        into += 1;
                    }
                    i += 1;
                    into += 1;
                }
            }
            c => {
                put(json, into, c);
                i += 1;
                into += 1;
            }
        }
    }
    put(json, into, 0);
}

// PORT STATUS: stage=2 confidence=high todos=0 deviations=D1,D3,D4,D5,D6,D7,D8,D9,D10,D11,D12,D13,D14,D17,D19,D21
