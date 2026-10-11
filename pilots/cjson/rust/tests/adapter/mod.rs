//! Judge adapter (judge.md section 5): the only test-side file that calls the `cjson` crate.
//!
//! STAGE 2 VERSION. It implements the interface below over the RULEBOOK section 6 API of the crate (a `Store`
//! behind a `Mutex`, `NodeId` handles, the `AllocHooks` seam) in safe Rust, and exposes the same plain,
//! C-shaped interface to `tests/upstream.rs` and `examples/dump.rs` as the Stage 1 adapter did (which called
//! the C-ABI functions in `unsafe` blocks). Neither the driver nor the tests changed.
//!
//! # The interface contract (RULEBOOK section 6.0; both adapters implement all of it)
//!
//! 1. Identity. [`Item`] is an opaque, `Copy + Eq + Hash` handle for "a `cJSON *`" (possibly NULL: [`NULL`]).
//!    Equality means "the same node" (the C pointer comparisons of the upstream tests); a handle stays valid
//!    and equal to itself while the node lives, across detach, add, replace and a root `remove` patch.
//!    Ownership is manual, as in C: `create_*`, `parse*`, `duplicate`, `detach_*` return a root the caller
//!    must `delete`; `add_*`, `insert_*`, `replace_*` take ownership of the item on success (and leave it with
//!    the caller on failure); `get_*`, `child`, `next`, `prev` return borrowed handles. Cycles are
//!    representable (`add_item_to_array(b, o)` with `o` an ancestor of `b`, RULEBOOK D11).
//! 2. Fields of every node of every kind. [`type_flags`] is the C type word: the kind bit (`INVALID` = 0, a
//!    root after a `remove` patch at path `""`), plus `IS_REFERENCE` 256 and `STRING_IS_CONST` 512.
//!    [`value_int`] / [`value_double`] are the stored fields (a parsed `true` has `valueint` 1, Q5), writable
//!    with [`set_value_int`] / [`set_value_double`] (no coupling between them, Q3). [`value_string`] (also
//!    for `Raw`) and [`key`] are `Option<Vec<u8>>`.
//! 3. Links. [`child`], [`next`], [`prev`] with the C invariant: the first child's `prev` is the last child,
//!    the last child's `next` is NULL, a node in no list has `prev` NULL (cJSON.c:2027-2058, 2256-2292).
//! 4. Parse. [`Parsed`]: `end` is the parse end on success and the error offset on failure (cJSON.c:1209-1212);
//!    `err` is the error position after the call with the reset rules of RULEBOOK Q26: [`ErrPos::Unset`] (NULL),
//!    [`ErrPos::At`] (offset into this call's input), [`ErrPos::Outside`] (set, but not inside this call's
//!    input: the stale value that `parse(None)` / `parse_with_opts(None, ..)` keep from the previous parse).
//! 5. Status codes. `apply_patches*` return the C status (0-11 and 13, 1 for a non-array patch list, Q23).
//! 6. Strings. Byte strings are `&[u8]` WITHOUT the terminating NUL (the adapter adds it); the library reads
//!    them up to the first NUL. `None` is the C NULL pointer. A string the library keeps by pointer
//!    (`create_string_reference`, `add_item_to_object_cs`) is `&'static CStr`. Returned strings are
//!    `Vec<u8>`/[`Printed`] holding the bytes up to the first NUL. [`value_string_addr`] is the address of the
//!    stored value buffer (a reference keeps the caller's buffer; a shorter `set_valuestring` keeps the
//!    buffer, a longer one gets a new one, cJSON.c:455-473).
//! 7. Writable views (RULEBOOK D21). [`with_string_value_mut`] and [`Printed::bytes_mut`] give the bytes before
//!    the NUL of the buffer that `cJSON_GetStringValue` / `cJSON_SetValuestring` / the print functions return;
//!    a write is seen by every later read, a NUL written inside truncates the value there.
//! 8. Allocation hooks (RULEBOOK D1, revised). [`hooks_on`] / [`hooks_off`] / [`hooks_failing_malloc`] /
//!    [`hook_stats`]: one allocation event per C allocation site with the C size (node 64 bytes, strings
//!    `strlen + 1`, print buffers as Q6), one free event per C free of a live allocation (a C `free(NULL)` is
//!    no event), growth of a print buffer under hooks = allocation of the new size, then free of the old.
//!    `allocs` counts every allocation event (also the failing one), `bytes` the sizes of successful ones,
//!    `frees` the free events. Dropping a [`Printed`] or the result of [`find_pointer_from_object_to`] is a
//!    free event, as the driver's `cJSON_free`.
//!
//! Capability (a constant, so that a Stage 2 adapter can say "not available" without touching the tests):
//! [`C_LAYOUT`]: the `raw_*` functions at the end of this file work (hand-corrupted lists, writes into the
//! node's own key, a node with two kind bits; RULEBOOK D7, D14). Stage 2: `false`, the `raw_*` functions
//! panic. Steps that need them print `SKIPPED` and are left out. There is no hooks capability: since the D1 revision (2026-10-10) every adapter
//! has the hooks, and `run.py --stage 2` always compares the allocator section.
//!
//! Locking. The library has process-global state (hooks, error position, RULEBOOK D1/D9). Tests run on
//! threads, so every test holds `upstream.rs::lock()`; the adapter itself is not synchronised.
#![allow(dead_code)]

