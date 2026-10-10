//! Judge adapter (judge.md section 5): the only test-side file that calls the `cjson` crate.
//!
//! STAGE 1 VERSION. It calls the C-ABI functions of `cJSON.h` / `cJSON_Utils.h` in `unsafe` blocks (allowed:
//! `tests/` is outside the zero-`unsafe` scope of `src/`) and exposes a plain, safe, C-shaped interface to
//! `tests/upstream.rs` and `examples/dump.rs`. Stage 2 replaces ONLY this file with an implementation of the
//! same interface over the RULEBOOK section 6 API. Neither the driver nor the tests may change.
//!
//! # The interface contract
//!
//! Handles. [`Item`] is an opaque, `Copy`, comparable handle for "a `cJSON *`" (possibly NULL: [`NULL`]).
//! Equality means "the same node" (the C pointer comparisons of the upstream tests). Ownership is manual, as
//! in C: `create_*`, `parse*`, `duplicate`, `detach_*` return a root the caller must `delete`; `add_*`,
//! `insert_*`, `replace_*` take ownership of the item on success (and leave it with the caller on failure);
//! `get_*`, `child`, `next`, `prev` return borrowed handles. A Stage 2 adapter implements `Item` over its
//! own arena or tree path and keeps these ownership rules.
//!
//! Strings. Byte strings are `&[u8]` WITHOUT the terminating NUL (the adapter adds it). `None` is the C
//! NULL pointer. A string that the library keeps by pointer (`create_string_reference`,
//! `add_item_to_object_cs`) is `&'static CStr`. Returned strings are `Vec<u8>`/[`Printed`] holding the bytes up
//! to the first NUL.
//!
//! Sentinels. Booleans are `bool`; `cJSON_bool` results are converted. Error position and parse end are
//! offsets into the input slice ([`Parsed`]); `ErrPos::Unset` is the NULL error pointer.
//!
//! Capabilities (constants, so that a Stage 2 adapter can say "not available" without touching the tests):
//! - [`HOOKS`]: `cJSON_InitHooks` allocator hooks exist (RULEBOOK D1). Stage 2: `false`. Tests that need
//!   them print `SKIPPED` and return; the driver skips its allocator section (and `run.py --stage 2` builds
//!   the original C driver with `-DDUMP_NO_HOOKS` when it reads `HOOKS = false` from this file).
//! - [`C_LAYOUT`]: the `raw_*` functions at the end of this file work (writes into node fields, corrupted
//!   lists, pointer identity of buffers; RULEBOOK D7, D11, D14). Stage 2: `false`.
//!
//! Locking. The library has process-global state (hooks, error position, RULEBOOK D1/D9). Tests run on
//! threads, so every test holds `upstream.rs::lock()`; the adapter itself is not synchronised.
#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicI64, Ordering};

use cjson::cJSON as lib;
use cjson::cJSON::{cJSON, cJSON_Hooks};
use cjson::cJSON_Utils as utils;

/// Stage 1: allocator hooks exist.
pub const HOOKS: bool = true;
/// Stage 1: the `raw_*` functions work.
pub const C_LAYOUT: bool = true;

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

pub const NESTING_LIMIT: usize = 1000;
pub const CIRCULAR_LIMIT: usize = 10000;

unsafe extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

// ---- handles and argument plumbing ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Item(*mut cJSON);

/// The C NULL.
pub const NULL: Item = Item(null_mut());

impl Item {
    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

/// A NUL-terminated copy of an optional byte string; `ptr()` is NULL for `None`.
struct CArg(Option<Vec<u8>>);

impl CArg {
    fn new(s: Option<&[u8]>) -> CArg {
        CArg(s.map(|s| {
            let mut v = Vec::with_capacity(s.len() + 1);
            v.extend_from_slice(s);
            v.push(0);
            v
        }))
    }
    fn ptr(&self) -> *const c_char {
        self.0.as_ref().map_or(null(), |v| v.as_ptr() as *const c_char)
    }
}

fn opt_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        // SAFETY: the library hands out NUL-terminated strings that stay alive until the next mutation.
        Some(unsafe { CStr::from_ptr(p) }.to_bytes().to_vec())
    }
}

fn b(v: c_int) -> bool {
    v != 0
}

fn cb(v: bool) -> c_int {
    v as c_int
}

