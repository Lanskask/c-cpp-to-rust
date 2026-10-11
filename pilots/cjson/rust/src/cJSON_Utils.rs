//! Idiomatic port (Stage 2) of `cJSON_Utils.c` @ 6d9f244: JSON Pointer, JSON Patch (RFC 6902), JSON Merge
//! Patch (RFC 7396) and object sorting over the [`Store`] of the `cJSON` Part.

use std::cmp::Ordering;

use crate::cJSON::{
    alloc_event, c_str, compare_double, flags, free_event, strdup, Node, NodeId, Printed, Store,
    Text,
};

/// A JSON Patch status of `cJSONUtils_ApplyPatches` (`cJSON_Utils.c:807-1062`, Q23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchError {
    /// The patch list is not an array (1).
    NotAnArray,
    /// A `test` operation found a different value (1).
    TestFailed,
    /// No string `path` (2).
    MissingPath,
    /// No valid `op` (3).
    BadOperation,
    /// `move`/`copy` without a string `from` (4).
    MissingFrom,
    /// `from` does not exist (5).
    FromNotFound,
    /// `copy`/`move`: the value could not be duplicated (6).
    DuplicateFailed,
    /// `add`/`replace` without `value` (7).
    MissingValue,
    /// `add`/`replace`: the value could not be duplicated (8).
    ValueDuplicateFailed,
    /// The parent of `path` does not exist (9).
    NoParent,
    /// Array insert past the end (10).
    InsertPastEnd,
    /// Bad array index (11).
    BadIndex,
    /// `remove`/`replace` of a path that does not exist (13).
    NothingToRemove,
}

impl PatchError {
    /// The exact C status code.
    pub fn code(self) -> i32 {
        match self {
            PatchError::NotAnArray | PatchError::TestFailed => 1,
            PatchError::MissingPath => 2,
            PatchError::BadOperation => 3,
            PatchError::MissingFrom => 4,
            PatchError::FromNotFound => 5,
            PatchError::DuplicateFailed => 6,
            PatchError::MissingValue => 7,
            PatchError::ValueDuplicateFailed => 8,
            PatchError::NoParent => 9,
            PatchError::InsertPastEnd => 10,
            PatchError::BadIndex => 11,
            PatchError::NothingToRemove => 13,
        }
    }
}

fn at(bytes: &[u8], index: usize) -> u8 {
    bytes.get(index).copied().unwrap_or(0)
}

/// A `cJSON_malloc` whose result C writes through without a NULL check (`cJSON_Utils.c:224/234`, ...).
fn malloc_unchecked(size: usize) -> Vec<u8> {
    if !alloc_event(size) {
        // DEVIATION(port): D2
        std::process::abort();
    }
    vec![0; size]
}

/// A `cJSON_malloc` of `size` bytes holding the C string `text`.
fn malloc_string(text: &[u8], size: usize) -> Vec<u8> {
    let mut buffer = malloc_unchecked(size.max(text.len() + 1));
    buffer[..text.len()].copy_from_slice(text);
    buffer
}