use std::ffi::CStr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use cjson::cJSON as lib;
use cjson::cJSON::{AllocHooks, NodeId, ParseError, Store};
use cjson::cJSON_Utils as utils;

/// Stage 2: the `raw_*` functions do not work (RULEBOOK D7, D14).
pub const C_LAYOUT: bool = false;

// ---- type flags (cJSON.h) ------------------------------------------------------------------------------------

pub const INVALID: i32 = 0;
pub const FALSE: i32 = 1 << 0;
pub const TRUE: i32 = 1 << 1;
pub const NULL_T: i32 = 1 << 2;
pub const NUMBER: i32 = 1 << 3;
pub const STRING: i32 = 1 << 4;
pub const ARRAY: i32 = 1 << 5;
pub const OBJECT: i32 = 1 << 6;
pub const RAW: i32 = 1 << 7;
pub const IS_REFERENCE: i32 = 256;
pub const STRING_IS_CONST: i32 = 512;

pub const NESTING_LIMIT: usize = lib::CJSON_NESTING_LIMIT;
pub const CIRCULAR_LIMIT: usize = lib::CJSON_CIRCULAR_LIMIT;

// ---- handles and argument plumbing ---------------------------------------------------------------------------

/// The one store behind every handle (the tests share it, serialised by `upstream.rs::lock()`).
static STORE: Mutex<Store> = Mutex::new(Store::new());

/// Lock the store; a panicking test must not poison the following ones.
fn store() -> MutexGuard<'static, Store> {
    STORE.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Item(Option<NodeId>);

/// The C NULL.
pub const NULL: Item = Item(None);

impl Item {
    pub fn is_null(self) -> bool {
        self.0.is_none()
    }
}

/// The node of a non-NULL handle (the Stage 1 adapter asserted the same).
fn id(item: Item) -> NodeId {
    match item.0 {
        Some(node) => node,
        None => panic!("adapter: NULL item"),
    }
}

// ---- parse ---------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrPos {
    /// `cJSON_GetErrorPtr()` is NULL.
    Unset,
    /// Offset of the error pointer into the input.
    At(usize),
    /// The error pointer is set but not inside this call's input: the input was NULL and the error is the
    /// stale one of the previous parse (RULEBOOK Q26).
    Outside,
}

pub struct Parsed {
    pub item: Item,
    /// Offset of `*return_parse_end` (only when `want_end` was set and the library wrote it): the end of the
    /// value on success, the error offset on failure (cJSON.c:1209-1212).
    pub end: Option<usize>,
    pub err: ErrPos,
}

enum ParseCall {
    Plain,
    WithLength(usize),
    Opts {
        require_nul: bool,
        want_end: bool,
    },
    LengthOpts {
        len: usize,
        require_nul: bool,
        want_end: bool,
    },
}

fn do_parse(text: Option<&[u8]>, call: ParseCall) -> Parsed {
    do_parse_nul(text, call, true)
}