// ---- parse ---------------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ErrPos {
    /// `cJSON_GetErrorPtr()` is NULL.
    Unset,
    /// Offset of the error pointer into the input.
    At(usize),
    /// The error pointer is not inside the input (or the input was NULL).
    Outside,
}

pub struct Parsed {
    pub item: Item,
    /// Offset of `*return_parse_end` (only when `want_end` was set and the library wrote it).
    pub end: Option<usize>,
    pub err: ErrPos,
}

enum ParseCall {
    Plain,
    WithLength(usize),
    Opts { require_nul: bool, want_end: bool },
    LengthOpts { len: usize, require_nul: bool, want_end: bool },
}

fn do_parse(text: Option<&[u8]>, call: ParseCall) -> Parsed {
    let arg = CArg::new(text);
    let base = arg.ptr();
    let mut end: *const c_char = null();
    // SAFETY: `base` is NULL or a NUL-terminated buffer of `text.len() + 1` bytes that outlives the call.
    let item = unsafe {
        match call {
            ParseCall::Plain => lib::cJSON_Parse(base),
            ParseCall::WithLength(len) => lib::cJSON_ParseWithLength(base, len),
            ParseCall::Opts { require_nul, want_end } => lib::cJSON_ParseWithOpts(
                base,
                if want_end { &raw mut end } else { null_mut() },
                cb(require_nul),
            ),
            ParseCall::LengthOpts { len, require_nul, want_end } => lib::cJSON_ParseWithLengthOpts(
                base,
                len,
                if want_end { &raw mut end } else { null_mut() },
                cb(require_nul),
            ),
        }
    };
    // SAFETY: plain read of the global error pointer.
    let e = unsafe { lib::cJSON_GetErrorPtr() };
    let err = if e.is_null() {
        ErrPos::Unset
    } else if !base.is_null() && e >= base {
        ErrPos::At(e as usize - base as usize)
    } else {
        ErrPos::Outside
    };
    let end = if end.is_null() || base.is_null() { None } else { Some(end as usize - base as usize) };
    Parsed { item: Item(item), end, err }
}

/// `cJSON_Parse`.
pub fn parse(text: Option<&[u8]>) -> Parsed {
    do_parse(text, ParseCall::Plain)
}

/// `cJSON_ParseWithLength`: `len` is passed as is (it may include the terminator).
pub fn parse_with_length(text: Option<&[u8]>, len: usize) -> Parsed {
    do_parse(text, ParseCall::WithLength(len))
}

/// `cJSON_ParseWithOpts`; `want_end` = pass a non-NULL `return_parse_end`.
pub fn parse_with_opts(text: Option<&[u8]>, require_nul: bool, want_end: bool) -> Parsed {
    do_parse(text, ParseCall::Opts { require_nul, want_end })
}

/// `cJSON_ParseWithLengthOpts`.
pub fn parse_with_length_opts(text: Option<&[u8]>, len: usize, require_nul: bool, want_end: bool) -> Parsed {
    do_parse(text, ParseCall::LengthOpts { len, require_nul, want_end })
}

// ---- print ---------------------------------------------------------------------------------------------------

/// A string returned by a print function; freed with `cJSON_free` when dropped (so the driver controls
/// when the allocator hooks see the free).
pub struct Printed(*mut c_char);

impl Printed {
    pub fn bytes(&self) -> &[u8] {
        // SAFETY: non-NULL by construction, NUL-terminated, alive until drop.
        unsafe { CStr::from_ptr(self.0) }.to_bytes()
    }
}

impl Drop for Printed {
    fn drop(&mut self) {
        // SAFETY: allocated by the library's print functions through the active hooks.
        unsafe { lib::cJSON_free(self.0 as *mut c_void) }
    }
}

fn printed(p: *mut c_char) -> Option<Printed> {
    if p.is_null() {
        None
    } else {
        Some(Printed(p))
    }
}

/// `cJSON_Print` (formatted).
pub fn print(item: Item) -> Option<Printed> {
    // SAFETY: `item` is NULL or a live node.
    printed(unsafe { lib::cJSON_Print(item.0) })
}

/// `cJSON_PrintUnformatted`.
pub fn print_unformatted(item: Item) -> Option<Printed> {
    // SAFETY: as above.
    printed(unsafe { lib::cJSON_PrintUnformatted(item.0) })
}