/// `compare_strings` (`cJSON_Utils.c:60-85`): NULL pointers are never equal; the sign of the difference.
fn compare_strings(string1: Option<&[u8]>, string2: Option<&[u8]>, case_sensitive: bool) -> i32 {
    let (Some(s1), Some(s2)) = (string1, string2) else {
        return 1;
    };
    let (s1, s2) = (c_str(s1), c_str(s2));
    // DEVIATION(port): D19
    let key = |c: &u8| {
        if case_sensitive {
            *c
        } else {
            c.to_ascii_lowercase()
        }
    };
    match s1.iter().map(key).cmp(s2.iter().map(key)) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

/// `compare_pointers` (`cJSON_Utils.c:112-146`): the next path element of a JSON pointer against a key.
// QUIRK(port): Q2
fn compare_pointers(name: Option<&[u8]>, pointer: &[u8], case_sensitive: bool) -> bool {
    let Some(name) = name else { return false };
    let name = c_str(name);
    let (mut n, mut p) = (0, 0);
    while at(name, n) != 0 && at(pointer, p) != 0 && at(pointer, p) != b'/' {
        if at(pointer, p) == b'~' {
            let escaped_tilde = at(pointer, p + 1) == b'0' && at(name, n) == b'~';
            let escaped_slash = at(pointer, p + 1) == b'1' && at(name, n) == b'/';
            if !escaped_tilde && !escaped_slash {
                return false;
            }
            p += 1;
        } else if case_sensitive {
            if at(name, n) != at(pointer, p) {
                return false;
            }
        } else if !at(name, n).eq_ignore_ascii_case(&at(pointer, p)) {
            return false;
        }
        n += 1;
        p += 1;
    }
    (at(pointer, p) != 0 && at(pointer, p) != b'/') == (at(name, n) != 0)
}

/// `encode_string_as_pointer` (`cJSON_Utils.c:170-195`): `/` becomes `~1`, `~` becomes `~0`.
fn encode_pointer(source: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(source.len() + 1);
    for &c in c_str(source) {
        match c {
            b'/' => out.extend_from_slice(b"~1"),
            b'~' => out.extend_from_slice(b"~0"),
            c => out.push(c),
        }
    }
    out
}

/// `cJSONUtils_FindPointerFromObjectTo` (`cJSON_Utils.c:197-276`): the pointer string as a `Printed`.
pub fn find_pointer_from_object_to(
    store: &Store,
    object: Option<NodeId>,
    target: Option<NodeId>,
) -> Option<Printed> {
    let (object_id, target_id) = (object?, target?);
    if object_id == target_id {
        // cJSONUtils_strdup("") through cJSON_malloc(1)
        return Some(Printed::from_vec(malloc_unchecked(1)));
    }
    let mut child_index: usize = 0;
    let mut current = store.child_of(object);
    while let Some(child) = current {
        if let Some(target_pointer) = find_pointer_from_object_to(store, Some(child), target) {
            if store.is_array(object) {
                // room for a 64 bit integer, '/' and NUL
                let size = target_pointer.bytes().len() + 20 + 2;
                let mut text = format!("/{child_index}").into_bytes();
                text.extend_from_slice(target_pointer.bytes());
                let buffer = malloc_string(&text, size);
                drop(target_pointer);
                return Some(Printed::from_vec(buffer));
            }
            if store.is_object(object) {
                let key = encode_pointer(store.key_bytes(Some(child)).unwrap_or(&[]));
                let size = target_pointer.bytes().len() + key.len() + 2;
                let mut text = vec![b'/'];
                text.extend_from_slice(&key);
                text.extend_from_slice(target_pointer.bytes());
                let buffer = malloc_string(&text, size);
                drop(target_pointer);
                return Some(Printed::from_vec(buffer));
            }
            // reached a leaf of the tree, found nothing
            drop(target_pointer);
            return None;
        }
        current = store.next_of(Some(child));
        child_index += 1;
    }
    None
}

/// `get_array_item` (`cJSON_Utils.c:278-290`): without the `int` index of `cJSON_GetArrayItem`.
fn get_array_item(store: &Store, array: Option<NodeId>, mut item: usize) -> Option<NodeId> {
    let mut child = store.child_of(array);
    while let Some(c) = child {
        if item == 0 {
            break;
        }
        item -= 1;
        child = store.next_of(Some(c));
    }
    child
}

/// `decode_array_index_from_pointer` (`cJSON_Utils.c:292-318`).
// QUIRK(port): Q19
fn decode_array_index_from_pointer(pointer: &[u8]) -> Option<usize> {
    if at(pointer, 0) == b'0' && at(pointer, 1) != 0 && at(pointer, 1) != b'/' {
        // leading zeroes are not permitted
        return None;
    }
    let mut parsed_index: usize = 0;
    let mut position = 0;
    while at(pointer, position).is_ascii_digit() {
        parsed_index = parsed_index
            .wrapping_mul(10)
            .wrapping_add(usize::from(at(pointer, position) - b'0'));
        position += 1;
    }
    if at(pointer, position) != 0 && at(pointer, position) != b'/' {
        return None;
    }
    Some(parsed_index)
}

/// `get_item_from_pointer` (`cJSON_Utils.c:320-370`).
fn get_item_from_pointer(
    store: &Store,
    object: Option<NodeId>,
    pointer: Option<&[u8]>,
    case_sensitive: bool,
) -> Option<NodeId> {
    let pointer = c_str(pointer?);
    let mut current = object;
    let mut p = 0;
    while at(pointer, p) == b'/' && current.is_some() {
        p += 1;
        let rest = pointer.get(p..).unwrap_or(&[]);
        if store.is_array(current) {
            let index = decode_array_index_from_pointer(rest)?;
            current = get_array_item(store, current, index);
        } else if store.is_object(current) {
            current = store.child_of(current);
            while let Some(c) = current {
                if compare_pointers(store.key_bytes(Some(c)), rest, case_sensitive) {
                    break;
                }
                current = store.next_of(Some(c));
            }
        } else {
            return None;
        }
        // skip to the next path token or end of string
        while at(pointer, p) != 0 && at(pointer, p) != b'/' {
            p += 1;
        }
    }
    current
}

/// `cJSONUtils_GetPointer`.
pub fn get_pointer(
    store: &Store,
    object: Option<NodeId>,
    pointer: Option<&[u8]>,
) -> Option<NodeId> {
    get_item_from_pointer(store, object, pointer, false)
}

/// `cJSONUtils_GetPointerCaseSensitive`.
pub fn get_pointer_case_sensitive(
    store: &Store,
    object: Option<NodeId>,
    pointer: Option<&[u8]>,
) -> Option<NodeId> {
    get_item_from_pointer(store, object, pointer, true)
}

/// `decode_pointer_inplace` (`cJSON_Utils.c:372-408`), bug included: plain characters after an escape are
/// never copied back and `~1` writes its `/` one byte too far.
// QUIRK(port): Q20
// BUG(port): Q20
fn decode_pointer_inplace(string: &mut [u8]) {
    fn put(buffer: &mut [u8], index: usize, value: u8) {
        if let Some(b) = buffer.get_mut(index) {
            *b = value;
        }
    }
    let (mut s, mut d) = (0, 0);
    while at(string, s) != 0 {
        if at(string, s) == b'~' {
            match at(string, s + 1) {
                b'0' => put(string, d, b'~'),
                b'1' => put(string, d + 1, b'/'),
                // invalid escape sequence
                _ => return,
            }
            s += 1;
        }
        d += 1;
        s += 1;
    }
    put(string, d, 0);
}

/// `detach_item_from_array` (`cJSON_Utils.c:410-447`): the non-broken `cJSON_DetachItemFromArray`.
fn detach_item_from_array(store: &mut Store, array: NodeId, which: usize) -> Option<NodeId> {
    let first = store.child_of(Some(array));
    let item = get_array_item(store, Some(array), which)?;
    let (next, prev) = (store.next_of(Some(item)), store.prev_of(Some(item)));
    if Some(item) != first {
        if let Some(p) = prev {
            store.set_next(p, next);
        }
    }
    if let Some(n) = next {
        store.set_prev(n, prev);
    }
    if Some(item) == first {
        store.set_child(array, next);
    } else if next.is_none() {
        if let Some(f) = first {
            store.set_prev(f, prev);
        }
    }
    store.set_prev(item, None);
    store.set_next(item, None);
    Some(item)
}

/// `detach_path` (`cJSON_Utils.c:449-505`): detach the item at `path`.
fn detach_path(
    store: &mut Store,
    object: Option<NodeId>,
    path: &[u8],
    case_sensitive: bool,
) -> Option<NodeId> {
    let path = c_str(path);
    // parent_pointer = cJSONUtils_strdup(path)
    let parent_pointer = strdup(Some(path))?;
    let size = parent_pointer.len();
    let detached = detach_path_in(store, object, path, case_sensitive);
    free_event(size);
    detached
}

fn detach_path_in(
    store: &mut Store,
    object: Option<NodeId>,
    path: &[u8],
    case_sensitive: bool,
) -> Option<NodeId> {
    // the last '/', then split the copy in parent and child
    let slash = path.iter().rposition(|&c| c == b'/')?;
    let parent_pointer = &path[..slash];
    let mut child_pointer = path[slash + 1..].to_vec();
    child_pointer.push(0);
    let parent = get_item_from_pointer(store, object, Some(parent_pointer), case_sensitive);
    decode_pointer_inplace(&mut child_pointer);
    if store.is_array(parent) {
        let index = decode_array_index_from_pointer(&child_pointer)?;
        detach_item_from_array(store, parent?, index)
    } else if store.is_object(parent) {
        let name = c_str(&child_pointer);
        if case_sensitive {
            store.detach_item_from_object_case_sensitive(parent, Some(name))
        } else {
            store.detach_item_from_object(parent, Some(name))
        }
    } else {
        // Couldn't find object to remove child from.
        None
    }
}

/// `sort_list` (`cJSON_Utils.c:507-602`): merge sort over the `next`/`prev` links, as C does, so the
/// first element's `prev` is left as the merge produced it (Q18).
// QUIRK(port): Q18
fn sort_list(store: &mut Store, list: Option<NodeId>, case_sensitive: bool) -> Option<NodeId> {
    let key_less = |store: &Store, a: Option<NodeId>, b: Option<NodeId>| -> bool {
        compare_strings(store.key_bytes(a), store.key_bytes(b), case_sensitive) < 0
    };
    if list.is_none() || store.next_of(list).is_none() {
        // One entry is sorted already.
        return list;
    }
    let mut current_item = list;
    while current_item.is_some() {
        let next = store.next_of(current_item);
        if next.is_none() || !key_less(store, current_item, next) {
            break;
        }
        current_item = next;
    }
    if current_item.is_none() || store.next_of(current_item).is_none() {
        // Leave sorted lists unmodified.
        return list;
    }
    // Walk two pointers to find the middle.
    let mut second = list;
    current_item = list;
    while current_item.is_some() {
        second = store.next_of(second);
        current_item = store.next_of(current_item);
        if current_item.is_some() {
            current_item = store.next_of(current_item);
        }
    }
    if let Some(s) = second {
        if let Some(p) = store.prev_of(Some(s)) {
            // Split the lists
            store.set_next(p, None);
            store.set_prev(s, None);
        }
    }
    let mut first = sort_list(store, list, case_sensitive);
    let mut second = sort_list(store, second, case_sensitive);
    let mut result: Option<NodeId> = None;
    let mut result_tail: Option<NodeId> = None;
    while first.is_some() && second.is_some() {
        let smaller = if key_less(store, first, second) {
            first
        } else {
            second
        };
        match (result, result_tail, smaller) {
            (Some(_), Some(tail), Some(sm)) => {
                store.set_next(tail, Some(sm));
                store.set_prev(sm, Some(tail));
            }
            _ => result = smaller,
        }
        result_tail = smaller;
        if first == smaller {
            first = store.next_of(first);
        } else {
            second = store.next_of(second);
        }
    }
    for rest in [first, second].into_iter().flatten() {
        let Some(tail) = result_tail.filter(|_| result.is_some()) else {
            return Some(rest);
        };
        store.set_next(tail, Some(rest));
        store.set_prev(rest, Some(tail));
    }
    result
}

/// `sort_object` (`cJSON_Utils.c:604-612`).
fn sort_object_impl(store: &mut Store, object: Option<NodeId>, case_sensitive: bool) {
    let Some(object) = object else { return };
    let sorted = sort_list(store, store.child_of(Some(object)), case_sensitive);
    store.set_child(object, sorted);
}

/// `cJSONUtils_SortObject`.
pub fn sort_object(store: &mut Store, object: Option<NodeId>) {
    sort_object_impl(store, object, false);
}

/// `cJSONUtils_SortObjectCaseSensitive`.
pub fn sort_object_case_sensitive(store: &mut Store, object: Option<NodeId>) {
    sort_object_impl(store, object, true);
}

/// `strcmp(a, b) != 0` with a NULL on either side counted as different (D20).
// DEVIATION(port): D20
fn different(a: Option<&[u8]>, b: Option<&[u8]>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => c_str(x) != c_str(y),
        _ => true,
    }
}