/// `append_nul = false`: the C caller's buffer is exactly the slice, with no terminating NUL after it (the parser's
/// length is then `strlen`-bounded by the slice end: RULEBOOK 6.0 item 6).
fn do_parse_nul(text: Option<&[u8]>, call: ParseCall, append_nul: bool) -> Parsed {
    // The C caller's buffer: the text and its terminating NUL.
    let buffer: Option<Vec<u8>> = text.map(|t| {
        let mut v = Vec::with_capacity(t.len() + 1);
        v.extend_from_slice(t);
        if append_nul {
            v.push(0);
        }
        v
    });
    let (result, want_end) = {
        let mut s = store();
        let b = buffer.as_deref();
        match call {
            ParseCall::Plain => (s.parse(b), false),
            ParseCall::WithLength(len) => (s.parse_with_length(b, len), false),
            ParseCall::Opts {
                require_nul,
                want_end,
            } => (s.parse_with_opts(b, require_nul), want_end),
            ParseCall::LengthOpts {
                len,
                require_nul,
                want_end,
            } => (s.parse_with_length_opts(b, len, require_nul), want_end),
        }
    };
    let (item, end) = match result {
        Ok(p) => (Item(Some(p.root)), Some(p.end)),
        Err(ParseError::At(offset)) => (NULL, Some(offset)),
        Err(ParseError::NoInput) => (NULL, None),
    };
    let err = match (text.is_some(), lib::get_error_offset()) {
        (true, Some(offset)) => ErrPos::At(offset),
        (false, Some(_)) => ErrPos::Outside,
        (_, None) => ErrPos::Unset,
    };
    Parsed {
        item,
        end: if want_end { end } else { None },
        err,
    }
}

/// `cJSON_Parse`.
pub fn parse(text: Option<&[u8]>) -> Parsed {
    do_parse(text, ParseCall::Plain)
}

/// `cJSON_Parse` of a buffer without a terminating NUL after the slice.
pub fn parse_unterminated(text: &[u8]) -> Parsed {
    do_parse_nul(Some(text), ParseCall::Plain, false)
}

/// `cJSON_ParseWithOpts` of a buffer without a terminating NUL after the slice.
pub fn parse_with_opts_unterminated(text: &[u8], require_nul: bool, want_end: bool) -> Parsed {
    do_parse_nul(
        Some(text),
        ParseCall::Opts {
            require_nul,
            want_end,
        },
        false,
    )
}

/// `cJSON_ParseWithLength`: `len` is passed as is (it may include the terminator).
pub fn parse_with_length(text: Option<&[u8]>, len: usize) -> Parsed {
    do_parse(text, ParseCall::WithLength(len))
}

/// `cJSON_ParseWithOpts`; `want_end` = pass a non-NULL `return_parse_end`.
pub fn parse_with_opts(text: Option<&[u8]>, require_nul: bool, want_end: bool) -> Parsed {
    do_parse(
        text,
        ParseCall::Opts {
            require_nul,
            want_end,
        },
    )
}

/// `cJSON_ParseWithLengthOpts`.
pub fn parse_with_length_opts(
    text: Option<&[u8]>,
    len: usize,
    require_nul: bool,
    want_end: bool,
) -> Parsed {
    do_parse(
        text,
        ParseCall::LengthOpts {
            len,
            require_nul,
            want_end,
        },
    )
}

// ---- print ---------------------------------------------------------------------------------------------------

/// A string returned by a print function; dropping it is the `cJSON_free` event of the allocator hooks.
pub struct Printed(lib::Printed);

impl Printed {
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    /// RULEBOOK D21: the returned buffer as writable bytes (those before the NUL); a NUL written inside
    /// shortens what [`Printed::bytes`] returns afterwards.
    pub fn bytes_mut(&mut self) -> &mut [u8] {
        self.0.bytes_mut()
    }
}

/// `cJSON_Print` (formatted).
pub fn print(item: Item) -> Option<Printed> {
    let printed = store().print(item.0);
    printed.map(Printed)
}

/// `cJSON_PrintUnformatted`.
pub fn print_unformatted(item: Item) -> Option<Printed> {
    let printed = store().print_unformatted(item.0);
    printed.map(Printed)
}

/// `cJSON_PrintBuffered`.
pub fn print_buffered(item: Item, prebuffer: i32, fmt: bool) -> Option<Printed> {
    let prebuffer = usize::try_from(prebuffer).ok()?;
    let printed = store().print_buffered(item.0, prebuffer, fmt);
    printed.map(Printed)
}