/// `cJSON_PrintBuffered`.
pub fn print_buffered(item: Item, prebuffer: i32, fmt: bool) -> Option<Printed> {
    // SAFETY: as above.
    printed(unsafe { lib::cJSON_PrintBuffered(item.0, prebuffer, cb(fmt)) })
}

/// `cJSON_PrintPreallocated(item, buf, length, fmt)`; `length` must not exceed `buf.len()` when `buf` is
/// `Some`. The C bytes written are visible in `buf` afterwards (also after a `false` result, Q6).
pub fn print_preallocated(item: Item, buf: Option<&mut [u8]>, length: i32, fmt: bool) -> bool {
    let p = match buf {
        Some(s) => {
            assert!(length < 0 || length as usize <= s.len(), "adapter: length exceeds the buffer");
            s.as_mut_ptr() as *mut c_char
        }
        None => null_mut(),
    };
    // SAFETY: `p` is NULL or valid for `length` bytes (asserted above).
    b(unsafe { lib::cJSON_PrintPreallocated(item.0, p, length, cb(fmt)) })
}

pub fn version() -> Vec<u8> {
    // SAFETY: returns a static string.
    opt_bytes(unsafe { lib::cJSON_Version() }).unwrap_or_default()
}

// ---- node access ---------------------------------------------------------------------------------------------

pub fn delete(item: Item) {
    // SAFETY: `item` is NULL or a root the caller owns.
    unsafe { lib::cJSON_Delete(item.0) }
}

/// The full `type` word (kind bits plus `IS_REFERENCE` / `STRING_IS_CONST`).
pub fn type_flags(item: Item) -> i32 {
    assert!(!item.is_null());
    // SAFETY: non-NULL live node.
    unsafe { (*item.0).type_0 }
}

pub fn value_int(item: Item) -> i32 {
    assert!(!item.is_null());
    // SAFETY: as above.
    unsafe { (*item.0).valueint }
}

pub fn value_double(item: Item) -> f64 {
    assert!(!item.is_null());
    // SAFETY: as above.
    unsafe { (*item.0).valuedouble }
}

/// The `string` member (the key), `None` when NULL.
pub fn key(item: Item) -> Option<Vec<u8>> {
    assert!(!item.is_null());
    // SAFETY: as above.
    opt_bytes(unsafe { (*item.0).string })
}

/// The `valuestring` member, `None` when NULL.
pub fn value_string(item: Item) -> Option<Vec<u8>> {
    assert!(!item.is_null());
    // SAFETY: as above.
    opt_bytes(unsafe { (*item.0).valuestring })
}

pub fn child(item: Item) -> Item {
    assert!(!item.is_null());
    // SAFETY: as above.
    Item(unsafe { (*item.0).child })
}

pub fn next(item: Item) -> Item {
    assert!(!item.is_null());
    // SAFETY: as above.
    Item(unsafe { (*item.0).next })
}

/// `prev` of the first sibling is the last sibling (cJSON invariant); NULL for a detached node.
pub fn prev(item: Item) -> Item {
    assert!(!item.is_null());
    // SAFETY: as above.
    Item(unsafe { (*item.0).prev })
}

/// `cJSON_ArrayForEach`: the children of `item` in order (empty for NULL).
pub fn children(item: Item) -> Vec<Item> {
    let mut v = Vec::new();
    if item.is_null() {
        return v;
    }
    let mut c = child(item);
    while !c.is_null() {
        v.push(c);
        c = next(c);
    }
    v
}

pub fn is_invalid(item: Item) -> bool {
    // SAFETY: `item` is NULL or a live node (all the predicates below).
    b(unsafe { lib::cJSON_IsInvalid(item.0) })
}
pub fn is_false(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsFalse(item.0) })
}
pub fn is_true(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsTrue(item.0) })
}
pub fn is_bool(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsBool(item.0) })
}
pub fn is_null(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsNull(item.0) })
}
pub fn is_number(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsNumber(item.0) })
}
pub fn is_string(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsString(item.0) })
}
pub fn is_array(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsArray(item.0) })
}
pub fn is_object(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsObject(item.0) })
}
pub fn is_raw(item: Item) -> bool {
    b(unsafe { lib::cJSON_IsRaw(item.0) })
}

pub fn array_size(item: Item) -> i32 {
    // SAFETY: NULL or live node.
    unsafe { lib::cJSON_GetArraySize(item.0) }
}

pub fn array_item(item: Item, index: i32) -> Item {
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_GetArrayItem(item.0, index) })
}