/// `compare_json` (`cJSON_Utils.c:614-700`): sorts the objects it compares (Q18).
// QUIRK(port): Q17
// QUIRK(port): Q18
fn compare_json(
    store: &mut Store,
    a: Option<NodeId>,
    b: Option<NodeId>,
    case_sensitive: bool,
) -> bool {
    let (Some(ta), Some(tb)) = (store.type_word(a), store.type_word(b)) else {
        return false;
    };
    if ta & 0xFF != tb & 0xFF {
        return false;
    }
    match ta & 0xFF {
        flags::NUMBER => {
            let (Some(na), Some(nb)) =
                (a.and_then(|i| store.node(i)), b.and_then(|i| store.node(i)))
            else {
                return false;
            };
            na.valueint == nb.valueint && compare_double(na.valuedouble, nb.valuedouble)
        }
        flags::STRING => !different(store.value_bytes(a), store.value_bytes(b)),
        flags::ARRAY => {
            let (mut ea, mut eb) = (store.child_of(a), store.child_of(b));
            while ea.is_some() && eb.is_some() {
                if !compare_json(store, ea, eb, case_sensitive) {
                    return false;
                }
                ea = store.next_of(ea);
                eb = store.next_of(eb);
            }
            // array size mismatch?
            ea.is_none() && eb.is_none()
        }
        flags::OBJECT => {
            sort_object_impl(store, a, case_sensitive);
            sort_object_impl(store, b, case_sensitive);
            let (mut ea, mut eb) = (store.child_of(a), store.child_of(b));
            while ea.is_some() && eb.is_some() {
                // compare object keys
                if compare_strings(store.key_bytes(ea), store.key_bytes(eb), case_sensitive) != 0 {
                    return false;
                }
                if !compare_json(store, ea, eb, case_sensitive) {
                    return false;
                }
                ea = store.next_of(ea);
                eb = store.next_of(eb);
            }
            // object length mismatch?
            ea.is_none() && eb.is_none()
        }
        // null, true or false
        _ => true,
    }
}