/// `cJSON_PrintPreallocated(item, buf, length, fmt)`; `length` must not exceed `buf.len()` when `buf` is
/// `Some`. The C bytes written are visible in `buf` afterwards (also after a `false` result, Q6).
pub fn print_preallocated(item: Item, buf: Option<&mut [u8]>, length: i32, fmt: bool) -> bool {
    let Some(s) = buf else { return false };
    let Ok(length) = usize::try_from(length) else {
        return false;
    };
    assert!(length <= s.len(), "adapter: length exceeds the buffer");
    store().print_preallocated(item.0, &mut s[..length], fmt)
}

pub fn version() -> Vec<u8> {
    lib::version().to_vec()
}

// ---- node access ---------------------------------------------------------------------------------------------

pub fn delete(item: Item) {
    store().delete(item.0);
}

/// The full `type` word (kind bits plus `IS_REFERENCE` / `STRING_IS_CONST`).
pub fn type_flags(item: Item) -> i32 {
    store().node_type(id(item)).unwrap_or(0) as i32
}

pub fn value_int(item: Item) -> i32 {
    store().value_int(id(item)).unwrap_or(0)
}

pub fn value_double(item: Item) -> f64 {
    store().value_double(id(item)).unwrap_or(0.0)
}

/// The `string` member (the key), `None` when NULL.
pub fn key(item: Item) -> Option<Vec<u8>> {
    store().key(id(item)).map(<[u8]>::to_vec)
}

/// The `valuestring` member, `None` when NULL.
pub fn value_string(item: Item) -> Option<Vec<u8>> {
    store().value_string(id(item)).map(<[u8]>::to_vec)
}

/// Address of the `valuestring` buffer (0 for NULL), for the "no copy" / "did not reallocate" assertions.
pub fn value_string_addr(item: Item) -> usize {
    store().value_string_addr(id(item))
}

/// Write the `valueint` field only (C: `item->valueint = v`; `valuedouble` is not touched, Q3).
pub fn set_value_int(item: Item, v: i32) {
    store().set_value_int(id(item), v);
}

/// Write the `valuedouble` field only (C: `item->valuedouble = v`; `valueint` is not touched, Q3).
pub fn set_value_double(item: Item, v: f64) {
    store().set_value_double(id(item), v);
}

/// RULEBOOK D21: `char *s = cJSON_GetStringValue(item)` used as a writable buffer. `f` gets the bytes before
/// the NUL; a NUL written inside truncates the value for every later read. Returns `false` without calling
/// `f` when C returns NULL (not a string, NULL item, no value) or the string is a reference (`IsReference`:
/// the buffer is the caller's, RULEBOOK D13).
pub fn with_string_value_mut(item: Item, f: impl FnOnce(&mut [u8])) -> bool {
    let mut s = store();
    match s.string_value_mut(item.0) {
        Some(bytes) => {
            f(bytes);
            true
        }
        None => false,
    }
}

pub fn child(item: Item) -> Item {
    Item(store().child(id(item)))
}

pub fn next(item: Item) -> Item {
    Item(store().next(id(item)))
}

/// `prev` of the first sibling is the last sibling (cJSON invariant); NULL for a detached node.
pub fn prev(item: Item) -> Item {
    Item(store().prev(id(item)))
}

/// `cJSON_ArrayForEach`: the children of `item` in order (empty for NULL).
pub fn children(item: Item) -> Vec<Item> {
    let ids = store().children(item.0);
    ids.into_iter().map(|c| Item(Some(c))).collect()
}

pub fn is_invalid(item: Item) -> bool {
    store().is_invalid(item.0)
}
pub fn is_false(item: Item) -> bool {
    store().is_false(item.0)
}
pub fn is_true(item: Item) -> bool {
    store().is_true(item.0)
}
pub fn is_bool(item: Item) -> bool {
    store().is_bool(item.0)
}
pub fn is_null(item: Item) -> bool {
    store().is_null(item.0)
}
pub fn is_number(item: Item) -> bool {
    store().is_number(item.0)
}
pub fn is_string(item: Item) -> bool {
    store().is_string(item.0)
}
pub fn is_array(item: Item) -> bool {
    store().is_array(item.0)
}
pub fn is_object(item: Item) -> bool {
    store().is_object(item.0)
}
pub fn is_raw(item: Item) -> bool {
    store().is_raw(item.0)
}