pub fn object_item(item: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    // SAFETY: as above; `n` outlives the call.
    Item(unsafe { lib::cJSON_GetObjectItem(item.0, n.ptr()) })
}

pub fn object_item_cs(item: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_GetObjectItemCaseSensitive(item.0, n.ptr()) })
}

pub fn has_object_item(item: Item, name: Option<&[u8]>) -> bool {
    let n = CArg::new(name);
    // SAFETY: as above.
    b(unsafe { lib::cJSON_HasObjectItem(item.0, n.ptr()) })
}

/// `cJSON_GetStringValue`.
pub fn get_string_value(item: Item) -> Option<Vec<u8>> {
    // SAFETY: NULL or live node.
    opt_bytes(unsafe { lib::cJSON_GetStringValue(item.0) })
}

/// `cJSON_GetNumberValue` (NaN for a non-number or NULL, Q24).
pub fn get_number_value(item: Item) -> f64 {
    // SAFETY: as above.
    unsafe { lib::cJSON_GetNumberValue(item.0) }
}

// ---- constructors --------------------------------------------------------------------------------------------

pub fn create_null() -> Item {
    // SAFETY: no preconditions.
    Item(unsafe { lib::cJSON_CreateNull() })
}
pub fn create_true() -> Item {
    Item(unsafe { lib::cJSON_CreateTrue() })
}
pub fn create_false() -> Item {
    Item(unsafe { lib::cJSON_CreateFalse() })
}
pub fn create_bool(v: bool) -> Item {
    Item(unsafe { lib::cJSON_CreateBool(cb(v)) })
}
pub fn create_number(v: f64) -> Item {
    Item(unsafe { lib::cJSON_CreateNumber(v) })
}
pub fn create_string(s: Option<&[u8]>) -> Item {
    let a = CArg::new(s);
    // SAFETY: `a` outlives the call; the library copies the string.
    Item(unsafe { lib::cJSON_CreateString(a.ptr()) })
}
pub fn create_raw(s: Option<&[u8]>) -> Item {
    let a = CArg::new(s);
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_CreateRaw(a.ptr()) })
}
pub fn create_array() -> Item {
    Item(unsafe { lib::cJSON_CreateArray() })
}
pub fn create_object() -> Item {
    Item(unsafe { lib::cJSON_CreateObject() })
}

/// `cJSON_CreateStringReference`: the item points at `s` itself (no copy).
pub fn create_string_reference(s: Option<&'static CStr>) -> Item {
    // SAFETY: `s` is 'static, so the reference never dangles.
    Item(unsafe { lib::cJSON_CreateStringReference(s.map_or(null(), |s| s.as_ptr())) })
}

/// `cJSON_CreateObjectReference`: the new item's `child` is `child` (no copy, RULEBOOK D12).
pub fn create_object_reference(child: Item) -> Item {
    // SAFETY: caller keeps `child` alive for the reference's lifetime (as in C).
    Item(unsafe { lib::cJSON_CreateObjectReference(child.0) })
}
pub fn create_array_reference(child: Item) -> Item {
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_CreateArrayReference(child.0) })
}

/// `cJSON_CreateIntArray(numbers, count)`; `numbers = None` is NULL (then `count` is only passed on).
pub fn create_int_array(numbers: Option<&[i32]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    // SAFETY: NULL or valid for `count` elements (asserted).
    Item(unsafe { lib::cJSON_CreateIntArray(numbers.map_or(null(), |n| n.as_ptr()), count) })
}
pub fn create_float_array(numbers: Option<&[f32]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_CreateFloatArray(numbers.map_or(null(), |n| n.as_ptr()), count) })
}
pub fn create_double_array(numbers: Option<&[f64]>, count: i32) -> Item {
    if let Some(n) = numbers {
        assert!(count < 0 || count as usize <= n.len());
    }
    // SAFETY: as above.
    Item(unsafe { lib::cJSON_CreateDoubleArray(numbers.map_or(null(), |n| n.as_ptr()), count) })
}
pub fn create_string_array(strings: Option<&[&[u8]]>, count: i32) -> Item {
    if let Some(s) = strings {
        assert!(count < 0 || count as usize <= s.len());
    }
    let owned: Vec<CArg> = strings.map_or(Vec::new(), |s| s.iter().map(|x| CArg::new(Some(x))).collect());
    let ptrs: Vec<*const c_char> = owned.iter().map(CArg::ptr).collect();
    let p = if strings.is_some() { ptrs.as_ptr() } else { null() };
    // SAFETY: `ptrs` holds `strings.len()` valid NUL-terminated pointers (asserted `count` bound).
    Item(unsafe { lib::cJSON_CreateStringArray(p, count) })
}