/// `insert_item_in_array` (`cJSON_Utils.c:702-742`): the non-broken `cJSON_InsertItemInArray`.
fn insert_item_in_array(
    store: &mut Store,
    array: NodeId,
    mut which: usize,
    new_item: NodeId,
) -> bool {
    let mut child = store.child_of(Some(array));
    while let Some(c) = child {
        if which == 0 {
            break;
        }
        child = store.next_of(Some(c));
        which -= 1;
    }
    if which > 0 {
        // item is after the end of the array
        return false;
    }
    let Some(child) = child else {
        store.add_item_to_array(Some(array), Some(new_item));
        return true;
    };
    // insert into the linked list
    let child_prev = store.prev_of(Some(child));
    store.set_next(new_item, Some(child));
    store.set_prev(new_item, child_prev);
    store.set_prev(child, Some(new_item));
    if store.child_of(Some(array)) == Some(child) {
        store.set_child(array, Some(new_item));
    } else if let Some(p) = child_prev {
        store.set_next(p, Some(new_item));
    }
    true
}

fn get_object_item(
    store: &Store,
    object: Option<NodeId>,
    name: &[u8],
    case_sensitive: bool,
) -> Option<NodeId> {
    if case_sensitive {
        store.get_object_item_case_sensitive(object, Some(name))
    } else {
        store.get_object_item(object, Some(name))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PatchOperation {
    Invalid,
    Add,
    Remove,
    Replace,
    Move,
    Copy,
    Test,
}

/// `decode_patch_operation` (`cJSON_Utils.c:752-794`).
fn decode_patch_operation(
    store: &Store,
    patch: Option<NodeId>,
    case_sensitive: bool,
) -> PatchOperation {
    let operation = get_object_item(store, patch, b"op", case_sensitive);
    if !store.is_string(operation) {
        return PatchOperation::Invalid;
    }
    match store.value_bytes(operation) {
        Some(b"add") => PatchOperation::Add,
        Some(b"remove") => PatchOperation::Remove,
        Some(b"replace") => PatchOperation::Replace,
        Some(b"move") => PatchOperation::Move,
        Some(b"copy") => PatchOperation::Copy,
        Some(b"test") => PatchOperation::Test,
        _ => PatchOperation::Invalid,
    }
}

/// `overwrite_item` (`cJSON_Utils.c:796-817`): frees key, value string and child list whatever the
/// reference flags say, then copies the whole replacement struct over `root`.
// DEVIATION(port): D12
fn overwrite_item(store: &mut Store, root: Option<NodeId>, replacement: Node) {
    let Some(root) = root else { return };
    let Some(node) = store.node(root) else { return };
    let (child, key_size, value_size) = (
        node.child,
        match &node.string {
            Text::Owned(v) => Some(v.len()),
            _ => None,
        },
        match &node.valuestring {
            Text::Owned(v) => Some(v.len()),
            _ => None,
        },
    );
    if let Some(size) = key_size {
        free_event(size);
    }
    if let Some(size) = value_size {
        free_event(size);
    }
    if child.is_some() {
        store.delete(child);
    }
    store.overwrite_fields(root, replacement);
}

/// `apply_patch` (`cJSON_Utils.c:819-1052`).
// QUIRK(port): Q23
fn apply_patch(
    store: &mut Store,
    object: Option<NodeId>,
    patch: Option<NodeId>,
    case_sensitive: bool,
) -> Result<(), PatchError> {
    let mut value: Option<NodeId> = None;
    // The C `parent_pointer` copy: its allocation size, freed in the cleanup.
    let mut parent_pointer_size: Option<usize> = None;
    let result = apply_patch_in(
        store,
        object,
        patch,
        case_sensitive,
        &mut value,
        &mut parent_pointer_size,
    );
    if value.is_some() {
        store.delete(value);
    }
    if let Some(size) = parent_pointer_size {
        free_event(size);
    }
    result
}

fn apply_patch_in(
    store: &mut Store,
    object: Option<NodeId>,
    patch: Option<NodeId>,
    case_sensitive: bool,
    value: &mut Option<NodeId>,
    parent_pointer_size: &mut Option<usize>,
) -> Result<(), PatchError> {
    let path = get_object_item(store, patch, b"path", case_sensitive);
    if !store.is_string(path) {
        // malformed patch.
        return Err(PatchError::MissingPath);
    }
    let opcode = decode_patch_operation(store, patch, case_sensitive);
    if opcode == PatchOperation::Invalid {
        return Err(PatchError::BadOperation);
    }
    // TODO(port): a String item without value (create_string_reference(NULL)) is dereferenced by C for
    // `path`, `from` and `op`; here its value is the empty string.
    let path_text: Vec<u8> = store.value_bytes(path).unwrap_or(&[]).to_vec();
    if opcode == PatchOperation::Test {
        // compare value: {...} with the given path
        let found = get_item_from_pointer(store, object, Some(&path_text), case_sensitive);
        let expected = get_object_item(store, patch, b"value", case_sensitive);
        return if compare_json(store, found, expected, case_sensitive) {
            Ok(())
        } else {
            Err(PatchError::TestFailed)
        };
    }

    // special case for replacing the root
    if path_text.is_empty() {
        if opcode == PatchOperation::Remove {
            let invalid = Node::zeroed();
            overwrite_item(store, object, invalid);
            return Ok(());
        }
        if opcode == PatchOperation::Replace || opcode == PatchOperation::Add {
            let source = get_object_item(store, patch, b"value", case_sensitive);
            if source.is_none() {
                // missing "value" for add/replace.
                return Err(PatchError::MissingValue);
            }
            let Some(duplicate) = store.duplicate(source, true) else {
                // out of memory for add/replace.
                return Err(PatchError::ValueDuplicateFailed);
            };
            if let Some(copy) = store.node_copy(duplicate) {
                overwrite_item(store, object, copy);
            }
            // delete the duplicated value (only its node: the strings and children moved into the root)
            store.free_node_only(duplicate);
            // the string "value" isn't needed
            if let Some(root) = object.and_then(|r| store.node_mut(r)) {
                if let Text::Owned(old) = std::mem::replace(&mut root.string, Text::None) {
                    free_event(old.len());
                }
            }
            return Ok(());
        }
    }

    if opcode == PatchOperation::Remove || opcode == PatchOperation::Replace {
        // Get rid of old.
        let Some(old_item) = detach_path(store, object, &path_text, case_sensitive) else {
            return Err(PatchError::NothingToRemove);
        };
        store.delete(Some(old_item));
        if opcode == PatchOperation::Remove {
            // For Remove, this job is done.
            return Ok(());
        }
    }

    // Copy/Move uses "from".
    if opcode == PatchOperation::Move || opcode == PatchOperation::Copy {
        let from = get_object_item(store, patch, b"from", case_sensitive);
        if !store.is_string(from) {
            // missing "from" for copy/move.
            return Err(PatchError::MissingFrom);
        }
        let from_text: Vec<u8> = store.value_bytes(from).unwrap_or(&[]).to_vec();
        if opcode == PatchOperation::Move {
            *value = detach_path(store, object, &from_text, case_sensitive);
        }
        if opcode == PatchOperation::Copy {
            *value = get_item_from_pointer(store, object, Some(&from_text), case_sensitive);
        }
        if value.is_none() {
            // missing "from" for copy/move.
            return Err(PatchError::FromNotFound);
        }
        if opcode == PatchOperation::Copy {
            *value = store.duplicate(*value, true);
        }
        if value.is_none() {
            // out of memory for copy/move.
            return Err(PatchError::DuplicateFailed);
        }
    } else {
        // Add/Replace uses "value".
        *value = get_object_item(store, patch, b"value", case_sensitive);
        if value.is_none() {
            // missing "value" for add/replace.
            return Err(PatchError::MissingValue);
        }
        *value = store.duplicate(*value, true);
        if value.is_none() {
            // out of memory for add/replace.
            return Err(PatchError::ValueDuplicateFailed);
        }
    }

    // Now, just add "value" to "path".
    // split pointer in parent and child
    let parent_copy = strdup(Some(&path_text));
    *parent_pointer_size = parent_copy.as_ref().map(Vec::len);
    let slash = parent_copy
        .as_ref()
        .and_then(|_| path_text.iter().rposition(|&c| c == b'/'));
    let mut child_pointer: Option<Vec<u8>> = slash.map(|s| {
        let mut child = path_text[s + 1..].to_vec();
        child.push(0);
        child
    });
    let parent_pointer: Option<&[u8]> = match (&parent_copy, slash) {
        (Some(_), Some(s)) => Some(&path_text[..s]),
        (Some(_), None) => Some(&path_text),
        (None, _) => None,
    };
    let parent = get_item_from_pointer(store, object, parent_pointer, case_sensitive);
    if let Some(child) = child_pointer.as_mut() {
        decode_pointer_inplace(child);
    }

    // add, remove, replace, move, copy, test.
    let (Some(parent_id), Some(child)) = (parent, child_pointer) else {
        // Couldn't find object to add to.
        return Err(PatchError::NoParent);
    };
    let child = c_str(&child);
    if store.is_array(Some(parent_id)) {
        if child == b"-" {
            store.add_item_to_array(Some(parent_id), *value);
            *value = None;
        } else {
            let Some(index) = decode_array_index_from_pointer(child) else {
                return Err(PatchError::BadIndex);
            };
            let Some(new_item) = *value else {
                return Err(PatchError::InsertPastEnd);
            };
            if !insert_item_in_array(store, parent_id, index, new_item) {
                return Err(PatchError::InsertPastEnd);
            }
            *value = None;
        }
    } else if store.is_object(Some(parent_id)) {
        if case_sensitive {
            store.delete_item_from_object_case_sensitive(Some(parent_id), Some(child));
        } else {
            store.delete_item_from_object(Some(parent_id), Some(child));
        }
        store.add_item_to_object(Some(parent_id), Some(child), *value);
        *value = None;
    } else {
        // parent is not an object
        return Err(PatchError::NoParent);
    }
    Ok(())
}

fn apply_patches_impl(
    store: &mut Store,
    object: Option<NodeId>,
    patches: Option<NodeId>,
    case_sensitive: bool,
) -> Result<(), PatchError> {
    if !store.is_array(patches) {
        // malformed patches.
        return Err(PatchError::NotAnArray);
    }
    let mut current_patch = store.child_of(patches);
    while let Some(patch) = current_patch {
        apply_patch(store, object, Some(patch), case_sensitive)?;
        current_patch = store.next_of(Some(patch));
    }
    Ok(())
}

/// `cJSONUtils_ApplyPatches`: the patches applied before a failing one stay applied (E4).
pub fn apply_patches(
    store: &mut Store,
    object: Option<NodeId>,
    patches: Option<NodeId>,
) -> Result<(), PatchError> {
    apply_patches_impl(store, object, patches, false)
}

/// `cJSONUtils_ApplyPatchesCaseSensitive`.
pub fn apply_patches_case_sensitive(
    store: &mut Store,
    object: Option<NodeId>,
    patches: Option<NodeId>,
) -> Result<(), PatchError> {
    apply_patches_impl(store, object, patches, true)
}

/// `compose_patch` (`cJSON_Utils.c:1096-1140`).
fn compose_patch(
    store: &mut Store,
    patches: Option<NodeId>,
    operation: Option<&[u8]>,
    path: Option<&[u8]>,
    suffix: Option<&[u8]>,
    value: Option<NodeId>,
) {
    let (Some(operation), Some(path)) = (operation, path) else {
        return;
    };
    if patches.is_none() {
        return;
    }
    let Some(patch) = store.create_object() else {
        return;
    };
    let op = store.create_string(Some(operation));
    store.add_item_to_object(Some(patch), Some(b"op"), op);
    match suffix {
        None => {
            let path_item = store.create_string(Some(path));
            store.add_item_to_object(Some(patch), Some(b"path"), path_item);
        }
        Some(suffix) => {
            let path = c_str(path);
            let encoded = encode_pointer(suffix);
            let size = path.len() + encoded.len() + 2;
            let mut buffer = malloc_unchecked(size);
            // sprintf("%s/", path), then the encoded suffix
            let mut text = path.to_vec();
            text.push(b'/');
            text.extend_from_slice(&encoded);
            buffer[..text.len()].copy_from_slice(&text);
            let path_item = store.create_string(Some(c_str(&buffer)));
            store.add_item_to_object(Some(patch), Some(b"path"), path_item);
            free_event(size);
        }
    }
    if let Some(value) = value {
        let copy = store.duplicate(Some(value), true);
        store.add_item_to_object(Some(patch), Some(b"value"), copy);
    }
    store.add_item_to_array(patches, Some(patch));
}

/// `cJSONUtils_AddPatchToArray`.
pub fn add_patch_to_array(
    store: &mut Store,
    array: Option<NodeId>,
    operation: Option<&[u8]>,
    path: Option<&[u8]>,
    value: Option<NodeId>,
) {
    compose_patch(store, array, operation, path, None, value);
}

fn key_copy(store: &Store, id: Option<NodeId>) -> Option<Vec<u8>> {
    store.key_bytes(id).map(<[u8]>::to_vec)
}

/// `create_patches` (`cJSON_Utils.c:1142-1271`).
// QUIRK(port): Q22
fn create_patches(
    store: &mut Store,
    patches: Option<NodeId>,
    path: &[u8],
    from: Option<NodeId>,
    to: Option<NodeId>,
    case_sensitive: bool,
) {
    let (Some(from_type), Some(to_type)) = (store.type_word(from), store.type_word(to)) else {
        return;
    };
    let path = c_str(path);
    if from_type & 0xFF != to_type & 0xFF {
        compose_patch(store, patches, Some(b"replace"), Some(path), None, to);
        return;
    }
    match from_type & 0xFF {
        flags::NUMBER => {
            let (Some(f), Some(t)) = (
                from.and_then(|i| store.node(i)),
                to.and_then(|i| store.node(i)),
            ) else {
                return;
            };
            if f.valueint != t.valueint || !compare_double(f.valuedouble, t.valuedouble) {
                compose_patch(store, patches, Some(b"replace"), Some(path), None, to);
            }
        }
        flags::STRING => {
            if different(store.value_bytes(from), store.value_bytes(to)) {
                compose_patch(store, patches, Some(b"replace"), Some(path), None, to);
            }
        }
        flags::ARRAY => {
            let mut index: usize = 0;
            let mut from_child = store.child_of(from);
            let mut to_child = store.child_of(to);
            // room for a 64 bit integer: log10(2^64) = 20
            let size = path.len() + 20 + 2;
            let _new_path = malloc_unchecked(size);
            // generate patches for all array elements that exist in both "from" and "to"
            while from_child.is_some() && to_child.is_some() {
                let mut element_path = path.to_vec();
                element_path.extend_from_slice(format!("/{index}").as_bytes());
                create_patches(
                    store,
                    patches,
                    &element_path,
                    from_child,
                    to_child,
                    case_sensitive,
                );
                from_child = store.next_of(from_child);
                to_child = store.next_of(to_child);
                index += 1;
            }
            // remove leftover elements from 'from' that are not in 'to'
            while from_child.is_some() {
                let suffix = index.to_string();
                compose_patch(
                    store,
                    patches,
                    Some(b"remove"),
                    Some(path),
                    Some(suffix.as_bytes()),
                    None,
                );
                from_child = store.next_of(from_child);
            }
            // add new elements in 'to' that were not in 'from'
            while to_child.is_some() {
                compose_patch(
                    store,
                    patches,
                    Some(b"add"),
                    Some(path),
                    Some(b"-"),
                    to_child,
                );
                to_child = store.next_of(to_child);
            }
            free_event(size);
        }
        flags::OBJECT => {
            sort_object_impl(store, from, case_sensitive);
            sort_object_impl(store, to, case_sensitive);
            let mut from_child = store.child_of(from);
            let mut to_child = store.child_of(to);
            // for all object values in the object with more of them
            while from_child.is_some() || to_child.is_some() {
                let diff = if from_child.is_none() {
                    1
                } else if to_child.is_none() {
                    -1
                } else {
                    compare_strings(
                        store.key_bytes(from_child),
                        store.key_bytes(to_child),
                        case_sensitive,
                    )
                };
                if diff == 0 {
                    // both object keys are the same
                    let name = encode_pointer(store.key_bytes(from_child).unwrap_or(&[]));
                    let size = path.len() + name.len() + 2;
                    let mut buffer = malloc_unchecked(size);
                    let mut text = path.to_vec();
                    text.push(b'/');
                    text.extend_from_slice(&name);
                    buffer[..text.len()].copy_from_slice(&text);
                    // create a patch for the element
                    create_patches(
                        store,
                        patches,
                        c_str(&buffer),
                        from_child,
                        to_child,
                        case_sensitive,
                    );
                    free_event(size);
                    from_child = store.next_of(from_child);
                    to_child = store.next_of(to_child);
                } else if diff < 0 {
                    // object element doesn't exist in 'to' --> remove it
                    let key = key_copy(store, from_child);
                    compose_patch(
                        store,
                        patches,
                        Some(b"remove"),
                        Some(path),
                        key.as_deref(),
                        None,
                    );
                    from_child = store.next_of(from_child);
                } else {
                    // object element doesn't exist in 'from' --> add it
                    let key = key_copy(store, to_child);
                    compose_patch(
                        store,
                        patches,
                        Some(b"add"),
                        Some(path),
                        key.as_deref(),
                        to_child,
                    );
                    to_child = store.next_of(to_child);
                }
            }
        }
        _ => {}
    }
}

fn generate_patches_impl(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
    case_sensitive: bool,
) -> Option<NodeId> {
    from?;
    to?;
    let patches = store.create_array();
    create_patches(store, patches, b"", from, to, case_sensitive);
    patches
}

/// `cJSONUtils_GeneratePatches`.
pub fn generate_patches(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
) -> Option<NodeId> {
    generate_patches_impl(store, from, to, false)
}

/// `cJSONUtils_GeneratePatchesCaseSensitive`.
pub fn generate_patches_case_sensitive(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
) -> Option<NodeId> {
    generate_patches_impl(store, from, to, true)
}

/// `merge_patch` (`cJSON_Utils.c:1304-1382`).
fn merge_patch_impl(
    store: &mut Store,
    target: Option<NodeId>,
    patch: Option<NodeId>,
    case_sensitive: bool,
) -> Option<NodeId> {
    if !store.is_object(patch) {
        // scalar value, array or NULL, just duplicate (before deleting the target: the patch may be inside it)
        let duplicate = store.duplicate(patch, true);
        store.delete(target);
        return duplicate;
    }
    let mut target = target;
    if !store.is_object(target) {
        store.delete(target);
        target = store.create_object();
    }
    let mut patch_child = store.child_of(patch);
    while let Some(child) = patch_child {
        let key = key_copy(store, Some(child));
        if store.is_null(Some(child)) {
            // NULL is the indicator to remove a value, see RFC7396
            if case_sensitive {
                store.delete_item_from_object_case_sensitive(target, key.as_deref());
            } else {
                store.delete_item_from_object(target, key.as_deref());
            }
        } else {
            let replace_me = if case_sensitive {
                store.detach_item_from_object_case_sensitive(target, key.as_deref())
            } else {
                store.detach_item_from_object(target, key.as_deref())
            };
            let Some(replacement) =
                merge_patch_impl(store, replace_me, Some(child), case_sensitive)
            else {
                store.delete(target);
                return None;
            };
            store.add_item_to_object(target, key.as_deref(), Some(replacement));
        }
        patch_child = store.next_of(Some(child));
    }
    target
}

/// `cJSONUtils_MergePatch`: consumes `target`, returns the new root.
pub fn merge_patch(
    store: &mut Store,
    target: Option<NodeId>,
    patch: Option<NodeId>,
) -> Option<NodeId> {
    merge_patch_impl(store, target, patch, false)
}

/// `cJSONUtils_MergePatchCaseSensitive`.
pub fn merge_patch_case_sensitive(
    store: &mut Store,
    target: Option<NodeId>,
    patch: Option<NodeId>,
) -> Option<NodeId> {
    merge_patch_impl(store, target, patch, true)
}

/// `generate_merge_patch` (`cJSON_Utils.c:1384-1478`).
// QUIRK(port): Q21
fn generate_merge_patch_impl(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
    case_sensitive: bool,
) -> Option<NodeId> {
    if to.is_none() {
        // patch to delete everything
        return store.create_null();
    }
    if !store.is_object(to) || !store.is_object(from) {
        return store.duplicate(to, true);
    }
    sort_object_impl(store, from, case_sensitive);
    sort_object_impl(store, to, case_sensitive);
    let mut from_child = store.child_of(from);
    let mut to_child = store.child_of(to);
    let patch = store.create_object()?;
    while from_child.is_some() || to_child.is_some() {
        let diff = match (from_child, to_child) {
            (Some(_), Some(_)) => {
                // strcmp whatever the case flag (Q21); NULL keys are different (D20)
                match (store.key_bytes(from_child), store.key_bytes(to_child)) {
                    (Some(a), Some(b)) => match a.cmp(b) {
                        Ordering::Less => -1,
                        Ordering::Equal => 0,
                        Ordering::Greater => 1,
                    },
                    // DEVIATION(port): D20
                    _ => 1,
                }
            }
            (Some(_), None) => -1,
            _ => 1,
        };
        if diff < 0 {
            // from has a value that to doesn't have -> remove
            let key = key_copy(store, from_child);
            let null = store.create_null();
            store.add_item_to_object(Some(patch), key.as_deref(), null);
            from_child = store.next_of(from_child);
        } else if diff > 0 {
            // to has a value that from doesn't have -> add to patch
            let key = key_copy(store, to_child);
            let copy = store.duplicate(to_child, true);
            store.add_item_to_object(Some(patch), key.as_deref(), copy);
            to_child = store.next_of(to_child);
        } else {
            // object key exists in both objects
            if !compare_json(store, from_child, to_child, case_sensitive) {
                // not identical --> generate a patch (the recursion is always case-insensitive, Q21)
                let key = key_copy(store, to_child);
                let nested = generate_merge_patch_impl(store, from_child, to_child, false);
                store.add_item_to_object(Some(patch), key.as_deref(), nested);
            }
            // next key in the object
            from_child = store.next_of(from_child);
            to_child = store.next_of(to_child);
        }
    }
    if store.child_of(Some(patch)).is_none() {
        // no patch generated
        store.delete(Some(patch));
        return None;
    }
    Some(patch)
}

/// `cJSONUtils_GenerateMergePatch`.
pub fn generate_merge_patch(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
) -> Option<NodeId> {
    generate_merge_patch_impl(store, from, to, false)
}

/// `cJSONUtils_GenerateMergePatchCaseSensitive`.
pub fn generate_merge_patch_case_sensitive(
    store: &mut Store,
    from: Option<NodeId>,
    to: Option<NodeId>,
) -> Option<NodeId> {
    generate_merge_patch_impl(store, from, to, true)
}

// PORT STATUS: stage=2 confidence=medium todos=1 deviations=D2,D12,D19,D20