pub fn array_size(item: Item) -> i32 {
    store().get_array_size(item.0) as i32
}

pub fn array_item(item: Item, index: i32) -> Item {
    let Ok(index) = usize::try_from(index) else {
        return NULL;
    };
    Item(store().get_array_item(item.0, index))
}

pub fn object_item(item: Item, name: Option<&[u8]>) -> Item {
    Item(store().get_object_item(item.0, name))
}

pub fn object_item_cs(item: Item, name: Option<&[u8]>) -> Item {
    Item(store().get_object_item_case_sensitive(item.0, name))
}

pub fn has_object_item(item: Item, name: Option<&[u8]>) -> bool {
    store().has_object_item(item.0, name)
}

/// `cJSON_GetStringValue`.
pub fn get_string_value(item: Item) -> Option<Vec<u8>> {
    store().get_string_value(item.0).map(<[u8]>::to_vec)
}

/// `cJSON_GetNumberValue` (NaN for a non-number or NULL, Q24).
pub fn get_number_value(item: Item) -> f64 {
    store().get_number_value(item.0)
}

// ---- constructors --------------------------------------------------------------------------------------------

pub fn create_null() -> Item {
    Item(store().create_null())
}
pub fn create_true() -> Item {
    Item(store().create_true())
}
pub fn create_false() -> Item {
    Item(store().create_false())
}
pub fn create_bool(v: bool) -> Item {
    Item(store().create_bool(v))
}
pub fn create_number(v: f64) -> Item {
    Item(store().create_number(v))
}
pub fn create_string(s: Option<&[u8]>) -> Item {
    Item(store().create_string(s))
}
pub fn create_raw(s: Option<&[u8]>) -> Item {
    Item(store().create_raw(s))
}
pub fn create_array() -> Item {
    Item(store().create_array())
}
pub fn create_object() -> Item {
    Item(store().create_object())
}

/// `cJSON_CreateStringReference`: the item points at `s` itself (no copy).
pub fn create_string_reference(s: Option<&'static CStr>) -> Item {
    Item(store().create_string_reference(s.map(CStr::to_bytes)))
}

/// `cJSON_CreateObjectReference`: the new item's `child` is `child` (no copy, RULEBOOK D12).
pub fn create_object_reference(child: Item) -> Item {
    Item(store().create_object_reference(child.0))
}
pub fn create_array_reference(child: Item) -> Item {
    Item(store().create_array_reference(child.0))
}

/// `cJSON_CreateIntArray(numbers, count)`; `numbers = None` is NULL (then `count` is only passed on).
pub fn create_int_array(numbers: Option<&[i32]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    match (numbers, usize::try_from(count)) {
        (Some(n), Ok(count)) => Item(store().create_int_array(&n[..count])),
        _ => NULL,
    }
}
pub fn create_float_array(numbers: Option<&[f32]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    match (numbers, usize::try_from(count)) {
        (Some(n), Ok(count)) => Item(store().create_float_array(&n[..count])),
        _ => NULL,
    }
}
pub fn create_double_array(numbers: Option<&[f64]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    match (numbers, usize::try_from(count)) {
        (Some(n), Ok(count)) => Item(store().create_double_array(&n[..count])),
        _ => NULL,
    }
}
pub fn create_string_array(strings: Option<&[&[u8]]>, count: i32) -> Item {
    if let Some(s) = strings {
        assert!(count < 0 || count as usize <= s.len());
    }
    match (strings, usize::try_from(count)) {
        (Some(s), Ok(count)) => Item(store().create_string_array(&s[..count])),
        _ => NULL,
    }
}

// ---- add / detach / delete / insert / replace ----------------------------------------------------------------

pub fn add_item_to_array(array: Item, item: Item) -> bool {
    store().add_item_to_array(array.0, item.0)
}

pub fn add_item_to_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    store().add_item_to_object(object.0, name, item.0)
}