// ---- add / detach / delete / insert / replace ----------------------------------------------------------------

pub fn add_item_to_array(array: Item, item: Item) -> bool {
    // SAFETY: handles are NULL or live nodes owned by the caller (ownership rules in the header).
    b(unsafe { lib::cJSON_AddItemToArray(array.0, item.0) })
}

pub fn add_item_to_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    let n = CArg::new(name);
    // SAFETY: the library copies the key.
    b(unsafe { lib::cJSON_AddItemToObject(object.0, n.ptr(), item.0) })
}

/// `cJSON_AddItemToObjectCS`: the key is stored by pointer, so it must be `'static`.
pub fn add_item_to_object_cs(object: Item, name: Option<&'static CStr>, item: Item) -> bool {
    // SAFETY: `name` is 'static.
    b(unsafe { lib::cJSON_AddItemToObjectCS(object.0, name.map_or(null(), |s| s.as_ptr()), item.0) })
}

pub fn add_item_reference_to_array(array: Item, item: Item) -> bool {
    // SAFETY: as add_item_to_array.
    b(unsafe { lib::cJSON_AddItemReferenceToArray(array.0, item.0) })
}

pub fn add_item_reference_to_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    let n = CArg::new(name);
    // SAFETY: as add_item_to_object.
    b(unsafe { lib::cJSON_AddItemReferenceToObject(object.0, n.ptr(), item.0) })
}

pub fn detach_item_via_pointer(parent: Item, item: Item) -> Item {
    // SAFETY: handles are NULL or live nodes.
    Item(unsafe { lib::cJSON_DetachItemViaPointer(parent.0, item.0) })
}
pub fn detach_item_from_array(array: Item, which: i32) -> Item {
    Item(unsafe { lib::cJSON_DetachItemFromArray(array.0, which) })
}
pub fn delete_item_from_array(array: Item, which: i32) {
    unsafe { lib::cJSON_DeleteItemFromArray(array.0, which) }
}
pub fn detach_item_from_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_DetachItemFromObject(object.0, n.ptr()) })
}
pub fn detach_item_from_object_cs(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_DetachItemFromObjectCaseSensitive(object.0, n.ptr()) })
}
pub fn delete_item_from_object(object: Item, name: Option<&[u8]>) {
    let n = CArg::new(name);
    unsafe { lib::cJSON_DeleteItemFromObject(object.0, n.ptr()) }
}
pub fn delete_item_from_object_cs(object: Item, name: Option<&[u8]>) {
    let n = CArg::new(name);
    unsafe { lib::cJSON_DeleteItemFromObjectCaseSensitive(object.0, n.ptr()) }
}

pub fn insert_item_in_array(array: Item, which: i32, item: Item) -> bool {
    b(unsafe { lib::cJSON_InsertItemInArray(array.0, which, item.0) })
}
pub fn replace_item_via_pointer(parent: Item, item: Item, replacement: Item) -> bool {
    b(unsafe { lib::cJSON_ReplaceItemViaPointer(parent.0, item.0, replacement.0) })
}
pub fn replace_item_in_array(array: Item, which: i32, item: Item) -> bool {
    b(unsafe { lib::cJSON_ReplaceItemInArray(array.0, which, item.0) })
}
pub fn replace_item_in_object(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    let n = CArg::new(name);
    b(unsafe { lib::cJSON_ReplaceItemInObject(object.0, n.ptr(), item.0) })
}
pub fn replace_item_in_object_cs(object: Item, name: Option<&[u8]>, item: Item) -> bool {
    let n = CArg::new(name);
    b(unsafe { lib::cJSON_ReplaceItemInObjectCaseSensitive(object.0, n.ptr(), item.0) })
}