/// `cJSON_AddItemToObjectCS`: the key is stored by pointer, so it must be `'static`.
pub fn add_item_to_object_cs(object: Item, name: Option<&'static CStr>, item: Item) -> bool {
    store().add_item_to_object_cs(object.0, name.map(CStr::to_bytes), item.0)
}

pub fn add_item_reference_to_array(array: Item, item: Item) -> bool {
    store().add_item_reference_to_array(array.0, item.0)
}

pub fn add_item_reference_to_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    store().add_item_reference_to_object(object.0, name, item.0)
}

pub fn detach_item_via_pointer(parent: Item, item: Item) -> Item {
    Item(store().detach_item_via_pointer(parent.0, item.0))
}
pub fn detach_item_from_array(array: Item, which: i32) -> Item {
    let Ok(which) = usize::try_from(which) else {
        return NULL;
    };
    Item(store().detach_item_from_array(array.0, which))
}
pub fn delete_item_from_array(array: Item, which: i32) {
    if let Ok(which) = usize::try_from(which) {
        store().delete_item_from_array(array.0, which);
    }
}
pub fn detach_item_from_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().detach_item_from_object(object.0, name))
}
pub fn detach_item_from_object_cs(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().detach_item_from_object_case_sensitive(object.0, name))
}
pub fn delete_item_from_object(object: Item, name: Option<&[u8]>) {
    store().delete_item_from_object(object.0, name);
}
pub fn delete_item_from_object_cs(object: Item, name: Option<&[u8]>) {
    store().delete_item_from_object_case_sensitive(object.0, name);
}

pub fn insert_item_in_array(array: Item, which: i32, item: Item) -> bool {
    let Ok(which) = usize::try_from(which) else {
        return false;
    };
    store().insert_item_in_array(array.0, which, item.0)
}
pub fn replace_item_via_pointer(parent: Item, item: Item, replacement: Item) -> bool {
    store().replace_item_via_pointer(parent.0, item.0, replacement.0)
}
pub fn replace_item_in_array(array: Item, which: i32, item: Item) -> bool {
    let Ok(which) = usize::try_from(which) else {
        return false;
    };
    store().replace_item_in_array(array.0, which, item.0)
}
pub fn replace_item_in_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    store().replace_item_in_object(object.0, name, item.0)
}
pub fn replace_item_in_object_cs(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    store().replace_item_in_object_case_sensitive(object.0, name, item.0)
}

pub fn add_null_to_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().add_null_to_object(object.0, name))
}
pub fn add_true_to_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().add_true_to_object(object.0, name))
}
pub fn add_false_to_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().add_false_to_object(object.0, name))
}
pub fn add_bool_to_object(object: Item, name: Option<&[u8]>, v: bool) -> Item {
    Item(store().add_bool_to_object(object.0, name, v))
}
pub fn add_number_to_object(object: Item, name: Option<&[u8]>, v: f64) -> Item {
    Item(store().add_number_to_object(object.0, name, v))
}
pub fn add_string_to_object(object: Item, name: Option<&[u8]>, s: Option<&[u8]>) -> Item {
    Item(store().add_string_to_object(object.0, name, s))
}
pub fn add_raw_to_object(object: Item, name: Option<&[u8]>, raw: Option<&[u8]>) -> Item {
    Item(store().add_raw_to_object(object.0, name, raw))
}
pub fn add_object_to_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().add_object_to_object(object.0, name))
}
pub fn add_array_to_object(object: Item, name: Option<&[u8]>) -> Item {
    Item(store().add_array_to_object(object.0, name))
}

// ---- whole-tree operations -----------------------------------------------------------------------------------

pub fn duplicate(item: Item, recurse: bool) -> Item {
    Item(store().duplicate(item.0, recurse))
}

pub fn compare(a: Item, bb: Item, case_sensitive: bool) -> bool {
    store().compare(a.0, bb.0, case_sensitive)
}

/// `cJSON_Minify` on a private NUL-terminated copy of `json`; returns the bytes up to the first NUL
/// (`None` for the NULL call).
pub fn minify(json: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut buf = json?.to_vec();
    buf.push(0);
    lib::minify(&mut buf);
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    buf.truncate(n);
    Some(buf)
}

/// `cJSON_SetNumberValue(item, v)` (the macro: NULL item returns `v`, else `cJSON_SetNumberHelper`).
pub fn set_number_value(item: Item, v: f64) -> f64 {
    store().set_number_value(item.0, v)
}

/// `cJSON_SetNumberHelper` directly (also for NULL, Q24).
pub fn set_number_helper(item: Item, v: f64) -> f64 {
    store().set_number_helper(item.0, v)
}

/// `cJSON_SetValuestring`: the new valuestring, `None` when the call fails.
pub fn set_valuestring(item: Item, s: Option<&[u8]>) -> Option<Vec<u8>> {
    store().set_valuestring(item.0, s).map(<[u8]>::to_vec)
}

/// The `cJSON_SetBoolValue` macro of `cJSON.h`: returns the new type word (`INVALID` when not a bool).
pub fn set_bool_value(item: Item, v: bool) -> i32 {
    store().set_bool_value(item.0, v) as i32
}

// ---- allocator hooks (RULEBOOK D1) ---------------------------------------------------------------------------

static N_ALLOC: AtomicI64 = AtomicI64::new(0);
static N_FREE: AtomicI64 = AtomicI64::new(0);
static N_BYTES: AtomicI64 = AtomicI64::new(0);
static FAIL_AT: AtomicI64 = AtomicI64::new(0);
/// Allocation trace (`" a<size>"` per allocation, `" f"` per free), capped at 4000 bytes like `dump.c`.
static TRACE_ON: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static TRACE: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

fn trace_add(s: &str) {
    if TRACE_ON.load(Ordering::SeqCst) {
        let mut t = TRACE.lock().unwrap();
        if t.len() + s.len() <= 4000 {
            t.push_str(s);
        }
    }
}

/// Start recording the allocation trace (cleared).
pub fn trace_start() {
    TRACE.lock().unwrap().clear();
    TRACE_ON.store(true, Ordering::SeqCst);
}

/// Stop recording and return the trace.
pub fn trace_stop() -> String {
    TRACE_ON.store(false, Ordering::SeqCst);
    std::mem::take(&mut *TRACE.lock().unwrap())
}

/// Counting hooks: allocation number `FAIL_AT` fails (upstream `h_malloc` / `h_free`).
struct CountingHooks;

impl AllocHooks for CountingHooks {
    fn allocate(&self, size: usize) -> bool {
        let a = N_ALLOC.fetch_add(1, Ordering::SeqCst) + 1;
        trace_add(&format!(" a{size}"));
        let f = FAIL_AT.load(Ordering::SeqCst);
        if f != 0 && a == f {
            return false;
        }
        N_BYTES.fetch_add(size as i64, Ordering::SeqCst);
        true
    }

    fn deallocate(&self, _size: usize) {
        trace_add(" f");
        N_FREE.fetch_add(1, Ordering::SeqCst);
    }
}

/// Upstream `failing_hooks`: every allocation fails.
struct FailingHooks;

impl AllocHooks for FailingHooks {
    fn allocate(&self, _size: usize) -> bool {
        false
    }