pub fn add_null_to_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddNullToObject(object.0, n.ptr()) })
}
pub fn add_true_to_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddTrueToObject(object.0, n.ptr()) })
}
pub fn add_false_to_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddFalseToObject(object.0, n.ptr()) })
}
pub fn add_bool_to_object(object: Item, name: Option<&[u8]>, v: bool) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddBoolToObject(object.0, n.ptr(), cb(v)) })
}
pub fn add_number_to_object(object: Item, name: Option<&[u8]>, v: f64) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddNumberToObject(object.0, n.ptr(), v) })
}
pub fn add_string_to_object(object: Item, name: Option<&[u8]>, s: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    let v = CArg::new(s);
    Item(unsafe { lib::cJSON_AddStringToObject(object.0, n.ptr(), v.ptr()) })
}
pub fn add_raw_to_object(object: Item, name: Option<&[u8]>, raw: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    let v = CArg::new(raw);
    Item(unsafe { lib::cJSON_AddRawToObject(object.0, n.ptr(), v.ptr()) })
}
pub fn add_object_to_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddObjectToObject(object.0, n.ptr()) })
}
pub fn add_array_to_object(object: Item, name: Option<&[u8]>) -> Item {
    let n = CArg::new(name);
    Item(unsafe { lib::cJSON_AddArrayToObject(object.0, n.ptr()) })
}

// ---- whole-tree operations -----------------------------------------------------------------------------------

pub fn duplicate(item: Item, recurse: bool) -> Item {
    // SAFETY: NULL or live node.
    Item(unsafe { lib::cJSON_Duplicate(item.0, cb(recurse)) })
}

pub fn compare(a: Item, bb: Item, case_sensitive: bool) -> bool {
    // SAFETY: as above.
    b(unsafe { lib::cJSON_Compare(a.0, bb.0, cb(case_sensitive)) })
}

/// `cJSON_Minify` on a private NUL-terminated copy of `json`; returns the bytes up to the first NUL
/// (`None` for the NULL call).
pub fn minify(json: Option<&[u8]>) -> Option<Vec<u8>> {
    let mut buf: Option<Vec<u8>> = json.map(|j| {
        let mut v = j.to_vec();
        v.push(0);
        v
    });
    let p = buf.as_mut().map_or(null_mut(), |v| v.as_mut_ptr() as *mut c_char);
    // SAFETY: NULL or a NUL-terminated buffer of exactly the input size plus one.
    unsafe { lib::cJSON_Minify(p) };
    buf.map(|v| {
        let n = v.iter().position(|&c| c == 0).unwrap_or(v.len());
        v[..n].to_vec()
    })
}

/// `cJSON_SetNumberValue(item, v)` (the macro: NULL item returns `v`, else `cJSON_SetNumberHelper`).
pub fn set_number_value(item: Item, v: f64) -> f64 {
    if item.is_null() {
        v
    } else {
        // SAFETY: live node.
        unsafe { lib::cJSON_SetNumberHelper(item.0, v) }
    }
}

/// `cJSON_SetNumberHelper` directly (also for NULL, Q24).
pub fn set_number_helper(item: Item, v: f64) -> f64 {
    // SAFETY: NULL or live node.
    unsafe { lib::cJSON_SetNumberHelper(item.0, v) }
}

/// `cJSON_SetValuestring`: the new valuestring, `None` when the call fails.
pub fn set_valuestring(item: Item, s: Option<&[u8]>) -> Option<Vec<u8>> {
    let a = CArg::new(s);
    // SAFETY: NULL or live node; `a` outlives the call.
    opt_bytes(unsafe { lib::cJSON_SetValuestring(item.0, a.ptr()) })
}

/// The `cJSON_SetBoolValue` macro of `cJSON.h`: returns the new type word (`INVALID` when not a bool).
pub fn set_bool_value(item: Item, v: bool) -> i32 {
    if item.is_null() {
        return INVALID;
    }
    // SAFETY: live node; the macro writes `type` the same way.
    unsafe {
        let t = (*item.0).type_0;
        if t & (FALSE | TRUE) != 0 {
            (*item.0).type_0 = (t & !(FALSE | TRUE)) | if v { TRUE } else { FALSE };
            (*item.0).type_0
        } else {
            INVALID
        }
    }
}

// ---- allocator hooks (RULEBOOK D1) ---------------------------------------------------------------------------

static N_ALLOC: AtomicI64 = AtomicI64::new(0);
static N_FREE: AtomicI64 = AtomicI64::new(0);
static N_BYTES: AtomicI64 = AtomicI64::new(0);
static FAIL_AT: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn h_malloc(n: usize) -> *mut c_void {
    let a = N_ALLOC.fetch_add(1, Ordering::SeqCst) + 1;
    let f = FAIL_AT.load(Ordering::SeqCst);
    if f != 0 && a == f {
        return null_mut();
    }
    N_BYTES.fetch_add(n as i64, Ordering::SeqCst);
    unsafe { malloc(n) }
}