    fn deallocate(&self, _size: usize) {}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HookStats {
    pub allocs: i64,
    pub frees: i64,
    pub bytes: i64,
    pub fail_at: i64,
}

/// Install counting hooks (counters reset); allocation number `fail_at` fails, `0` = never.
pub fn hooks_on(fail_at: i64) {
    N_ALLOC.store(0, Ordering::SeqCst);
    N_FREE.store(0, Ordering::SeqCst);
    N_BYTES.store(0, Ordering::SeqCst);
    FAIL_AT.store(fail_at, Ordering::SeqCst);
    lib::init_hooks(Some(Arc::new(CountingHooks)));
}

/// `cJSON_InitHooks(NULL)`: back to the default allocator; the counters keep their values.
pub fn hooks_off() {
    lib::init_hooks(None);
    FAIL_AT.store(0, Ordering::SeqCst);
}

/// Install hooks whose malloc always fails (upstream `failing_hooks`).
pub fn hooks_failing_malloc() {
    lib::init_hooks(Some(Arc::new(FailingHooks)));
}

pub fn hook_stats() -> HookStats {
    HookStats {
        allocs: N_ALLOC.load(Ordering::SeqCst),
        frees: N_FREE.load(Ordering::SeqCst),
        bytes: N_BYTES.load(Ordering::SeqCst),
        fail_at: FAIL_AT.load(Ordering::SeqCst),
    }
}

// ---- cJSON_Utils ---------------------------------------------------------------------------------------------

pub fn get_pointer(object: Item, pointer: Option<&[u8]>) -> Item {
    Item(utils::get_pointer(&store(), object.0, pointer))
}

pub fn get_pointer_cs(object: Item, pointer: Option<&[u8]>) -> Item {
    Item(utils::get_pointer_case_sensitive(
        &store(),
        object.0,
        pointer,
    ))
}

pub fn generate_patches(from: Item, to: Item) -> Item {
    Item(utils::generate_patches(&mut store(), from.0, to.0))
}

pub fn generate_patches_cs(from: Item, to: Item) -> Item {
    Item(utils::generate_patches_case_sensitive(
        &mut store(),
        from.0,
        to.0,
    ))
}

pub fn add_patch_to_array(array: Item, operation: Option<&[u8]>, path: Option<&[u8]>, value: Item) {
    utils::add_patch_to_array(&mut store(), array.0, operation, path, value.0);
}

fn status(result: Result<(), utils::PatchError>) -> i32 {
    result.map_or_else(utils::PatchError::code, |()| 0)
}

/// Status code `0..=13` (Q23).
pub fn apply_patches(object: Item, patches: Item) -> i32 {
    status(utils::apply_patches(&mut store(), object.0, patches.0))
}

pub fn apply_patches_cs(object: Item, patches: Item) -> i32 {
    status(utils::apply_patches_case_sensitive(
        &mut store(),
        object.0,
        patches.0,
    ))
}

/// Consumes `target`; returns the new root.
pub fn merge_patch(target: Item, patch: Item) -> Item {
    Item(utils::merge_patch(&mut store(), target.0, patch.0))
}

pub fn merge_patch_cs(target: Item, patch: Item) -> Item {
    Item(utils::merge_patch_case_sensitive(
        &mut store(),
        target.0,
        patch.0,
    ))
}

pub fn generate_merge_patch(from: Item, to: Item) -> Item {
    Item(utils::generate_merge_patch(&mut store(), from.0, to.0))
}

pub fn generate_merge_patch_cs(from: Item, to: Item) -> Item {
    Item(utils::generate_merge_patch_case_sensitive(
        &mut store(),
        from.0,
        to.0,
    ))
}

pub fn find_pointer_from_object_to(object: Item, target: Item) -> Option<Vec<u8>> {
    let found = utils::find_pointer_from_object_to(&store(), object.0, target.0);
    // The caller's `cJSON_free`: dropping the string is the free event.
    found.map(|p| p.bytes().to_vec())
}

pub fn sort_object(object: Item) {
    utils::sort_object(&mut store(), object.0);
}

pub fn sort_object_cs(object: Item) {
    utils::sort_object_case_sensitive(&mut store(), object.0);
}

// ---- raw access (C_LAYOUT only; no Stage 2 equivalent) -------------------------------------------------------

const NO_RAW: &str = "adapter: raw_* needs the C node layout (C_LAYOUT is false, RULEBOOK D7, D14)";

/// A fresh zeroed node with `type` = `t` (the upstream tests' stack `cJSON invalid[1]`).
pub fn raw_new_with_type(_t: i32) -> Item {
    panic!("{NO_RAW}")
}

pub fn raw_set_prev(_item: Item, _prev: Item) {
    panic!("{NO_RAW}")
}

/// Free `valuestring` through the hooks and set it to NULL (upstream `corruptedString`).
pub fn raw_free_valuestring(_item: Item) {
    panic!("{NO_RAW}")
}

/// `cJSON_AddItemToObject(object, item->string, item)` with the key aliasing the item's own `string` (D7).
pub fn raw_add_item_with_aliased_key(_object: Item, _item: Item, _name: &[u8]) -> bool {
    panic!("{NO_RAW}")
}

/// `cJSON_SetValuestring(item, item->valuestring + offset)`: the new value overlaps the old buffer (D7).
pub fn raw_set_valuestring_from_own(_item: Item, _offset: usize) -> Option<Vec<u8>> {
    panic!("{NO_RAW}")
}