unsafe extern "C" fn h_free(p: *mut c_void) {
    if !p.is_null() {
        N_FREE.fetch_add(1, Ordering::SeqCst);
    }
    unsafe { free(p) }
}

unsafe extern "C" fn failing_malloc(_n: usize) -> *mut c_void {
    null_mut()
}

unsafe extern "C" fn normal_free(p: *mut c_void) {
    unsafe { free(p) }
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
    let mut h = cJSON_Hooks { malloc_fn: Some(h_malloc), free_fn: Some(h_free) };
    // SAFETY: the library copies the hook struct.
    unsafe { lib::cJSON_InitHooks(&raw mut h) }
}

/// `cJSON_InitHooks(NULL)`: back to the default allocator; the counters keep their values.
pub fn hooks_off() {
    // SAFETY: NULL restores the defaults.
    unsafe { lib::cJSON_InitHooks(null_mut()) };
    FAIL_AT.store(0, Ordering::SeqCst);
}

/// Install hooks whose malloc always fails (upstream `failing_hooks`).
pub fn hooks_failing_malloc() {
    let mut h = cJSON_Hooks { malloc_fn: Some(failing_malloc), free_fn: Some(normal_free) };
    // SAFETY: the library copies the hook struct.
    unsafe { lib::cJSON_InitHooks(&raw mut h) }
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
    let p = CArg::new(pointer);
    // SAFETY: NULL or live node; `p` outlives the call.
    Item(unsafe { utils::cJSONUtils_GetPointer(object.0.cast(), p.ptr()) }.cast())
}

pub fn get_pointer_cs(object: Item, pointer: Option<&[u8]>) -> Item {
    let p = CArg::new(pointer);
    // SAFETY: as above.
    Item(unsafe { utils::cJSONUtils_GetPointerCaseSensitive(object.0.cast(), p.ptr()) }.cast())
}

pub fn generate_patches(from: Item, to: Item) -> Item {
    // SAFETY: NULL or live nodes.
    Item(unsafe { utils::cJSONUtils_GeneratePatches(from.0.cast(), to.0.cast()) }.cast())
}

pub fn generate_patches_cs(from: Item, to: Item) -> Item {
    // SAFETY: as above.
    Item(unsafe { utils::cJSONUtils_GeneratePatchesCaseSensitive(from.0.cast(), to.0.cast()) }.cast())
}

pub fn add_patch_to_array(array: Item, operation: Option<&[u8]>, path: Option<&[u8]>, value: Item) {
    let o = CArg::new(operation);
    let p = CArg::new(path);
    // SAFETY: as above; the library copies the strings.
    unsafe { utils::cJSONUtils_AddPatchToArray(array.0.cast(), o.ptr(), p.ptr(), value.0.cast()) }
}

/// Status code `0..=13` (Q23).
pub fn apply_patches(object: Item, patches: Item) -> i32 {
    // SAFETY: NULL or live nodes.
    unsafe { utils::cJSONUtils_ApplyPatches(object.0.cast(), patches.0.cast()) }
}

pub fn apply_patches_cs(object: Item, patches: Item) -> i32 {
    // SAFETY: as above.
    unsafe { utils::cJSONUtils_ApplyPatchesCaseSensitive(object.0.cast(), patches.0.cast()) }
}

/// Consumes `target`; returns the new root.
pub fn merge_patch(target: Item, patch: Item) -> Item {
    // SAFETY: `target` is NULL or a root the caller owns; `patch` is NULL or live.
    Item(unsafe { utils::cJSONUtils_MergePatch(target.0.cast(), patch.0.cast()) }.cast())
}

pub fn merge_patch_cs(target: Item, patch: Item) -> Item {
    // SAFETY: as above.
    Item(unsafe { utils::cJSONUtils_MergePatchCaseSensitive(target.0.cast(), patch.0.cast()) }.cast())
}

pub fn generate_merge_patch(from: Item, to: Item) -> Item {
    // SAFETY: NULL or live nodes.
    Item(unsafe { utils::cJSONUtils_GenerateMergePatch(from.0.cast(), to.0.cast()) }.cast())
}

pub fn generate_merge_patch_cs(from: Item, to: Item) -> Item {
    // SAFETY: as above.
    Item(unsafe { utils::cJSONUtils_GenerateMergePatchCaseSensitive(from.0.cast(), to.0.cast()) }.cast())
}

pub fn find_pointer_from_object_to(object: Item, target: Item) -> Option<Vec<u8>> {
    // SAFETY: NULL or live nodes; the result is freed with `cJSON_free`.
    let p = unsafe { utils::cJSONUtils_FindPointerFromObjectTo(object.0.cast(), target.0.cast()) };
    let out = opt_bytes(p);
    if !p.is_null() {
        // SAFETY: allocated by the library through the active hooks.
        unsafe { lib::cJSON_free(p as *mut c_void) };
    }
    out
}

pub fn sort_object(object: Item) {
    // SAFETY: NULL or live node.
    unsafe { utils::cJSONUtils_SortObject(object.0.cast()) }
}

pub fn sort_object_cs(object: Item) {
    // SAFETY: as above.
    unsafe { utils::cJSONUtils_SortObjectCaseSensitive(object.0.cast()) }
}

// ---- raw access (C_LAYOUT only; no Stage 2 equivalent) -------------------------------------------------------

/// A fresh zeroed node with `type` = `t` (the upstream tests' stack `cJSON invalid[1]`). Delete with
/// [`delete`].
pub fn raw_new_with_type(t: i32) -> Item {
    // SAFETY: CreateNull returns a zeroed node whose `type` we overwrite.
    unsafe {
        let n = lib::cJSON_CreateNull();
        if !n.is_null() {
            (*n).type_0 = t;
        }
        Item(n)
    }
}

pub fn raw_set_prev(item: Item, prev: Item) {
    assert!(!item.is_null());
    // SAFETY: live node.
    unsafe { (*item.0).prev = prev.0 }
}

/// Free `valuestring` through the hooks and set it to NULL (upstream `corruptedString`).
pub fn raw_free_valuestring(item: Item) {
    assert!(!item.is_null());
    // SAFETY: live node owning its valuestring.
    unsafe {
        lib::cJSON_free((*item.0).valuestring as *mut c_void);
        (*item.0).valuestring = null_mut();
    }
}

/// Give `item` a fresh heap copy of `name` as its `string`, then `cJSON_AddItemToObject(object, item->string,
/// item)`: the key argument aliases the item's own `string` (upstream use-after-free regression test).
pub fn raw_add_item_with_aliased_key(object: Item, item: Item, name: &[u8]) -> bool {
    assert!(!item.is_null());
    let a = CArg::new(Some(name));
    // SAFETY: the copy is allocated with `cJSON_malloc`, as `cJSON_strdup` does.
    unsafe {
        let n = a.0.as_ref().unwrap().len();
        let p = lib::cJSON_malloc(n) as *mut u8;
        assert!(!p.is_null());
        std::ptr::copy_nonoverlapping(a.ptr() as *const u8, p, n);
        (*item.0).string = p as *mut c_char;
        b(lib::cJSON_AddItemToObject(object.0, (*item.0).string, item.0))
    }
}

/// Address of the `valuestring` buffer (0 for NULL), for the "did not reallocate" assertions.
pub fn raw_value_string_addr(item: Item) -> usize {
    assert!(!item.is_null());
    // SAFETY: live node.
    unsafe { (*item.0).valuestring as usize }
}

/// `cJSON_SetValuestring(item, s)` returning the address of the result (0 for NULL).
pub fn raw_set_valuestring_addr(item: Item, s: &[u8]) -> usize {
    let a = CArg::new(Some(s));
    // SAFETY: live node.
    unsafe { lib::cJSON_SetValuestring(item.0, a.ptr()) as usize }
}

/// `cJSON_SetValuestring(item, item->valuestring + offset)`: the new value overlaps the old buffer (D7).
/// Returns the bytes of the result (`None` = NULL).
pub fn raw_set_valuestring_from_own(item: Item, offset: usize) -> Option<Vec<u8>> {
    assert!(!item.is_null());
    // SAFETY: live node whose valuestring is at least `offset + 1` bytes long (caller's duty).
    unsafe {
        let p = (*item.0).valuestring.add(offset);
        opt_bytes(lib::cJSON_SetValuestring(item.0, p))
    }
}
