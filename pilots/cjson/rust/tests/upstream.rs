//! Translation of the 21 upstream test programs of cJSON 1.7.19 (`pilots/cjson/src/tests/*.c`) through the
//! Judge adapter (judge.md section 5.3). Same inputs, same expected values. Changing this file is a Judge
//! change (own commit).
//!
//! Rules of the translation:
//! - The crate is reached only through `adapter`. Variadic or pointer-array helpers become slices.
//! - Tests that called private functions or wrote struct fields in C (`parse_*`, `print_*`, `ensure`,
//!   `skip_utf8_bom`, stack-built `cJSON` structs) go through the public API when the same behaviour is
//!   observable there (a parse test parses a whole document, a print test prints one). The cases where it
//!   is not are listed in `migration/RUN-NOTES.md`, section "Stage 2 Judge".
//! - A step that only exists with the C node layout (`adapter::C_LAYOUT`: hand-corrupted lists, a write into
//!   the node's own key, a node with two kind bits; RULEBOOK D7, D14) prints `SKIPPED` and is left out when
//!   the adapter says so; the Stage 1 adapter runs all of them. Allocator hooks (D1, revised 2026-10-10),
//!   cycles (D11, revised) and `Invalid` nodes (built by a root `remove` patch, Q23) are in every adapter.
//! - `ENABLE_LOCALES`: no upstream test file contains an `#ifdef`, so there is no `#[cfg(feature = "locales")]`
//!   here; every test runs in both feature sets (RULEBOOK section 3, both run in the C locale).
//! - The one `TEST_IGNORE()` test stays `#[ignore]`.
//! - Global library state (hooks, error position) is shared by every test: each test takes `lock()`.
mod adapter;

use adapter as a;
use adapter::{ErrPos, Item, NULL};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

// ---- helpers (common.h and Unity) ----------------------------------------------------------------------------

fn lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn data(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../src/tests")
        .join(rel)
}

/// `read_file` of common.h.
fn read_file(rel: &str) -> Vec<u8> {
    std::fs::read(data(rel)).unwrap_or_else(|e| panic!("Failed to read {rel}: {e}"))
}

fn pj(text: &str) -> Item {
    a::parse(Some(text.as_bytes())).item
}

fn pb(text: &[u8]) -> Item {
    a::parse(Some(text)).item
}

/// The kind bits (`type & 0xFF`, Unity `TEST_ASSERT_BITS(0xFF, ...)`).
fn kind(i: Item) -> i32 {
    a::type_flags(i) & 0xFF
}

fn vstr(i: Item) -> Option<String> {
    a::value_string(i).map(|v| String::from_utf8_lossy(&v).into_owned())
}

fn kstr(i: Item) -> Option<String> {
    a::key(i).map(|v| String::from_utf8_lossy(&v).into_owned())
}

fn pu(i: Item) -> Option<String> {
    a::print_unformatted(i).map(|p| String::from_utf8_lossy(p.bytes()).into_owned())
}

fn pf(i: Item) -> Option<String> {
    a::print(i).map(|p| String::from_utf8_lossy(p.bytes()).into_owned())
}

/// `TEST_ASSERT_EQUAL_DOUBLE`: Unity's relative tolerance of 1e-12.
fn assert_double(expected: f64, actual: f64) {
    let ok = expected == actual || (expected - actual).abs() <= expected.abs() * 1e-12;
    assert!(ok, "expected {expected:?}, got {actual:?}");
}

/// common.h `assert_not_in_list`.
fn assert_not_in_list(i: Item) {
    assert!(
        a::next(i).is_null(),
        "Linked list next pointer is not NULL."
    );
    assert!(
        a::prev(i).is_null(),
        "Linked list previous pointer is not NULL."
    );
}
fn assert_has_no_reference(i: Item) {
    assert_eq!(
        a::type_flags(i) & a::IS_REFERENCE,
        0,
        "Item should not have a string as reference."
    );
}
fn assert_has_no_const_string(i: Item) {
    assert_eq!(
        a::type_flags(i) & a::STRING_IS_CONST,
        0,
        "Item should not have a const string."
    );
}
fn assert_has_no_valuestring(i: Item) {
    assert!(a::value_string(i).is_none(), "Valuestring is not NULL.");
}
fn assert_has_no_string(i: Item) {
    assert!(a::key(i).is_none(), "String is not NULL.");
}
fn assert_has_no_child(i: Item) {
    assert!(a::child(i).is_null(), "Item has a child.");
}
fn assert_has_child(i: Item) {
    assert!(!a::child(i).is_null(), "Item doesn't have a child.");
}

/// Run `f` on a thread with a big stack: duplicating or deleting a tree `CIRCULAR_LIMIT` deep recurses that deep,
/// and a Stage 1 debug build needs more than the 2 MiB of a test thread (the C test runs on the 8 MiB main
/// thread). Panics propagate.
fn on_big_stack(f: impl FnOnce() + Send) {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(256 << 20)
            .spawn_scoped(s, f)
            .unwrap()
            .join()
            .unwrap();
    });
}

/// Print `SKIPPED` for a step the adapter cannot do (see the header).
fn skipped(why: &str) {
    eprintln!("SKIPPED: {why}");
}

/// The upstream `failing_hooks` (malloc always fails), reset to the defaults when the guard drops (D1).
struct HooksReset;
impl Drop for HooksReset {
    fn drop(&mut self) {
        a::hooks_off();
    }
}
fn failing_hooks() -> HooksReset {
    a::hooks_failing_malloc();
    HooksReset
}

/// An `Invalid` node (type word 0) through the public API: a JSON Patch `remove` at the root path `""`
/// overwrites the root with `{..., cJSON_Invalid, ...}` (cJSON_Utils.c:839-849, RULEBOOK Q23). The C tests
/// zero a stack node instead.
fn make_invalid() -> Item {
    let root = pj("{\"a\":1}");
    let patches = pj("[{\"op\":\"remove\",\"path\":\"\"}]");
    assert_eq!(a::apply_patches(root, patches), 0);
    a::delete(patches);
    assert_eq!(a::type_flags(root), a::INVALID);
    assert!(a::child(root).is_null() && a::key(root).is_none() && a::value_string(root).is_none());
    root
}

// =====================================================================================================================
// parse_examples.c
// =====================================================================================================================

fn do_test(name: &str) {
    let expected = read_file(&format!("inputs/{name}.expected"));
    let content = read_file(&format!("inputs/{name}"));
    let tree = a::parse(Some(&content)).item;
    assert!(!tree.is_null(), "Failed to read of parse test.");
    let actual = a::print(tree).expect("Failed to print tree back to JSON.");
    assert_eq!(
        String::from_utf8_lossy(&expected),
        String::from_utf8_lossy(actual.bytes())
    );
    drop(actual);
    a::delete(tree);
}

#[test]
fn file_test1_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test1");
}
#[test]
fn file_test2_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test2");
}
#[test]
fn file_test3_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test3");
}
#[test]
fn file_test4_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test4");
}
#[test]
fn file_test5_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test5");
}

#[test]
fn file_test6_should_not_be_parsed() {
    let _g = lock();
    let test6 = read_file("inputs/test6");
    let r = a::parse(Some(&test6));
    assert!(r.item.is_null(), "Should fail to parse what is not JSON.");
    assert_eq!(r.err, ErrPos::At(0), "Error pointer is incorrect.");
}

#[test]
fn file_test7_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test7");
}
#[test]
fn file_test8_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test8");
}
#[test]
fn file_test9_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test9");
}
#[test]
fn file_test10_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test10");
}
#[test]
fn file_test11_should_be_parsed_and_printed() {
    let _g = lock();
    do_test("test11");
}

#[test]
fn test12_should_not_be_parsed() {
    let _g = lock();
    let test12 = "{ \"name\": ";
    let r = a::parse(Some(test12.as_bytes()));
    assert!(r.item.is_null(), "Should fail to parse incomplete JSON.");
    assert_eq!(
        r.err,
        ErrPos::At(test12.len()),
        "Error pointer is incorrect."
    );
}

const TEST_13: &str = concat!(
    "{",
    "\"Image\":{",
    "\"Width\":800,",
    "\"Height\":600,",
    "\"Title\":\"Viewfrom15thFloor\",",
    "\"Thumbnail\":{",
    "\"Url\":\"http:/*www.example.com/image/481989943\",",
    "\"Height\":125,",
    "\"Width\":\"100\"",
    "},",
    "\"IDs\":[116,943,234,38793]",
    "}",
    "}"
);

#[test]
fn test13_should_be_parsed_without_null_termination() {
    let _g = lock();
    // The adapter copies the text into a NUL-terminated buffer; the length passed is the text length.
    let tree = a::parse_with_length(Some(TEST_13.as_bytes()), TEST_13.len()).item;
    assert!(!tree.is_null(), "Failed to parse valid json.");
    a::delete(tree);
}

#[test]
fn test14_should_not_be_parsed() {
    let _g = lock();
    let tree = a::parse_with_length(Some(TEST_13.as_bytes()), TEST_13.len() - 1).item;
    assert!(
        tree.is_null(),
        "Should not continue after buffer_length is reached."
    );
}

/// Address Sanitizer regression test; here: it must not crash.
#[test]
fn test15_should_not_heap_buffer_overflow() {
    let _g = lock();
    for s in ["{\"1\":1,", "{\"1\":1, "] {
        let json = a::parse_with_length(Some(s.as_bytes()), s.len()).item;
        a::delete(json);
    }
}

// =====================================================================================================================
// parse_number.c (private `parse_number` -> whole-document parse of a number)
// =====================================================================================================================

fn assert_is_number(n: Item) {
    assert!(!n.is_null(), "Item is NULL.");
    assert_not_in_list(n);
    assert_has_no_child(n);
    assert_eq!(kind(n), a::NUMBER, "Item doesn't have expected type.");
    assert_has_no_reference(n);
    assert_has_no_const_string(n);
    assert_has_no_valuestring(n);
    assert_has_no_string(n);
}

fn assert_parse_number(string: &str, integer: i32, real: f64) {
    let item = pj(string);
    assert_is_number(item);
    assert_eq!(integer, a::value_int(item));
    assert_double(real, a::value_double(item));
    a::delete(item);
}

fn assert_parse_big_number(string: &str) {
    let item = pj(string);
    assert_is_number(item);
    a::delete(item);
}

#[test]
fn parse_number_should_parse_zero() {
    let _g = lock();
    assert_parse_number("0", 0, 0.0);
    assert_parse_number("0.0", 0, 0.0);
    assert_parse_number("-0", 0, -0.0);
}

#[test]
fn parse_number_should_parse_negative_integers() {
    let _g = lock();
    assert_parse_number("-1", -1, -1.0);
    assert_parse_number("-32768", -32768, -32768.0);
    assert_parse_number("-2147483648", -2147483648.0_f64 as i32, -2147483648.0);
}

#[test]
fn parse_number_should_parse_positive_integers() {
    let _g = lock();
    assert_parse_number("1", 1, 1.0);
    assert_parse_number("32767", 32767, 32767.0);
    assert_parse_number("2147483647", 2147483647.0_f64 as i32, 2147483647.0);
}

#[test]
fn parse_number_should_parse_positive_reals() {
    let _g = lock();
    assert_parse_number("0.001", 0, 0.001);
    assert_parse_number("10e-10", 0, 10e-10);
    assert_parse_number("10E-10", 0, 10e-10);
    assert_parse_number("10e10", i32::MAX, 10e10);
    assert_parse_number("123e+127", i32::MAX, 123e127);
    assert_parse_number("123e-128", 0, 123e-128);
}

#[test]
fn parse_number_should_parse_negative_reals() {
    let _g = lock();
    assert_parse_number("-0.001", 0, -0.001);
    assert_parse_number("-10e-10", 0, -10e-10);
    assert_parse_number("-10E-10", 0, -10e-10);
    assert_parse_number("-10e20", i32::MIN, -10e20);
    assert_parse_number("-123e+127", i32::MIN, -123e127);
    assert_parse_number("-123e-128", 0, -123e-128);
}

#[test]
fn parse_number_should_parse_big_numbers() {
    let _g = lock();
    assert_parse_big_number("9999999999999999999999999999999999999999999999912345678901234567");
    assert_parse_big_number("9999999999999999999999999999999999999999999999912345678901234567E10");
    assert_parse_big_number("999999999999999999999999999999999999999999999991234567890.1234567");
}

// =====================================================================================================================
// parse_hex4.c (private `parse_hex4` -> the four hex digits of a `\uXXXX` escape)
// =====================================================================================================================

/// The UTF-8 bytes of the string value parsed from `"\u<digits>"` (an optional second escape follows).
fn parse_escape(digits: &str, second: Option<&str>) -> Option<Vec<u8>> {
    let text = match second {
        None => format!("\"\\u{digits}\""),
        Some(s) => format!("\"\\u{digits}\\u{s}\""),
    };
    let item = pj(&text);
    if item.is_null() {
        return None;
    }
    let v = a::value_string(item);
    a::delete(item);
    v
}

fn utf8(cp: u32) -> Vec<u8> {
    let mut b = [0u8; 4];
    char::from_u32(cp)
        .expect("scalar value")
        .encode_utf8(&mut b)
        .as_bytes()
        .to_vec()
}

#[test]
fn parse_hex4_should_parse_all_combinations() {
    let _g = lock();
    for number in 0u32..=0xFFFF {
        let lower = format!("{number:04x}");
        let upper = format!("{number:04X}");
        if (0xD800..=0xDBFF).contains(&number) {
            // a high surrogate: paired with U+DC00 it gives 0x10000 + ((number - 0xD800) << 10)
            let expected = utf8(0x10000 + ((number - 0xD800) << 10));
            assert_eq!(
                parse_escape(&lower, Some("dc00")),
                Some(expected.clone()),
                "Failed to parse lowercase digits."
            );
            assert_eq!(
                parse_escape(&upper, Some("DC00")),
                Some(expected),
                "Failed to parse uppercase digits."
            );
        } else if (0xDC00..=0xDFFF).contains(&number) {
            // a low surrogate as the second half of a pair with U+D800
            let expected = utf8(0x10000 + (number - 0xDC00));
            assert_eq!(
                parse_escape("d800", Some(&lower)),
                Some(expected.clone()),
                "Failed to parse lowercase digits."
            );
            assert_eq!(
                parse_escape("D800", Some(&upper)),
                Some(expected),
                "Failed to parse uppercase digits."
            );
        } else {
            let expected = if number == 0 {
                Vec::new()
            } else {
                utf8(number)
            };
            assert_eq!(
                parse_escape(&lower, None),
                Some(expected.clone()),
                "Failed to parse lowercase digits."
            );
            assert_eq!(
                parse_escape(&upper, None),
                Some(expected),
                "Failed to parse uppercase digits."
            );
        }
    }
}

#[test]
fn parse_hex4_should_parse_mixed_case() {
    let _g = lock();
    for d in [
        "beef", "beeF", "beEf", "beEF", "bEef", "bEeF", "bEEf", "bEEF", "Beef", "BeeF", "BeEf",
        "BeEF", "BEef", "BEeF", "BEEf", "BEEF",
    ] {
        assert_eq!(parse_escape(d, None), Some(utf8(0xBEEF)), "{d}");
    }
}

// =====================================================================================================================
// parse_string.c (private `parse_string` -> whole-document parse of a string)
// =====================================================================================================================

fn assert_parse_string(string: &str, expected: &str) {
    let item = pj(string);
    assert!(!item.is_null(), "Couldn't parse string.");
    assert_not_in_list(item);
    assert_has_no_child(item);
    assert_eq!(kind(item), a::STRING, "Item doesn't have expected type.");
    assert_has_no_reference(item);
    assert_has_no_const_string(item);
    assert!(a::value_string(item).is_some(), "Valuestring is NULL.");
    assert_has_no_string(item);
    assert_eq!(
        a::value_string(item).as_deref(),
        Some(expected.as_bytes()),
        "The parsed result isn't as expected."
    );
    a::delete(item);
}

fn assert_not_parse_string(string: &str) {
    let item = pj(string);
    assert!(
        item.is_null() || kind(item) != a::STRING,
        "Malformed string should not be accepted."
    );
    a::delete(item);
}

#[test]
fn parse_string_should_parse_strings() {
    let _g = lock();
    assert_parse_string("\"\"", "");
    assert_parse_string(
        "\" !\\\"#$%&'()*+,-./\\/0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_'abcdefghijklmnopqrstuvwxyz{|}~\"",
        " !\"#$%&'()*+,-.//0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_'abcdefghijklmnopqrstuvwxyz{|}~",
    );
    assert_parse_string(
        "\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u20AC\\u732b\"",
        "\"\\/\x08\x0c\n\r\t€猫",
    );
    assert_parse_string("\"\x08\x0c\n\r\t\"", "\x08\x0c\n\r\t");
}

#[test]
fn parse_string_should_parse_utf16_surrogate_pairs() {
    let _g = lock();
    assert_parse_string("\"\\uD83D\\udc31\"", "🐱");
}

#[test]
fn parse_string_should_not_parse_non_strings() {
    let _g = lock();
    assert_not_parse_string("this\" is not a string\"");
    assert_not_parse_string("");
}

#[test]
fn parse_string_should_not_parse_invalid_backslash() {
    let _g = lock();
    assert_not_parse_string("Abcdef\\123");
    assert_not_parse_string("Abcdef\\e23");
}

#[test]
fn parse_string_should_not_overflow_with_closing_backslash() {
    let _g = lock();
    assert_not_parse_string("\"000000000000000000\\");
}

#[test]
fn parse_string_should_parse_bug_94() {
    let _g = lock();
    let string = "\"~!@\\\\#$%^&*()\\\\\\\\-\\\\+{}[]:\\\\;\\\\\\\"\\\\<\\\\>?/.,DC=ad,DC=com\"";
    assert_parse_string(
        string,
        "~!@\\#$%^&*()\\\\-\\+{}[]:\\;\\\"\\<\\>?/.,DC=ad,DC=com",
    );
}

// =====================================================================================================================
// parse_array.c (private `parse_array` -> whole-document parse of an array)
// =====================================================================================================================

fn assert_is_array(i: Item) {
    assert!(!i.is_null(), "Item is NULL.");
    assert_not_in_list(i);
    assert_eq!(kind(i), a::ARRAY, "Item doesn't have expected type.");
    assert_has_no_reference(i);
    assert_has_no_const_string(i);
    assert_has_no_valuestring(i);
    assert_has_no_string(i);
}

fn assert_not_array(json: &str) {
    let item = pj(json);
    assert!(
        item.is_null() || kind(item) != a::ARRAY,
        "{json:?} must not parse as an array"
    );
    a::delete(item);
}

#[test]
fn parse_array_should_parse_empty_arrays() {
    let _g = lock();
    for j in ["[]", "[\n\t]"] {
        let item = pj(j);
        assert_is_array(item);
        assert_has_no_child(item);
        a::delete(item);
    }
}

#[test]
fn parse_array_should_parse_arrays_with_one_element() {
    let _g = lock();
    let item = pj("[1]");
    assert_is_array(item);
    assert_has_child(item);
    assert_eq!(kind(a::child(item)), a::NUMBER);
    a::delete(item);

    let item = pj("[\"hello!\"]");
    assert_is_array(item);
    assert_has_child(item);
    assert_eq!(kind(a::child(item)), a::STRING);
    assert_eq!(vstr(a::child(item)).as_deref(), Some("hello!"));
    a::delete(item);

    let item = pj("[[]]");
    assert_is_array(item);
    assert_has_child(item);
    assert_eq!(kind(a::child(item)), a::ARRAY);
    assert_has_no_child(a::child(item));
    a::delete(item);

    let item = pj("[null]");
    assert_is_array(item);
    assert_has_child(item);
    assert_eq!(kind(a::child(item)), a::NULL_T);
    a::delete(item);
}

#[test]
fn parse_array_should_parse_arrays_with_multiple_elements() {
    let _g = lock();
    let item = pj("[1\t,\n2, 3]");
    assert_is_array(item);
    assert_has_child(item);
    let c = a::child(item);
    assert!(!a::next(c).is_null());
    assert!(!a::next(a::next(c)).is_null());
    assert!(a::next(a::next(a::next(c))).is_null());
    assert_eq!(kind(c), a::NUMBER);
    assert_eq!(kind(a::next(c)), a::NUMBER);
    assert_eq!(kind(a::next(a::next(c))), a::NUMBER);
    a::delete(item);

    let expected_types = [
        a::NUMBER,
        a::NULL_T,
        a::TRUE,
        a::FALSE,
        a::ARRAY,
        a::STRING,
        a::OBJECT,
    ];
    let item = pj("[1, null, true, false, [], \"hello\", {}]");
    assert_is_array(item);
    let mut node = a::child(item);
    let mut i = 0;
    while i < expected_types.len() && !node.is_null() {
        assert_eq!(kind(node), expected_types[i]);
        i += 1;
        node = a::next(node);
    }
    assert_eq!(i, 7);
    a::delete(item);
}

#[test]
fn parse_array_should_not_parse_non_arrays() {
    let _g = lock();
    for j in [
        "",
        "[",
        "]",
        "{\"hello\":[]}",
        "42",
        "3.14",
        "\"[]hello world!\n\"",
    ] {
        assert_not_array(j);
    }
}

// =====================================================================================================================
// parse_object.c (private `parse_object` -> whole-document parse of an object)
// =====================================================================================================================

fn assert_is_object(i: Item) {
    assert!(!i.is_null(), "Item is NULL.");
    assert_not_in_list(i);
    assert_eq!(kind(i), a::OBJECT, "Item doesn't have expected type.");
    assert_has_no_reference(i);
    assert_has_no_const_string(i);
    assert_has_no_valuestring(i);
    assert_has_no_string(i);
}

fn assert_is_child(c: Item, name: &str, ty: i32) {
    assert!(!c.is_null(), "Child item is NULL.");
    assert!(a::key(c).is_some(), "Child item doesn't have a name.");
    assert_eq!(
        kstr(c).as_deref(),
        Some(name),
        "Child item has the wrong name."
    );
    assert_eq!(kind(c), ty);
}

fn assert_not_object(json: &str) {
    let item = pj(json);
    assert!(
        item.is_null() || kind(item) != a::OBJECT,
        "{json:?} must not parse as an object"
    );
    a::delete(item);
}

#[test]
fn parse_object_should_parse_empty_objects() {
    let _g = lock();
    for j in ["{}", "{\n\t}"] {
        let item = pj(j);
        assert_is_object(item);
        assert_has_no_child(item);
        a::delete(item);
    }
}

#[test]
fn parse_object_should_parse_objects_with_one_element() {
    let _g = lock();
    for (j, name, ty) in [
        ("{\"one\":1}", "one", a::NUMBER),
        ("{\"hello\":\"world!\"}", "hello", a::STRING),
        ("{\"array\":[]}", "array", a::ARRAY),
        ("{\"null\":null}", "null", a::NULL_T),
    ] {
        let item = pj(j);
        assert_is_object(item);
        assert_is_child(a::child(item), name, ty);
        a::delete(item);
    }
}

#[test]
fn parse_object_should_parse_objects_with_multiple_elements() {
    let _g = lock();
    let item = pj("{\"one\":1\t,\t\"two\"\n:2, \"three\":3}");
    assert_is_object(item);
    let c = a::child(item);
    assert_is_child(c, "one", a::NUMBER);
    assert_is_child(a::next(c), "two", a::NUMBER);
    assert_is_child(a::next(a::next(c)), "three", a::NUMBER);
    a::delete(item);

    let expected_types = [
        a::NUMBER,
        a::NULL_T,
        a::TRUE,
        a::FALSE,
        a::ARRAY,
        a::STRING,
        a::OBJECT,
    ];
    let expected_names = ["one", "NULL", "TRUE", "FALSE", "array", "world", "object"];
    let item = pj(
        "{\"one\":1, \"NULL\":null, \"TRUE\":true, \"FALSE\":false, \"array\":[], \"world\":\"hello\", \"object\":{}}",
    );
    assert_is_object(item);
    let mut node = a::child(item);
    let mut i = 0;
    while i < expected_types.len() && !node.is_null() {
        assert_is_child(node, expected_names[i], expected_types[i]);
        i += 1;
        node = a::next(node);
    }
    assert_eq!(i, 7);
    a::delete(item);
}

#[test]
fn parse_object_should_not_parse_non_objects() {
    let _g = lock();
    for j in [
        "",
        "{",
        "}",
        "[\"hello\",{}]",
        "42",
        "3.14",
        "\"{}hello world!\n\"",
    ] {
        assert_not_object(j);
    }
}

// =====================================================================================================================
// parse_value.c (private `parse_value` -> whole-document parse)
// =====================================================================================================================

fn assert_parse_value(string: &str, ty: i32) {
    let item = pj(string);
    assert!(!item.is_null(), "Item is NULL.");
    assert_not_in_list(item);
    assert_eq!(kind(item), ty, "Item doesn't have expected type.");
    assert_has_no_reference(item);
    assert_has_no_const_string(item);
    assert_has_no_string(item);
    a::delete(item);
}

#[test]
fn parse_value_should_parse_null() {
    let _g = lock();
    assert_parse_value("null", a::NULL_T);
}
#[test]
fn parse_value_should_parse_true() {
    let _g = lock();
    assert_parse_value("true", a::TRUE);
}
#[test]
fn parse_value_should_parse_false() {
    let _g = lock();
    assert_parse_value("false", a::FALSE);
}
#[test]
fn parse_value_should_parse_number() {
    let _g = lock();
    assert_parse_value("1.5", a::NUMBER);
}
#[test]
fn parse_value_should_parse_string() {
    let _g = lock();
    assert_parse_value("\"\"", a::STRING);
    assert_parse_value("\"hello\"", a::STRING);
}
#[test]
fn parse_value_should_parse_array() {
    let _g = lock();
    assert_parse_value("[]", a::ARRAY);
}
#[test]
fn parse_value_should_parse_object() {
    let _g = lock();
    assert_parse_value("{}", a::OBJECT);
}

// =====================================================================================================================
// print_string.c (private `print_string_ptr` -> printing a string item)
// =====================================================================================================================

fn assert_print_string(expected: &str, input: Option<&[u8]>) {
    // `print_string_ptr(NULL)` is reached through a string reference whose valuestring is NULL.
    let item = match input {
        Some(s) => a::create_string(Some(s)),
        None => a::create_string_reference(None),
    };
    assert!(!item.is_null());
    assert_eq!(
        pu(item).as_deref(),
        Some(expected),
        "The printed string isn't as expected."
    );
    a::delete(item);
}

#[test]
fn print_string_should_print_empty_strings() {
    let _g = lock();
    assert_print_string("\"\"", Some(b""));
    assert_print_string("\"\"", None);
}

#[test]
fn print_string_should_print_ascii() {
    let _g = lock();
    let ascii: Vec<u8> = (1u8..0x7F).collect();
    assert_print_string(
        "\"\\u0001\\u0002\\u0003\\u0004\\u0005\\u0006\\u0007\\b\\t\\n\\u000b\\f\\r\\u000e\\u000f\\u0010\\u0011\\u0012\\u0013\\u0014\\u0015\\u0016\\u0017\\u0018\\u0019\\u001a\\u001b\\u001c\\u001d\\u001e\\u001f !\\\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\"",
        Some(&ascii),
    );
}

#[test]
fn print_string_should_print_utf8() {
    let _g = lock();
    assert_print_string("\"ü猫慕\"", Some("ü猫慕".as_bytes()));
}

// =====================================================================================================================
// print_number.c (private `print_number` -> printing a number item)
// =====================================================================================================================

/// The upstream helper also strips the third exponent digit that old MSVC prints (RULEBOOK D4, D18); the
/// Judge runs on macOS, which prints two, so that loop has nothing to do and is not ported.
fn assert_print_number(expected: &str, input: f64) {
    let item = a::create_number(input);
    assert!(!item.is_null());
    assert_eq!(
        pu(item).as_deref(),
        Some(expected),
        "Printed number is not as expected."
    );
    a::delete(item);
}

#[test]
fn print_number_should_print_zero() {
    let _g = lock();
    assert_print_number("0", 0.0);
}

#[test]
fn print_number_should_print_negative_integers() {
    let _g = lock();
    assert_print_number("-1", -1.0);
    assert_print_number("-32768", -32768.0);
    assert_print_number("-2147483648", -2147483648.0);
}

#[test]
fn print_number_should_print_positive_integers() {
    let _g = lock();
    assert_print_number("1", 1.0);
    assert_print_number("32767", 32767.0);
    assert_print_number("2147483647", 2147483647.0);
}

#[test]
fn print_number_should_print_positive_reals() {
    let _g = lock();
    assert_print_number("0.123", 0.123);
    assert_print_number("1e-09", 10e-10);
    assert_print_number("1000000000000", 10e11);
    assert_print_number("1.23e+129", 123e+127);
    assert_print_number("1.23e-126", 123e-128);
    assert_print_number("3.1415926535897931", 3.1415926535897931);
}

#[test]
fn print_number_should_print_negative_reals() {
    let _g = lock();
    assert_print_number("-0.0123", -0.0123);
    assert_print_number("-1e-09", -10e-10);
    assert_print_number("-1e+21", -10e20);
    assert_print_number("-1.23e+129", -123e+127);
    assert_print_number("-1.23e-126", -123e-128);
}

#[test]
#[ignore = "upstream TEST_IGNORE() (print_number.c:105): NaN and Inf cannot be tested easily in C89"]
fn print_number_should_print_non_number() {}

// =====================================================================================================================
// print_array.c / print_object.c / print_value.c
// =====================================================================================================================

fn assert_print_array(expected: &str, input: &str) {
    let item = pj(input);
    assert!(!item.is_null(), "Failed to parse array.");
    assert_eq!(kind(item), a::ARRAY);
    assert_eq!(
        pu(item).as_deref(),
        Some(input),
        "Unformatted array is not correct."
    );
    assert_eq!(
        pf(item).as_deref(),
        Some(expected),
        "Formatted array is not correct."
    );
    a::delete(item);
}

#[test]
fn print_array_should_print_empty_arrays() {
    let _g = lock();
    assert_print_array("[]", "[]");
}

#[test]
fn print_array_should_print_arrays_with_one_element() {
    let _g = lock();
    assert_print_array("[1]", "[1]");
    assert_print_array("[\"hello!\"]", "[\"hello!\"]");
    assert_print_array("[[]]", "[[]]");
    assert_print_array("[null]", "[null]");
}

#[test]
fn print_array_should_print_arrays_with_multiple_elements() {
    let _g = lock();
    assert_print_array("[1, 2, 3]", "[1,2,3]");
    assert_print_array(
        "[1, null, true, false, [], \"hello\", {\n\t}]",
        "[1,null,true,false,[],\"hello\",{}]",
    );
}

fn assert_print_object(expected: &str, input: &str) {
    let item = pj(input);
    assert!(!item.is_null(), "Failed to parse object.");
    assert_eq!(kind(item), a::OBJECT);
    assert_eq!(
        pu(item).as_deref(),
        Some(input),
        "Unformatted object is not correct."
    );
    assert_eq!(
        pf(item).as_deref(),
        Some(expected),
        "Formatted object is not correct."
    );
    a::delete(item);
}

#[test]
fn print_object_should_print_empty_objects() {
    let _g = lock();
    assert_print_object("{\n}", "{}");
}

#[test]
fn print_object_should_print_objects_with_one_element() {
    let _g = lock();
    assert_print_object("{\n\t\"one\":\t1\n}", "{\"one\":1}");
    assert_print_object("{\n\t\"hello\":\t\"world!\"\n}", "{\"hello\":\"world!\"}");
    assert_print_object("{\n\t\"array\":\t[]\n}", "{\"array\":[]}");
    assert_print_object("{\n\t\"null\":\tnull\n}", "{\"null\":null}");
}

#[test]
fn print_object_should_print_objects_with_multiple_elements() {
    let _g = lock();
    assert_print_object(
        "{\n\t\"one\":\t1,\n\t\"two\":\t2,\n\t\"three\":\t3\n}",
        "{\"one\":1,\"two\":2,\"three\":3}",
    );
    assert_print_object(
        "{\n\t\"one\":\t1,\n\t\"NULL\":\tnull,\n\t\"TRUE\":\ttrue,\n\t\"FALSE\":\tfalse,\n\t\"array\":\t[],\n\t\"world\":\t\"hello\",\n\t\"object\":\t{\n\t}\n}",
        "{\"one\":1,\"NULL\":null,\"TRUE\":true,\"FALSE\":false,\"array\":[],\"world\":\"hello\",\"object\":{}}",
    );
}

fn assert_print_value(input: &str) {
    let item = pj(input);
    assert!(!item.is_null(), "Failed to parse value.");
    assert_eq!(
        pu(item).as_deref(),
        Some(input),
        "Printed value is not as expected."
    );
    a::delete(item);
}

#[test]
fn print_value_should_print_null() {
    let _g = lock();
    assert_print_value("null");
}
#[test]
fn print_value_should_print_true() {
    let _g = lock();
    assert_print_value("true");
}
#[test]
fn print_value_should_print_false() {
    let _g = lock();
    assert_print_value("false");
}
#[test]
fn print_value_should_print_number() {
    let _g = lock();
    assert_print_value("1.5");
}
#[test]
fn print_value_should_print_string() {
    let _g = lock();
    assert_print_value("\"\"");
    assert_print_value("\"hello\"");
}
#[test]
fn print_value_should_print_array() {
    let _g = lock();
    assert_print_value("[]");
}
#[test]
fn print_value_should_print_object() {
    let _g = lock();
    assert_print_value("{}");
}

// =====================================================================================================================
// misc_tests.c
// =====================================================================================================================

/// `cJSON_ArrayForEach` over a real array (the C test builds the list on the stack by hand).
#[test]
fn cjson_array_foreach_should_loop_over_arrays() {
    let _g = lock();
    let array = pj("[0,1,2,3,4,5,6,7,8,9]");
    let elements = a::children(array);
    assert_eq!(elements.len(), 10);
    for (i, e) in elements.iter().enumerate() {
        assert!(
            *e == a::array_item(array, i as i32),
            "Not iterating over array properly"
        );
    }
    a::delete(array);
}

#[test]
fn cjson_array_foreach_should_not_dereference_null_pointer() {
    let _g = lock();
    assert!(a::children(NULL).is_empty());
}

#[test]
fn cjson_get_object_item_should_get_object_items() {
    let _g = lock();
    let item = pj("{\"one\":1, \"Two\":2, \"tHree\":3}");
    assert!(
        a::object_item(NULL, Some(b"test")).is_null(),
        "Failed to fail on NULL pointer."
    );
    assert!(
        a::object_item(item, None).is_null(),
        "Failed to fail on NULL string."
    );
    let found = a::object_item(item, Some(b"one"));
    assert!(!found.is_null(), "Failed to find first item.");
    assert_double(a::value_double(found), 1.0);
    let found = a::object_item(item, Some(b"tWo"));
    assert!(!found.is_null(), "Failed to find first item.");
    assert_double(a::value_double(found), 2.0);
    let found = a::object_item(item, Some(b"three"));
    assert!(!found.is_null(), "Failed to find item.");
    assert_double(a::value_double(found), 3.0);
    assert!(
        a::object_item(item, Some(b"four")).is_null(),
        "Should not find something that isn't there."
    );
    a::delete(item);
}

#[test]
fn cjson_get_object_item_case_sensitive_should_get_object_items() {
    let _g = lock();
    let item = pj("{\"one\":1, \"Two\":2, \"tHree\":3}");
    assert!(
        a::object_item_cs(NULL, Some(b"test")).is_null(),
        "Failed to fail on NULL pointer."
    );
    assert!(
        a::object_item_cs(item, None).is_null(),
        "Failed to fail on NULL string."
    );
    let found = a::object_item_cs(item, Some(b"one"));
    assert!(!found.is_null(), "Failed to find first item.");
    assert_double(a::value_double(found), 1.0);
    let found = a::object_item_cs(item, Some(b"Two"));
    assert!(!found.is_null(), "Failed to find first item.");
    assert_double(a::value_double(found), 2.0);
    let found = a::object_item_cs(item, Some(b"tHree"));
    assert!(!found.is_null(), "Failed to find item.");
    assert_double(a::value_double(found), 3.0);
    assert!(
        a::object_item_cs(item, Some(b"One")).is_null(),
        "Should not find something that isn't there."
    );
    a::delete(item);
}

#[test]
fn cjson_get_object_item_should_not_crash_with_array() {
    let _g = lock();
    let array = pj("[1]");
    assert!(a::object_item(array, Some(b"name")).is_null());
    a::delete(array);
}

#[test]
fn cjson_get_object_item_case_sensitive_should_not_crash_with_array() {
    let _g = lock();
    let array = pj("[1]");
    assert!(a::object_item_cs(array, Some(b"name")).is_null());
    a::delete(array);
}

/// The C test builds stack nodes with `type = <kind> | cJSON_StringIsConst`. Here every item gets that flag by
/// being added with `cJSON_AddItemToObjectCS` (cJSON.c:2095); the `invalid` node is a root after a `remove`
/// patch at `""` (type 0, [`make_invalid`]), so it ends with type `Invalid | StringIsConst` as in C.
#[test]
fn typecheck_functions_should_check_type() {
    let _g = lock();
    let holder = a::create_object();
    let with_const = |item: Item| {
        assert!(a::add_item_to_object_cs(holder, Some(c"k"), item));
        assert_eq!(a::type_flags(item) & a::STRING_IS_CONST, a::STRING_IS_CONST);
        item
    };
    let invalid = make_invalid();
    assert!(a::add_item_to_object_cs(holder, Some(c"invalid"), invalid));
    assert_eq!(a::type_flags(invalid), a::INVALID | a::STRING_IS_CONST);
    let not_invalid = |f: fn(Item) -> bool| assert!(!f(invalid));

    assert!(!a::is_invalid(NULL));
    let item = with_const(a::create_false());
    assert!(!a::is_invalid(item));
    assert!(a::is_invalid(invalid));
    let _ = a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_false());
    assert!(!a::is_false(NULL));
    not_invalid(a::is_false);
    assert!(a::is_false(item));
    assert!(a::is_bool(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_true());
    assert!(!a::is_true(NULL));
    not_invalid(a::is_true);
    assert!(a::is_true(item));
    assert!(a::is_bool(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_null());
    assert!(!a::is_null(NULL));
    not_invalid(a::is_null);
    assert!(a::is_null(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_number(0.0));
    assert!(!a::is_number(NULL));
    not_invalid(a::is_number);
    assert!(a::is_number(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_string(Some(b"s")));
    assert!(!a::is_string(NULL));
    not_invalid(a::is_string);
    assert!(a::is_string(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_array());
    assert!(!a::is_array(NULL));
    not_invalid(a::is_array);
    assert!(a::is_array(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_object());
    assert!(!a::is_object(NULL));
    not_invalid(a::is_object);
    assert!(a::is_object(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    let item = with_const(a::create_raw(Some(b"r")));
    assert!(!a::is_raw(NULL));
    not_invalid(a::is_raw);
    assert!(a::is_raw(item));
    a::delete_item_from_object_cs(holder, Some(b"k"));

    a::delete(holder); // also deletes `invalid`, its child
}

#[test]
fn cjson_should_not_parse_to_deeply_nested_jsons() {
    let _g = lock();
    on_big_stack(|| {
        let deep_json = vec![b'['; a::NESTING_LIMIT];
        let r = a::parse(Some(&deep_json));
        assert!(r.item.is_null(), "To deep JSONs should not be parsed.");
    });
}

/// Runs as written in both Stages: node handles can form the cycle (RULEBOOK D11, revised 2026-10-10).
#[test]
fn cjson_should_not_follow_too_deep_circular_references() {
    let _g = lock();
    on_big_stack(|| {
        let o = a::create_array();
        let aa = a::create_array();
        let b = a::create_array();
        a::add_item_to_array(o, aa);
        a::add_item_to_array(aa, b);
        a::add_item_to_array(b, o);
        let x = a::duplicate(o, true);
        assert!(x.is_null());
        a::detach_item_from_array(b, 0);
        a::delete(o);
    });
}

/// Not an upstream test: the RULEBOOK D11 recommendation, the same duplicate limit without a cycle. An
/// acyclic array nested `CIRCULAR_LIMIT + 1` deep must not be duplicated.
#[test]
fn extra_duplicate_should_stop_at_the_circular_limit_without_a_cycle() {
    let _g = lock();
    on_big_stack(|| {
        let root = a::create_array();
        let mut cur = root;
        for _ in 0..a::CIRCULAR_LIMIT + 1 {
            let next = a::create_array();
            assert!(a::add_item_to_array(cur, next));
            cur = next;
        }
        assert!(a::duplicate(root, true).is_null());
        a::delete(root);
    });
}

/// Not an upstream test: the RULEBOOK D10 duty. A release build must reach the C limits on a 2 MiB thread (a
/// spawned Rust thread's default; C gets them on its 8 MiB main stack): parse and print a document nested
/// `NESTING_LIMIT` deep, duplicate an array nested `CIRCULAR_LIMIT` deep, delete both. `run.py --stage 2`
/// runs it with `cargo test --release`; debug builds ignore it (their frames are several times larger).
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "release only: run.py --stage 2 runs it with --release (RULEBOOK D10)"
)]
fn extra_release_build_reaches_the_limits_on_a_2_mib_stack() {
    let _g = lock();
    std::thread::scope(|s| {
        let t = std::thread::Builder::new()
            .stack_size(2 << 20)
            .spawn_scoped(s, || {
                let mut text = vec![b'['; a::NESTING_LIMIT];
                text.extend(vec![b']'; a::NESTING_LIMIT]);
                let doc = a::parse(Some(&text)).item;
                assert!(
                    !doc.is_null(),
                    "a document nested NESTING_LIMIT deep must parse"
                );
                assert_eq!(
                    a::print_unformatted(doc).map(|p| p.bytes().to_vec()),
                    Some(text.clone())
                );
                assert!(a::print(doc).is_some());
                a::delete(doc);
                let mut deeper = vec![b'['; a::NESTING_LIMIT + 1];
                deeper.extend(vec![b']'; a::NESTING_LIMIT + 1]);
                assert!(
                    a::parse(Some(&deeper)).item.is_null(),
                    "one level more must not parse"
                );

                let root = a::create_array();
                let mut cur = root;
                for _ in 1..a::CIRCULAR_LIMIT {
                    let next = a::create_array();
                    assert!(a::add_item_to_array(cur, next));
                    cur = next;
                }
                let copy = a::duplicate(root, true);
                assert!(
                    !copy.is_null(),
                    "an array nested CIRCULAR_LIMIT deep must be duplicated"
                );
                a::delete(copy);
                a::delete(root);
            });
        t.unwrap().join().unwrap();
    });
}

#[test]
fn cjson_set_number_value_should_set_numbers() {
    let _g = lock();
    let number = a::create_number(0.0);
    a::set_number_value(number, 1.5);
    assert_eq!(1, a::value_int(number));
    assert_double(1.5, a::value_double(number));

    a::set_number_value(number, -1.5);
    assert_eq!(-1, a::value_int(number));
    assert_double(-1.5, a::value_double(number));

    a::set_number_value(number, 1.0 + i32::MAX as f64);
    assert_eq!(i32::MAX, a::value_int(number));
    assert_double(1.0 + i32::MAX as f64, a::value_double(number));

    a::set_number_value(number, -1.0 + i32::MIN as f64);
    assert_eq!(i32::MIN, a::value_int(number));
    assert_double(-1.0 + i32::MIN as f64, a::value_double(number));
    a::delete(number);
}

/// The C test links four stack nodes by hand; here they are the children of a real array.
#[test]
fn cjson_detach_item_via_pointer_should_detach_items() {
    let _g = lock();
    let parent = a::create_array();
    let list: Vec<Item> = (0..4).map(|_| a::create_null()).collect();
    for l in &list {
        assert!(a::add_item_to_array(parent, *l));
    }

    // detach in the middle (list[1])
    assert!(
        a::detach_item_via_pointer(parent, list[1]) == list[1],
        "Failed to detach in the middle."
    );
    assert!(
        a::prev(list[1]).is_null() && a::next(list[1]).is_null(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(a::next(list[0]) == list[2] && a::prev(list[2]) == list[0]);

    // detach beginning (list[0])
    assert!(
        a::detach_item_via_pointer(parent, list[0]) == list[0],
        "Failed to detach beginning."
    );
    assert!(
        a::prev(list[0]).is_null() && a::next(list[0]).is_null(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        a::prev(list[2]) == list[3] && a::child(parent) == list[2],
        "Didn't set the new beginning."
    );

    // detach end (list[3])
    assert!(
        a::detach_item_via_pointer(parent, list[3]) == list[3],
        "Failed to detach end."
    );
    assert!(
        a::prev(list[3]).is_null() && a::next(list[3]).is_null(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        a::next(list[2]).is_null() && a::child(parent) == list[2],
        "Didn't set the new end"
    );

    // detach single item (list[2])
    assert!(
        a::detach_item_via_pointer(parent, list[2]) == list[2],
        "Failed to detach single item."
    );
    assert!(
        a::prev(list[2]).is_null() && a::next(list[2]).is_null(),
        "Didn't set pointers of detached item to NULL."
    );
    assert!(
        a::child(parent).is_null(),
        "Child of the parent wasn't set to NULL."
    );

    for l in list {
        a::delete(l);
    }
    a::delete(parent);
}

/// Not an upstream test: RULEBOOK D14 (narrowed). A reference made by `cJSON_CreateArrayReference(x)` has `x` as
/// its first child; when `x` is in no list its `prev` is NULL, so `cJSON_AddItemToArray` and
/// `cJSON_InsertItemInArray` return true without appending (cJSON.c:2050-2055; measured on the original C by
/// `experiments/e6_cjson_reference_add_probe.c`). Stage 2 must keep that branch.
#[test]
fn extra_add_to_a_reference_of_a_node_in_no_list_succeeds_without_appending() {
    let _g = lock();
    let x = a::create_null();
    let y = a::create_true();
    let z = a::create_false();
    let r = a::create_array_reference(x);
    assert!(a::add_item_to_array(r, y));
    assert!(a::insert_item_in_array(r, 5, z));
    assert_eq!(a::array_size(r), 1);
    assert_eq!(pu(r).as_deref(), Some("[null]"));
    assert!(a::prev(y).is_null() && a::prev(z).is_null() && a::next(x).is_null());
    a::delete(r);
    a::delete(x);
    a::delete(y);
    a::delete(z);
}

/// The C test zeroes `prev` by hand (D14, `C_LAYOUT` only). The guard it tests (cJSON.c:2260) is also
/// reachable publicly: a node in no list has `prev` NULL (cJSON.c:243-246), so detaching it from an array
/// whose first child it is not returns NULL and leaves the array as it was. That variant runs always.
#[test]
fn cjson_detach_item_via_pointer_should_return_null_if_item_prev_is_null() {
    let _g = lock();
    let parent = a::create_array();
    let l0 = a::create_null();
    let stray = a::create_null();
    assert!(a::add_item_to_array(parent, l0));
    assert!(a::prev(stray).is_null());
    assert!(
        a::detach_item_via_pointer(parent, stray).is_null(),
        "a node in no list must not be detached"
    );
    assert!(a::child(parent) == l0 && a::prev(l0) == l0 && a::next(l0).is_null());
    a::delete(stray);
    a::delete(parent);

    if !a::C_LAYOUT {
        skipped("corrupted list: prev set to NULL by hand (RULEBOOK D14)");
        return;
    }
    let parent = a::create_array();
    let l0 = a::create_null();
    let l1 = a::create_null();
    a::add_item_to_array(parent, l0);
    a::add_item_to_array(parent, l1);
    a::raw_set_prev(l0, NULL);
    a::raw_set_prev(l1, NULL);
    assert!(
        a::detach_item_via_pointer(parent, l1).is_null(),
        "Failed to detach in the middle."
    );
    assert!(
        a::detach_item_via_pointer(parent, l0) == l0,
        "Failed to detach in the middle."
    );
    a::delete(l0);
    a::delete(parent);
}

/// The C test uses stack nodes as replacements; here they are heap items.
#[test]
fn cjson_replace_item_via_pointer_should_replace_items() {
    let _g = lock();
    let beginning = a::create_null();
    let middle = a::create_null();
    let end = a::create_null();
    assert!(!beginning.is_null() && !middle.is_null() && !end.is_null());
    let array = a::create_array();
    assert!(!array.is_null());
    a::add_item_to_array(array, beginning);
    a::add_item_to_array(array, middle);
    a::add_item_to_array(array, end);
    let r0 = a::create_null();
    let r1 = a::create_null();
    let r2 = a::create_null();

    // replace beginning
    assert!(a::replace_item_via_pointer(array, beginning, r0));
    assert!(a::prev(r0) == end);
    assert!(a::next(r0) == middle);
    assert!(a::prev(middle) == r0);
    assert!(a::child(array) == r0);

    // replace middle
    assert!(a::replace_item_via_pointer(array, middle, r1));
    assert!(a::prev(r1) == r0);
    assert!(a::next(r1) == end);
    assert!(a::prev(end) == r1);

    // replace end
    assert!(a::replace_item_via_pointer(array, end, r2));
    assert!(a::prev(r2) == r1);
    assert!(a::next(r2).is_null());
    assert!(a::next(r1) == r2);

    a::delete(array);
}

#[test]
fn cjson_replace_item_in_object_should_preserve_name() {
    let _g = lock();
    let root = a::create_object();
    let child = a::create_number(1.0);
    assert!(!child.is_null());
    let replacement = a::create_number(2.0);
    assert!(!replacement.is_null());

    assert!(
        a::add_item_to_object(root, Some(b"child"), child),
        "add item to object failed"
    );
    a::replace_item_in_object(root, Some(b"child"), replacement);

    assert!(a::child(root) == replacement);
    assert_eq!(kstr(replacement).as_deref(), Some("child"));
    a::delete(root);
}

#[test]
fn cjson_functions_should_not_crash_with_null_pointers() {
    let _g = lock();
    let mut buffer = [0u8; 10];
    let item = a::create_string(Some(b"item"));
    let array = a::create_array();
    let item1 = a::create_string(Some(b"item1"));
    let item2 = a::create_string(Some(b"corrupted array item3"));
    let corrupted_string = a::create_string(Some(b"corrupted"));

    a::add_item_to_array(array, item1);
    a::add_item_to_array(array, item2);

    let original_prev = a::prev(item2);
    if a::C_LAYOUT {
        a::raw_set_prev(item2, NULL);
        a::raw_free_valuestring(corrupted_string);
    } else {
        skipped("corrupted array item and string (RULEBOOK D14)");
    }

    a::hooks_off();
    assert!(a::parse(None).item.is_null());
    assert!(a::parse_with_opts(None, true, true).item.is_null());
    assert!(a::print(NULL).is_none());
    assert!(a::print_unformatted(NULL).is_none());
    assert!(a::print_buffered(NULL, 10, true).is_none());
    assert!(!a::print_preallocated(NULL, Some(&mut buffer), 10, true));
    assert!(!a::print_preallocated(item, None, 1, true));
    a::delete(NULL);
    a::array_size(NULL);
    assert!(a::array_item(NULL, 0).is_null());
    assert!(a::object_item(NULL, Some(b"item")).is_null());
    assert!(a::object_item(item, None).is_null());
    assert!(a::object_item_cs(NULL, Some(b"item")).is_null());
    assert!(a::object_item_cs(item, None).is_null());
    assert!(!a::has_object_item(NULL, Some(b"item")));
    assert!(!a::has_object_item(item, None));
    assert!(!a::is_invalid(NULL));
    assert!(!a::is_false(NULL));
    assert!(!a::is_true(NULL));
    assert!(!a::is_bool(NULL));
    assert!(!a::is_null(NULL));
    assert!(!a::is_number(NULL));
    assert!(!a::is_string(NULL));
    assert!(!a::is_array(NULL));
    assert!(!a::is_object(NULL));
    assert!(!a::is_raw(NULL));
    assert!(a::create_string(None).is_null());
    assert!(a::create_raw(None).is_null());
    assert!(a::create_int_array(None, 10).is_null());
    assert!(a::create_float_array(None, 10).is_null());
    assert!(a::create_double_array(None, 10).is_null());
    assert!(a::create_string_array(None, 10).is_null());
    a::add_item_to_array(NULL, item);
    a::add_item_to_array(item, NULL);
    a::add_item_to_object(item, Some(b"item"), NULL);
    a::add_item_to_object(item, None, item);
    a::add_item_to_object(NULL, Some(b"item"), item);
    a::add_item_to_object_cs(item, Some(c"item"), NULL);
    a::add_item_to_object_cs(item, None, item);
    a::add_item_to_object_cs(NULL, Some(c"item"), item);
    a::add_item_reference_to_array(NULL, item);
    a::add_item_reference_to_array(item, NULL);
    a::add_item_reference_to_object(item, Some(b"item"), NULL);
    a::add_item_reference_to_object(item, None, item);
    a::add_item_reference_to_object(NULL, Some(b"item"), item);
    assert!(a::detach_item_via_pointer(NULL, item).is_null());
    assert!(a::detach_item_via_pointer(item, NULL).is_null());
    assert!(a::detach_item_from_array(NULL, 0).is_null());
    a::delete_item_from_array(NULL, 0);
    assert!(a::detach_item_from_object(NULL, Some(b"item")).is_null());
    assert!(a::detach_item_from_object(item, None).is_null());
    assert!(a::detach_item_from_object_cs(NULL, Some(b"item")).is_null());
    assert!(a::detach_item_from_object_cs(item, None).is_null());
    a::delete_item_from_object(NULL, Some(b"item"));
    a::delete_item_from_object(item, None);
    a::delete_item_from_object_cs(NULL, Some(b"item"));
    a::delete_item_from_object_cs(item, None);
    assert!(!a::insert_item_in_array(array, 0, NULL));
    if a::C_LAYOUT {
        // with the corrupted list (item2.prev == NULL) inserting at index 1 fails; on a sound list it succeeds
        assert!(!a::insert_item_in_array(array, 1, item));
    }
    assert!(!a::insert_item_in_array(NULL, 0, item));
    assert!(!a::insert_item_in_array(item, 0, NULL));
    assert!(!a::replace_item_via_pointer(NULL, item, item));
    assert!(!a::replace_item_via_pointer(item, NULL, item));
    assert!(!a::replace_item_via_pointer(item, item, NULL));
    assert!(!a::replace_item_in_array(item, 0, NULL));
    assert!(!a::replace_item_in_array(NULL, 0, item));
    assert!(!a::replace_item_in_object(NULL, Some(b"item"), item));
    assert!(!a::replace_item_in_object(item, None, item));
    assert!(!a::replace_item_in_object(item, Some(b"item"), NULL));
    assert!(!a::replace_item_in_object_cs(NULL, Some(b"item"), item));
    assert!(!a::replace_item_in_object_cs(item, None, item));
    assert!(!a::replace_item_in_object_cs(item, Some(b"item"), NULL));
    assert!(a::duplicate(NULL, true).is_null());
    assert!(!a::compare(item, NULL, false));
    assert!(!a::compare(NULL, item, false));
    assert!(a::set_valuestring(NULL, Some(b"test")).is_none());
    if a::C_LAYOUT {
        // valuestring was freed and set to NULL by hand
        assert!(a::set_valuestring(corrupted_string, Some(b"test")).is_none());
    }
    assert!(a::set_valuestring(item, None).is_none());
    assert!(a::minify(None).is_none());
    // cJSON_SetNumberHelper should handle NULL gracefully
    assert!(a::set_number_helper(NULL, 0.0).is_nan());

    // restore corrupted item2 to delete it
    if a::C_LAYOUT {
        a::raw_set_prev(item2, original_prev);
    }
    a::delete(corrupted_string);
    a::delete(array);
    a::delete(item);
}

/// D7: the new value points into the old buffer; needs the C layout (pointer arithmetic on the valuestring).
#[test]
fn cjson_set_valuestring_should_return_null_if_strings_overlap() {
    let _g = lock();
    if !a::C_LAYOUT {
        skipped("overlapping valuestring (RULEBOOK D7)");
        return;
    }
    let obj = pj("\"foo0z\"");
    let str1 = a::set_valuestring(obj, Some(b"abcde"));
    assert_eq!(str1.as_deref(), Some(b"abcde".as_slice()));
    // The string passed to strcpy overlaps, which is not allowed.
    let str2 = a::raw_set_valuestring_from_own(obj, 1);
    // If it overlaps, the string will be messed up.
    assert_eq!(vstr(obj).as_deref(), Some("abcde"));
    assert!(str2.is_none());
    a::delete(obj);
}

/// `ensure` is private and the public `cJSON_Hooks` has no `realloc`: with custom hooks C grows a print buffer by
/// allocating the new size, copying and freeing the old one (cJSON.c:233-237, 545-565). The same failure through
/// the public API (RULEBOOK D1, revised): `PrintBuffered` with a 10-byte prebuffer (allocation 1) that must grow,
/// and the growth allocation (number 2) fails. `ensure` frees the old buffer and fails, so the print returns NULL.
#[test]
fn ensure_should_fail_on_failed_realloc() {
    let _g = lock();
    let item = a::create_string(Some(&[b'a'; 200]));
    a::hooks_on(2);
    let printed = a::print_buffered(item, 10, false);
    let st = a::hook_stats();
    a::hooks_off();
    assert!(
        printed.is_none(),
        "Ensure didn't fail with failing realloc."
    );
    assert_eq!(
        (st.allocs, st.frees, st.bytes),
        (2, 1, 10),
        "growth = allocate(new), free(old) on failure"
    );
    a::delete(item);
}

/// `skip_utf8_bom` is private; the BOM is observable as the parse end of a document that starts with one.
#[test]
fn skip_utf8_bom_should_skip_bom() {
    let _g = lock();
    let r = a::parse_with_opts(Some(b"\xEF\xBB\xBF{}"), false, true);
    assert!(!r.item.is_null());
    assert!(a::is_object(r.item));
    assert_eq!(
        r.end,
        Some(5),
        "BOM (3 bytes) and the object (2 bytes) are consumed"
    );
    a::delete(r.item);
}

#[test]
fn skip_utf8_bom_should_not_skip_bom_if_not_at_beginning() {
    let _g = lock();
    let r = a::parse(Some(b" \xEF\xBB\xBF{}"));
    assert!(r.item.is_null());
}

#[test]
fn cjson_get_string_value_should_get_a_string() {
    let _g = lock();
    let string = a::create_string(Some(b"test"));
    let number = a::create_number(1.0);
    assert_eq!(a::get_string_value(string), a::value_string(string));
    assert!(a::get_string_value(number).is_none());
    assert!(a::get_string_value(NULL).is_none());
    a::delete(number);
    a::delete(string);
}

#[test]
fn cjson_get_number_value_should_get_a_number() {
    let _g = lock();
    let string = a::create_string(Some(b"test"));
    let number = a::create_number(1.0);
    assert_double(a::get_number_value(number), a::value_double(number));
    assert!(a::get_number_value(string).is_nan());
    assert!(a::get_number_value(NULL).is_nan());
    a::delete(number);
    a::delete(string);
}

#[test]
fn cjson_create_string_reference_should_create_a_string_reference() {
    let _g = lock();
    const S: &std::ffi::CStr = c"I am a string!";
    let r = a::create_string_reference(Some(S));
    assert_eq!(a::value_string(r).as_deref(), Some(S.to_bytes()));
    assert_eq!(
        a::value_string_addr(r),
        S.as_ptr() as usize,
        "the reference must not copy"
    );
    assert_eq!(a::IS_REFERENCE | a::STRING, a::type_flags(r));
    a::delete(r);
}

#[test]
fn cjson_create_object_reference_should_create_an_object_reference() {
    let _g = lock();
    let number_object = a::create_object();
    let number = a::create_number(42.0);
    assert!(a::is_number(number));
    assert!(a::is_object(number_object));
    a::add_item_to_object_cs(number_object, Some(c"number"), number);

    let number_reference = a::create_object_reference(number);
    assert!(a::child(number_reference) == number);
    assert_eq!(a::OBJECT | a::IS_REFERENCE, a::type_flags(number_reference));

    a::delete(number_object);
    a::delete(number_reference);
}

#[test]
fn cjson_create_array_reference_should_create_an_array_reference() {
    let _g = lock();
    let number_array = a::create_array();
    let number = a::create_number(42.0);
    assert!(a::is_number(number));
    assert!(a::is_array(number_array));
    a::add_item_to_array(number_array, number);

    let number_reference = a::create_array_reference(number);
    assert!(a::child(number_reference) == number);
    assert_eq!(a::ARRAY | a::IS_REFERENCE, a::type_flags(number_reference));

    a::delete(number_array);
    a::delete(number_reference);
}

#[test]
fn cjson_add_item_to_object_or_array_should_not_add_itself() {
    let _g = lock();
    let object = a::create_object();
    let array = a::create_array();
    assert!(
        !a::add_item_to_object(object, Some(b"key"), object),
        "add an object to itself should fail"
    );
    assert!(
        !a::add_item_to_array(array, array),
        "add an array to itself should fail"
    );
    a::delete(object);
    a::delete(array);
}

/// The key argument is the item's own `string`; writing that member needs the C layout (RULEBOOK D7).
#[test]
fn cjson_add_item_to_object_should_not_use_after_free_when_string_is_aliased() {
    let _g = lock();
    if !a::C_LAYOUT {
        skipped("aliased key string: item->string passed as the key (RULEBOOK D7)");
        return;
    }
    let object = a::create_object();
    let number = a::create_number(42.0);
    assert!(!object.is_null());
    assert!(!number.is_null());
    // The following should not have a use after free that would show up in valgrind or with AddressSanitizer.
    a::raw_add_item_with_aliased_key(object, number, b"number");
    a::delete(object);
}

#[test]
fn cjson_delete_item_from_array_should_not_broken_list_structure() {
    let _g = lock();
    let expected_json1 = "{\"rd\":[{\"a\":\"123\"}]}";
    let expected_json2 = "{\"rd\":[{\"a\":\"123\"},{\"b\":\"456\"}]}";
    let expected_json3 = "{\"rd\":[{\"b\":\"456\"}]}";

    let root = pj("{}");
    let array = a::add_array_to_object(root, Some(b"rd"));
    let item1 = pj("{\"a\":\"123\"}");
    let item2 = pj("{\"b\":\"456\"}");

    a::add_item_to_array(array, item1);
    assert_eq!(pu(root).as_deref(), Some(expected_json1));
    a::add_item_to_array(array, item2);
    assert_eq!(pu(root).as_deref(), Some(expected_json2));
    // this should not broken list structure
    a::delete_item_from_array(array, 0);
    assert_eq!(pu(root).as_deref(), Some(expected_json3));
    a::delete(root);
}

#[test]
fn cjson_set_valuestring_to_object_should_not_leak_memory() {
    let _g = lock();
    let root = pj("{}");
    let stringvalue = "valuestring could be changed safely";
    let reference_valuestring = c"reference item should be freed by yourself";
    let short_valuestring = "shorter valuestring";
    let long_valuestring =
        "new valuestring which much longer than previous should be changed safely";
    let item1 = a::create_string(Some(stringvalue.as_bytes()));
    let item2 = a::create_string_reference(Some(reference_valuestring));

    a::add_item_to_object(root, Some(b"one"), item1);
    a::add_item_to_object(root, Some(b"two"), item2);

    let ptr1 = a::value_string_addr(item1);
    let rv = a::set_valuestring(
        a::object_item(root, Some(b"one")),
        Some(short_valuestring.as_bytes()),
    );
    assert!(rv.is_some());
    assert_eq!(
        ptr1,
        a::value_string_addr(item1),
        "new valuestring shorter than old should not reallocate memory"
    );
    assert_eq!(
        vstr(a::object_item(root, Some(b"one"))).as_deref(),
        Some(short_valuestring)
    );

    // we needn't to free the original valuestring manually
    let ptr1 = a::value_string_addr(item1);
    let rv = a::set_valuestring(
        a::object_item(root, Some(b"one")),
        Some(long_valuestring.as_bytes()),
    );
    assert!(rv.is_some());
    assert_ne!(
        ptr1,
        a::value_string_addr(item1),
        "new valuestring longer than old should reallocate memory"
    );
    assert_eq!(
        vstr(a::object_item(root, Some(b"one"))).as_deref(),
        Some(long_valuestring)
    );

    let rv = a::set_valuestring(
        a::object_item(root, Some(b"two")),
        Some(long_valuestring.as_bytes()),
    );
    assert!(
        rv.is_none(),
        "valuestring of reference object should not be changed"
    );
    assert_eq!(
        vstr(a::object_item(root, Some(b"two"))).as_deref(),
        Some(reference_valuestring.to_str().unwrap())
    );
    a::delete(root);
}

#[test]
fn cjson_set_bool_value_must_not_break_objects() {
    let _g = lock();
    assert_eq!(a::set_bool_value(NULL, true), a::INVALID);

    let bobj = a::create_false();
    assert!(a::is_false(bobj));
    assert_eq!(a::set_bool_value(bobj, true), a::TRUE);
    assert!(a::is_true(bobj));
    a::set_bool_value(bobj, true);
    assert!(a::is_true(bobj));
    assert_eq!(a::set_bool_value(bobj, false), a::FALSE);
    assert!(a::is_false(bobj));
    a::set_bool_value(bobj, false);
    assert!(a::is_false(bobj));

    let sobj = a::create_string(Some(b"test"));
    assert!(a::is_string(sobj));
    a::set_bool_value(sobj, true);
    assert!(a::is_string(sobj));
    a::set_bool_value(sobj, false);
    assert!(a::is_string(sobj));

    let oobj = a::create_object();
    assert!(a::is_object(oobj));
    a::set_bool_value(oobj, true);
    assert!(a::is_object(oobj));
    a::set_bool_value(oobj, false);
    assert!(a::is_object(oobj));

    let refobj = a::create_string_reference(Some(c"conststring"));
    assert!(a::is_string(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::set_bool_value(refobj, true);
    assert!(a::is_string(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::set_bool_value(refobj, false);
    assert!(a::is_string(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::delete(refobj);

    let refobj = a::create_object_reference(oobj);
    assert!(a::is_object(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::set_bool_value(refobj, true);
    assert!(a::is_object(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::set_bool_value(refobj, false);
    assert!(a::is_object(refobj));
    assert!(a::type_flags(refobj) & a::IS_REFERENCE != 0);
    a::delete(refobj);

    a::delete(oobj);
    a::delete(bobj);
    a::delete(sobj);
}

#[test]
fn cjson_parse_big_numbers_should_not_report_error() {
    let _g = lock();
    let v1 = pj("{\"a\": true, \"b\": [ null,9999999999999999999999999999999999999999999999912345678901234567]}");
    let v2 = pj("{\"a\": true, \"b\": [ null,999999999999999999999999999999999999999999999991234567890.1234567E3]}");
    let invalid1 = "{\"a\": true, \"b\": [ null,99999999999999999999999999999999999999999999999.1234567890.1234567]}";
    let invalid2 = "{\"a\": true, \"b\": [ null,99999999999999999999999999999999999999999999999E1234567890e1234567]}";
    assert!(!v1.is_null());
    assert!(!v2.is_null());
    assert!(
        pj(invalid1).is_null(),
        "Invalid big number JSONs should not be parsed."
    );
    assert!(
        pj(invalid2).is_null(),
        "Invalid big number JSONs should not be parsed."
    );
    a::delete(v1);
    a::delete(v2);
}

// =====================================================================================================================
// parse_with_opts.c
// =====================================================================================================================

#[test]
fn parse_with_opts_should_handle_null() {
    let _g = lock();
    assert!(
        a::parse_with_opts(None, false, true).item.is_null(),
        "Failed to handle NULL input."
    );
    let item = a::parse_with_opts(Some(b"{}"), false, false).item;
    assert!(!item.is_null(), "Failed to handle NULL error pointer.");
    a::delete(item);
    assert!(
        a::parse_with_opts(None, false, false).item.is_null(),
        "Failed to handle both NULL."
    );
    assert!(
        a::parse_with_opts(Some(b"{"), false, false).item.is_null(),
        "Failed to handle NULL error pointer with parse error."
    );
}

#[test]
fn parse_with_opts_should_handle_empty_strings() {
    let _g = lock();
    let r = a::parse_with_opts(Some(b""), false, false);
    assert!(r.item.is_null());
    assert_eq!(r.err, ErrPos::At(0));
    let r = a::parse_with_opts(Some(b""), false, true);
    assert!(r.item.is_null());
    assert_eq!(r.end, Some(0));
    assert_eq!(r.err, ErrPos::At(0));
}

#[test]
fn parse_with_opts_should_handle_incomplete_json() {
    let _g = lock();
    let json = b"{ \"name\": ";
    let r = a::parse_with_opts(Some(json), false, true);
    assert!(r.item.is_null());
    assert_eq!(r.end, Some(json.len()));
    assert_eq!(r.err, ErrPos::At(json.len()));
}

#[test]
fn parse_with_opts_should_require_null_if_requested() {
    let _g = lock();
    let item = a::parse_with_opts(Some(b"{}"), true, false).item;
    assert!(!item.is_null());
    a::delete(item);
    let item = a::parse_with_opts(Some(b"{} \n"), true, false).item;
    assert!(!item.is_null());
    a::delete(item);
    assert!(a::parse_with_opts(Some(b"{}x"), true, false).item.is_null());
}

#[test]
fn parse_with_opts_should_return_parse_end() {
    let _g = lock();
    let r = a::parse_with_opts(Some(b"[] empty array XD"), false, true);
    assert!(!r.item.is_null());
    assert_eq!(r.end, Some(2));
    a::delete(r.item);
}

#[test]
fn parse_with_opts_should_parse_utf8_bom() {
    let _g = lock();
    let with_bom = a::parse_with_opts(Some(b"\xEF\xBB\xBF{}"), true, false).item;
    assert!(!with_bom.is_null());
    let without_bom = a::parse_with_opts(Some(b"{}"), true, false).item;
    assert!(!with_bom.is_null());
    assert!(a::compare(with_bom, without_bom, true));
    a::delete(with_bom);
    a::delete(without_bom);
}

// =====================================================================================================================
// compare_tests.c
// =====================================================================================================================

fn compare_from_string(x: &str, y: &str, case_sensitive: bool) -> bool {
    let a_json = pj(x);
    assert!(!a_json.is_null(), "Failed to parse a.");
    let b_json = pj(y);
    assert!(!b_json.is_null(), "Failed to parse b.");
    let result = a::compare(a_json, b_json, case_sensitive);
    a::delete(a_json);
    a::delete(b_json);
    result
}

#[test]
fn cjson_compare_should_compare_null_pointer_as_not_equal() {
    let _g = lock();
    assert!(!a::compare(NULL, NULL, true));
    assert!(!a::compare(NULL, NULL, false));
}

/// The C test zeroes a stack node (type Invalid); here it is a root after a `remove` patch at `""`.
#[test]
fn cjson_compare_should_compare_invalid_as_not_equal() {
    let _g = lock();
    let invalid = make_invalid();
    assert!(!a::compare(invalid, invalid, false));
    assert!(!a::compare(invalid, invalid, true));
    a::delete(invalid);
}

#[test]
fn cjson_compare_should_compare_numbers() {
    let _g = lock();
    assert!(compare_from_string("1", "1", true));
    assert!(compare_from_string("1", "1", false));
    assert!(compare_from_string("0.0001", "0.0001", true));
    assert!(compare_from_string("0.0001", "0.0001", false));
    assert!(compare_from_string("1E100", "10E99", false));
    assert!(!compare_from_string("0.5E-100", "0.5E-101", false));
    assert!(!compare_from_string("1", "2", true));
    assert!(!compare_from_string("1", "2", false));
}

#[test]
fn cjson_compare_should_compare_booleans() {
    let _g = lock();
    assert!(compare_from_string("true", "true", true));
    assert!(compare_from_string("true", "true", false));
    assert!(compare_from_string("false", "false", true));
    assert!(compare_from_string("false", "false", false));
    assert!(!compare_from_string("true", "false", true));
    assert!(!compare_from_string("true", "false", false));
    assert!(!compare_from_string("false", "true", true));
    assert!(!compare_from_string("false", "true", false));
}

#[test]
fn cjson_compare_should_compare_null() {
    let _g = lock();
    assert!(compare_from_string("null", "null", true));
    assert!(compare_from_string("null", "null", false));
    assert!(!compare_from_string("null", "true", true));
    assert!(!compare_from_string("null", "true", false));
}

/// A node whose type is `Number | String` (two kind bits); needs the C layout.
#[test]
fn cjson_compare_should_not_accept_invalid_types() {
    let _g = lock();
    if !a::C_LAYOUT {
        skipped("a node with two kind bits cannot be built (RULEBOOK D14)");
        return;
    }
    let invalid = a::raw_new_with_type(a::NUMBER | a::STRING);
    assert!(!a::compare(invalid, invalid, true));
    assert!(!a::compare(invalid, invalid, false));
    a::delete(invalid);
}

#[test]
fn cjson_compare_should_compare_strings() {
    let _g = lock();
    assert!(compare_from_string("\"abcdefg\"", "\"abcdefg\"", true));
    assert!(compare_from_string("\"abcdefg\"", "\"abcdefg\"", false));
    assert!(!compare_from_string("\"ABCDEFG\"", "\"abcdefg\"", true));
    assert!(!compare_from_string("\"ABCDEFG\"", "\"abcdefg\"", false));
}

/// The C test parses a string and flips `type` to Raw; `cJSON_CreateRaw` builds the same node.
#[test]
fn cjson_compare_should_compare_raw() {
    let _g = lock();
    let raw1 = a::create_raw(Some(b"[true, false]"));
    assert!(!raw1.is_null());
    let raw2 = a::create_raw(Some(b"[true, false]"));
    assert!(!raw2.is_null());
    assert!(a::compare(raw1, raw2, true));
    assert!(a::compare(raw1, raw2, false));
    a::delete(raw1);
    a::delete(raw2);
}

#[test]
fn cjson_compare_should_compare_arrays() {
    let _g = lock();
    assert!(compare_from_string("[]", "[]", true));
    assert!(compare_from_string("[]", "[]", false));
    assert!(compare_from_string(
        "[false,true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        true
    ));
    assert!(compare_from_string(
        "[false,true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        false
    ));
    assert!(compare_from_string("[[[1], 2]]", "[[[1], 2]]", true));
    assert!(compare_from_string("[[[1], 2]]", "[[[1], 2]]", false));
    assert!(!compare_from_string(
        "[true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        true
    ));
    assert!(!compare_from_string(
        "[true,null,42,\"string\",[],{}]",
        "[false, true, null, 42, \"string\", [], {}]",
        false
    ));
    // Arrays that are a prefix of another array
    assert!(!compare_from_string("[1,2,3]", "[1,2]", true));
    assert!(!compare_from_string("[1,2,3]", "[1,2]", false));
}

#[test]
fn cjson_compare_should_compare_objects() {
    let _g = lock();
    assert!(compare_from_string("{}", "{}", true));
    assert!(compare_from_string("{}", "{}", false));
    assert!(compare_from_string(
        "{\"false\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        true
    ));
    assert!(!compare_from_string(
        "{\"False\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        true
    ));
    assert!(compare_from_string(
        "{\"False\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        false
    ));
    assert!(!compare_from_string(
        "{\"Flse\": false, \"true\": true, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        "{\"true\": true, \"false\": false, \"null\": null, \"number\": 42, \"string\": \"string\", \"array\": [], \"object\": {}}",
        false
    ));
    // test objects that are a subset of each other
    assert!(!compare_from_string(
        "{\"one\": 1, \"two\": 2}",
        "{\"one\": 1, \"two\": 2, \"three\": 3}",
        true
    ));
    assert!(!compare_from_string(
        "{\"one\": 1, \"two\": 2}",
        "{\"one\": 1, \"two\": 2, \"three\": 3}",
        false
    ));
}

// =====================================================================================================================
// cjson_add.c
// =====================================================================================================================

fn type_is(i: Item, t: i32) {
    assert!(!i.is_null());
    assert_eq!(a::type_flags(i), t);
}

#[test]
fn cjson_add_null_should_add_null() {
    let _g = lock();
    let root = a::create_object();
    a::add_null_to_object(root, Some(b"null"));
    type_is(a::object_item_cs(root, Some(b"null")), a::NULL_T);
    a::delete(root);
}

#[test]
fn cjson_add_null_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_null_to_object(NULL, Some(b"null")).is_null());
    assert!(a::add_null_to_object(root, None).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_null_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_null_to_object(root, Some(b"null")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_add_true_should_add_true() {
    let _g = lock();
    let root = a::create_object();
    a::add_true_to_object(root, Some(b"true"));
    type_is(a::object_item_cs(root, Some(b"true")), a::TRUE);
    a::delete(root);
}

#[test]
fn cjson_add_true_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_true_to_object(NULL, Some(b"true")).is_null());
    assert!(a::add_true_to_object(root, None).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_true_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_true_to_object(root, Some(b"true")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_create_int_array_should_fail_on_allocation_failure() {
    let _g = lock();
    let numbers = [1, 2, 3];
    {
        let h = failing_hooks();
        assert!(a::create_int_array(Some(&numbers), 3).is_null());
        drop(h);
    }
}

#[test]
fn cjson_create_float_array_should_fail_on_allocation_failure() {
    let _g = lock();
    let numbers = [1.0f32, 2.0, 3.0];
    {
        let h = failing_hooks();
        assert!(a::create_float_array(Some(&numbers), 3).is_null());
        drop(h);
    }
}

#[test]
fn cjson_create_double_array_should_fail_on_allocation_failure() {
    let _g = lock();
    let numbers = [1.0f64, 2.0, 3.0];
    {
        let h = failing_hooks();
        assert!(a::create_double_array(Some(&numbers), 3).is_null());
        drop(h);
    }
}

#[test]
fn cjson_create_string_array_should_fail_on_allocation_failure() {
    let _g = lock();
    let strings: [&[u8]; 3] = [b"1", b"2", b"3"];
    {
        let h = failing_hooks();
        assert!(a::create_string_array(Some(&strings), 3).is_null());
        drop(h);
    }
}

#[test]
fn cjson_add_false_should_add_false() {
    let _g = lock();
    let root = a::create_object();
    a::add_false_to_object(root, Some(b"false"));
    type_is(a::object_item_cs(root, Some(b"false")), a::FALSE);
    a::delete(root);
}

#[test]
fn cjson_add_false_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_false_to_object(NULL, Some(b"false")).is_null());
    assert!(a::add_false_to_object(root, None).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_false_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_false_to_object(root, Some(b"false")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_add_bool_should_add_bool() {
    let _g = lock();
    let root = a::create_object();
    // true
    a::add_bool_to_object(root, Some(b"true"), true);
    type_is(a::object_item_cs(root, Some(b"true")), a::TRUE);
    // false
    a::add_bool_to_object(root, Some(b"false"), false);
    type_is(a::object_item_cs(root, Some(b"false")), a::FALSE);
    a::delete(root);
}

#[test]
fn cjson_add_bool_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_bool_to_object(NULL, Some(b"false"), false).is_null());
    assert!(a::add_bool_to_object(root, None, false).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_bool_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_bool_to_object(root, Some(b"false"), false).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_add_number_should_add_number() {
    let _g = lock();
    let root = a::create_object();
    a::add_number_to_object(root, Some(b"number"), 42.0);
    let number = a::object_item_cs(root, Some(b"number"));
    type_is(number, a::NUMBER);
    assert_double(a::value_double(number), 42.0);
    assert_eq!(a::value_int(number), 42);
    a::delete(root);
}

#[test]
fn cjson_add_number_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_number_to_object(NULL, Some(b"number"), 42.0).is_null());
    assert!(a::add_number_to_object(root, None, 42.0).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_number_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_number_to_object(root, Some(b"number"), 42.0).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_add_string_should_add_string() {
    let _g = lock();
    let root = a::create_object();
    a::add_string_to_object(root, Some(b"string"), Some(b"Hello World!"));
    let string = a::object_item_cs(root, Some(b"string"));
    type_is(string, a::STRING);
    assert_eq!(vstr(string).as_deref(), Some("Hello World!"));
    a::delete(root);
}

#[test]
fn cjson_add_string_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_string_to_object(NULL, Some(b"string"), Some(b"string")).is_null());
    assert!(a::add_string_to_object(root, None, Some(b"string")).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_string_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_string_to_object(root, Some(b"string"), Some(b"string")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn cjson_add_raw_should_add_raw() {
    let _g = lock();
    let root = a::create_object();
    a::add_raw_to_object(root, Some(b"raw"), Some(b"{}"));
    let raw = a::object_item_cs(root, Some(b"raw"));
    type_is(raw, a::RAW);
    assert_eq!(vstr(raw).as_deref(), Some("{}"));
    a::delete(root);
}

#[test]
fn cjson_add_raw_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_raw_to_object(NULL, Some(b"raw"), Some(b"{}")).is_null());
    assert!(a::add_raw_to_object(root, None, Some(b"{}")).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_raw_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_raw_to_object(root, Some(b"raw"), Some(b"{}")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn c_json_add_object_should_add_object() {
    let _g = lock();
    let root = a::create_object();
    a::add_object_to_object(root, Some(b"object"));
    type_is(a::object_item_cs(root, Some(b"object")), a::OBJECT);
    a::delete(root);
}

#[test]
fn cjson_add_object_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_object_to_object(NULL, Some(b"object")).is_null());
    assert!(a::add_object_to_object(root, None).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_object_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_object_to_object(root, Some(b"object")).is_null());
        drop(h);
    }
    a::delete(root);
}

#[test]
fn c_json_add_array_should_add_array() {
    let _g = lock();
    let root = a::create_object();
    a::add_array_to_object(root, Some(b"array"));
    type_is(a::object_item_cs(root, Some(b"array")), a::ARRAY);
    a::delete(root);
}

#[test]
fn cjson_add_array_should_fail_with_null_pointers() {
    let _g = lock();
    let root = a::create_object();
    assert!(a::add_array_to_object(NULL, Some(b"array")).is_null());
    assert!(a::add_array_to_object(root, None).is_null());
    a::delete(root);
}

#[test]
fn cjson_add_array_should_fail_on_allocation_failure() {
    let _g = lock();
    let root = a::create_object();
    {
        let h = failing_hooks();
        assert!(a::add_array_to_object(root, Some(b"array")).is_null());
        drop(h);
    }
    a::delete(root);
}

// =====================================================================================================================
// minify_tests.c
// =====================================================================================================================

fn minified(input: &[u8]) -> String {
    String::from_utf8_lossy(&a::minify(Some(input)).unwrap()).into_owned()
}

#[test]
fn cjson_minify_should_not_overflow_buffer() {
    let _g = lock();
    assert_eq!("", minified(b"/* bla"));
    assert_eq!("\"\\", minified(b"\"\\"));
}

#[test]
fn cjson_minify_should_remove_single_line_comments() {
    let _g = lock();
    assert_eq!(
        "{}",
        minified(b"{// this is {} \"some kind\" of [] comment /*, don't you see\n}")
    );
}

#[test]
fn cjson_minify_should_remove_spaces() {
    let _g = lock();
    assert_eq!("{\"key\":true}", minified(b"{ \"key\":\ttrue\r\n    }"));
}

#[test]
fn cjson_minify_should_remove_multiline_comments() {
    let _g = lock();
    assert_eq!(
        "{}",
        minified(b"{/* this is\n a /* multi\n //line \n {comment \"\\\" */}")
    );
}

#[test]
fn cjson_minify_should_not_modify_strings() {
    let _g = lock();
    let to_minify = "\"this is a string \\\" \\t bla\"";
    assert_eq!(to_minify, minified(to_minify.as_bytes()));
}

#[test]
fn cjson_minify_should_minify_json() {
    let _g = lock();
    let to_minify = concat!(
        "{\n",
        "    \"glossary\": { // comment\n",
        "        \"title\": \"example glossary\",\n",
        "  /* multi\n",
        " line */\n",
        "\t\t\"GlossDiv\": {\n",
        "            \"title\": \"S\",\n",
        "\t\t\t\"GlossList\": {\n",
        "                \"GlossEntry\": {\n",
        "                    \"ID\": \"SGML\",\n",
        "\t\t\t\t\t\"SortAs\": \"SGML\",\n",
        "\t\t\t\t\t\"Acronym\": \"SGML\",\n",
        "\t\t\t\t\t\"Abbrev\": \"ISO 8879:1986\",\n",
        "\t\t\t\t\t\"GlossDef\": {\n",
        "\t\t\t\t\t\t\"GlossSeeAlso\": [\"GML\", \"XML\"]\n",
        "                    },\n",
        "\t\t\t\t\t\"GlossSee\": \"markup\"\n",
        "                }\n",
        "            }\n",
        "        }\n",
        "    }\n",
        "}"
    );
    let expected = concat!(
        "{",
        "\"glossary\":{",
        "\"title\":\"example glossary\",",
        "\"GlossDiv\":{",
        "\"title\":\"S\",",
        "\"GlossList\":{",
        "\"GlossEntry\":{",
        "\"ID\":\"SGML\",",
        "\"SortAs\":\"SGML\",",
        "\"Acronym\":\"SGML\",",
        "\"Abbrev\":\"ISO 8879:1986\",",
        "\"GlossDef\":{",
        "\"GlossSeeAlso\":[\"GML\",\"XML\"]",
        "},",
        "\"GlossSee\":\"markup\"",
        "}",
        "}",
        "}",
        "}",
        "}"
    );
    assert_eq!(expected, minified(to_minify.as_bytes()));
}

#[test]
fn cjson_minify_should_not_loop_infinitely() {
    let _g = lock();
    // this should not be an infinite loop
    let _ = minified(b"8 / 5\n");
}

// =====================================================================================================================
// readme_examples.c
// =====================================================================================================================

const MONITOR_JSON: &str = "{\n\
\t\"name\":\t\"Awesome 4K\",\n\
\t\"resolutions\":\t[{\n\
\t\t\t\"width\":\t1280,\n\
\t\t\t\"height\":\t720\n\
\t\t}, {\n\
\t\t\t\"width\":\t1920,\n\
\t\t\t\"height\":\t1080\n\
\t\t}, {\n\
\t\t\t\"width\":\t3840,\n\
\t\t\t\"height\":\t2160\n\
\t\t}]\n\
}";

const RESOLUTIONS: [[u32; 2]; 3] = [[1280, 720], [1920, 1080], [3840, 2160]];

fn create_monitor() -> Option<String> {
    let monitor = a::create_object();
    let name = a::create_string(Some(b"Awesome 4K"));
    // after creation was successful, immediately add it to the monitor, thereby transferring ownership
    a::add_item_to_object(monitor, Some(b"name"), name);
    let resolutions = a::create_array();
    a::add_item_to_object(monitor, Some(b"resolutions"), resolutions);
    for r in RESOLUTIONS {
        let resolution = a::create_object();
        a::add_item_to_array(resolutions, resolution);
        let width = a::create_number(r[0] as f64);
        a::add_item_to_object(resolution, Some(b"width"), width);
        let height = a::create_number(r[1] as f64);
        a::add_item_to_object(resolution, Some(b"height"), height);
    }
    let s = pf(monitor);
    a::delete(monitor);
    s
}

fn create_monitor_with_helpers() -> Option<String> {
    let monitor = a::create_object();
    assert!(!a::add_string_to_object(monitor, Some(b"name"), Some(b"Awesome 4K")).is_null());
    let resolutions = a::add_array_to_object(monitor, Some(b"resolutions"));
    assert!(!resolutions.is_null());
    for r in RESOLUTIONS {
        let resolution = a::create_object();
        assert!(!a::add_number_to_object(resolution, Some(b"width"), r[0] as f64).is_null());
        assert!(!a::add_number_to_object(resolution, Some(b"height"), r[1] as f64).is_null());
        a::add_item_to_array(resolutions, resolution);
    }
    let s = pf(monitor);
    a::delete(monitor);
    s
}

/// `compare_double` of cJSON.c (the Judge shim of Stage 1 copies it as well).
fn compare_double(x: f64, y: f64) -> bool {
    let max_val = if x.abs() > y.abs() { x.abs() } else { y.abs() };
    (x - y).abs() <= max_val * f64::EPSILON
}

/// 1 if the monitor supports full hd, 0 otherwise.
fn supports_full_hd(monitor: &str) -> bool {
    let monitor_json = pj(monitor);
    if monitor_json.is_null() {
        return false;
    }
    let mut status = false;
    let name = a::object_item_cs(monitor_json, Some(b"name"));
    if a::is_string(name) && a::value_string(name).is_some() {
        println!("Checking monitor \"{}\"", vstr(name).unwrap());
    }
    let resolutions = a::object_item_cs(monitor_json, Some(b"resolutions"));
    for resolution in a::children(resolutions) {
        let width = a::object_item_cs(resolution, Some(b"width"));
        let height = a::object_item_cs(resolution, Some(b"height"));
        if !a::is_number(width) || !a::is_number(height) {
            status = false;
            break;
        }
        if compare_double(a::value_double(width), 1920.0)
            && compare_double(a::value_double(height), 1080.0)
        {
            status = true;
            break;
        }
    }
    a::delete(monitor_json);
    status
}

#[test]
fn create_monitor_should_create_a_monitor() {
    let _g = lock();
    assert_eq!(create_monitor().as_deref(), Some(MONITOR_JSON));
}

#[test]
fn create_monitor_with_helpers_should_create_a_monitor() {
    let _g = lock();
    assert_eq!(create_monitor_with_helpers().as_deref(), Some(MONITOR_JSON));
}

#[test]
fn supports_full_hd_should_check_for_full_hd_support() {
    let _g = lock();
    let monitor_without_hd = "{\n\
\t\t\"name\": \"lame monitor\",\n\
\t\t\"resolutions\":\t[{\n\
\t\t\t\"width\":\t640,\n\
\t\t\t\"height\":\t480\n\
\t\t}]\n\
}";
    assert!(supports_full_hd(MONITOR_JSON));
    assert!(!supports_full_hd(monitor_without_hd));
}

// =====================================================================================================================
// json_patch_tests.c
// =====================================================================================================================

fn parse_test_file(filename: &str) -> Item {
    let file = read_file(filename);
    let json = a::parse(Some(&file)).item;
    assert!(!json.is_null(), "Failed to parse test json.");
    assert!(a::is_array(json), "Json is not an array.");
    json
}

fn test_apply_patch(test: Item) -> bool {
    let comment = a::object_item_cs(test, Some(b"comment"));
    if a::is_string(comment) {
        println!("Testing \"{}\"", vstr(comment).unwrap());
    } else {
        println!("Testing unknown");
    }
    let disabled = a::object_item_cs(test, Some(b"disabled"));
    if a::is_true(disabled) {
        println!("SKIPPED");
        return true;
    }
    let doc = a::object_item_cs(test, Some(b"doc"));
    assert!(!doc.is_null(), "No \"doc\" in the test.");
    let patch = a::object_item_cs(test, Some(b"patch"));
    assert!(!patch.is_null(), "No \"patch\"in the test.");
    // Make a working copy of 'doc'
    let object = a::duplicate(doc, true);
    assert!(!object.is_null());
    let expected = a::object_item_cs(test, Some(b"expected"));
    let error_element = a::object_item_cs(test, Some(b"error"));
    let successful;
    if !error_element.is_null() {
        // excepting an error
        assert!(
            0 != a::apply_patches_cs(object, patch),
            "Test didn't fail as it's supposed to."
        );
        successful = true;
    } else {
        // apply the patch
        assert_eq!(
            0,
            a::apply_patches_cs(object, patch),
            "Failed to apply patches."
        );
        successful = if expected.is_null() {
            true
        } else {
            a::compare(object, expected, true)
        };
    }
    a::delete(object);
    println!("{}", if successful { "OK" } else { "FAILED" });
    successful
}

fn test_generate_test(test: Item) -> bool {
    let disabled = a::object_item_cs(test, Some(b"disabled"));
    if a::is_true(disabled) {
        println!("SKIPPED");
        return true;
    }
    let doc = a::object_item_cs(test, Some(b"doc"));
    assert!(!doc.is_null(), "No \"doc\" in the test.");
    // Make a working copy of 'doc'
    let object = a::duplicate(doc, true);
    assert!(!object.is_null());
    let expected = a::object_item_cs(test, Some(b"expected"));
    if expected.is_null() {
        a::delete(object);
        // if there is no expected output, this test doesn't make sense
        return true;
    }
    let patch = a::generate_patches_cs(doc, expected);
    assert!(!patch.is_null(), "Failed to generate patches.");
    println!("{}", pf(patch).unwrap_or_default());
    // apply the generated patch
    assert_eq!(
        0,
        a::apply_patches_cs(object, patch),
        "Failed to apply generated patch."
    );
    let successful = a::compare(object, expected, true);
    a::delete(patch);
    a::delete(object);
    println!(
        "{}",
        if successful {
            "generated patch: OK"
        } else {
            "generated patch: FAILED"
        }
    );
    successful
}

fn run_patch_file(filename: &str) {
    let tests = parse_test_file(filename);
    let mut failed = false;
    for test in a::children(tests) {
        failed |= !test_apply_patch(test);
        failed |= !test_generate_test(test);
    }
    a::delete(tests);
    assert!(!failed, "Some tests failed.");
}

#[test]
fn cjson_utils_should_pass_json_patch_test_tests() {
    let _g = lock();
    run_patch_file("json-patch-tests/tests.json");
}

#[test]
fn cjson_utils_should_pass_json_patch_test_spec_tests() {
    let _g = lock();
    run_patch_file("json-patch-tests/spec_tests.json");
}

#[test]
fn cjson_utils_should_pass_json_patch_test_cjson_utils_tests() {
    let _g = lock();
    run_patch_file("json-patch-tests/cjson-utils-tests.json");
}

// =====================================================================================================================
// old_utils_tests.c
// =====================================================================================================================

const MERGES: [[&str; 3]; 15] = [
    ["{\"a\":\"b\"}", "{\"a\":\"c\"}", "{\"a\":\"c\"}"],
    [
        "{\"a\":\"b\"}",
        "{\"b\":\"c\"}",
        "{\"a\":\"b\",\"b\":\"c\"}",
    ],
    ["{\"a\":\"b\"}", "{\"a\":null}", "{}"],
    ["{\"a\":\"b\",\"b\":\"c\"}", "{\"a\":null}", "{\"b\":\"c\"}"],
    ["{\"a\":[\"b\"]}", "{\"a\":\"c\"}", "{\"a\":\"c\"}"],
    ["{\"a\":\"c\"}", "{\"a\":[\"b\"]}", "{\"a\":[\"b\"]}"],
    [
        "{\"a\":{\"b\":\"c\"}}",
        "{\"a\":{\"b\":\"d\",\"c\":null}}",
        "{\"a\":{\"b\":\"d\"}}",
    ],
    ["{\"a\":[{\"b\":\"c\"}]}", "{\"a\":[1]}", "{\"a\":[1]}"],
    ["[\"a\",\"b\"]", "[\"c\",\"d\"]", "[\"c\",\"d\"]"],
    ["{\"a\":\"b\"}", "[\"c\"]", "[\"c\"]"],
    ["{\"a\":\"foo\"}", "null", "null"],
    ["{\"a\":\"foo\"}", "\"bar\"", "\"bar\""],
    ["{\"e\":null}", "{\"a\":1}", "{\"e\":null,\"a\":1}"],
    ["[1,2]", "{\"a\":\"b\",\"c\":null}", "{\"a\":\"b\"}"],
    [
        "{}",
        "{\"a\":{\"bb\":{\"ccc\":null}}}",
        "{\"a\":{\"bb\":{}}}",
    ],
];

#[test]
fn json_pointer_tests() {
    let _g = lock();
    let json = "{\
\"foo\": [\"bar\", \"baz\"],\
\"\": 0,\
\"a/b\": 1,\
\"c%d\": 2,\
\"e^f\": 3,\
\"g|h\": 4,\
\"i\\\\j\": 5,\
\"k\\\"l\": 6,\
\" \": 7,\
\"m~n\": 8\
}";
    let root = pj(json);
    let gp = |p: &str| a::get_pointer(root, Some(p.as_bytes()));
    let oi = |k: &str| a::object_item(root, Some(k.as_bytes()));
    assert!(gp("") == root);
    assert!(gp("/foo") == oi("foo"));
    assert!(gp("/foo/0") == a::child(oi("foo")));
    assert!(gp("/foo/0") == a::child(oi("foo")));
    assert!(gp("/") == oi(""));
    assert!(gp("/a~1b") == oi("a/b"));
    assert!(gp("/c%d") == oi("c%d"));
    assert!(gp("/c^f") == oi("c^f"));
    assert!(gp("/c|f") == oi("c|f"));
    assert!(gp("/i\\j") == oi("i\\j"));
    assert!(gp("/k\"l") == oi("k\"l"));
    assert!(gp("/ ") == oi(" "));
    assert!(gp("/m~0n") == oi("m~n"));
    a::delete(root);
}

#[test]
fn misc_tests() {
    let _g = lock();
    let numbers = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    println!("JSON Pointer construct");
    let object = a::create_object();
    let nums = a::create_int_array(Some(&numbers), 10);
    let num6 = a::array_item(nums, 6);
    a::add_item_to_object(object, Some(b"numbers"), nums);
    let pointer = a::find_pointer_from_object_to(object, num6);
    assert_eq!(pointer.as_deref(), Some(b"/numbers/6".as_slice()));
    let pointer = a::find_pointer_from_object_to(object, nums);
    assert_eq!(pointer.as_deref(), Some(b"/numbers".as_slice()));
    let pointer = a::find_pointer_from_object_to(object, object);
    assert_eq!(pointer.as_deref(), Some(b"".as_slice()));

    let object1 = a::create_object();
    let object2 = a::create_string(Some(b"m~n"));
    a::add_item_to_object(object1, Some(b"m~n"), object2);
    let pointer = a::find_pointer_from_object_to(object1, object2);
    assert_eq!(pointer.as_deref(), Some(b"/m~0n".as_slice()));

    let object3 = a::create_object();
    let object4 = a::create_string(Some(b"m/n"));
    a::add_item_to_object(object3, Some(b"m/n"), object4);
    let pointer = a::find_pointer_from_object_to(object3, object4);
    assert_eq!(pointer.as_deref(), Some(b"/m~1n".as_slice()));

    a::delete(object);
    a::delete(object1);
    a::delete(object3);
}

#[test]
fn sort_tests() {
    let _g = lock();
    let random = b"QWERTYUIOPASDFGHJKLZXCVBNM";
    // JSON Sort test:
    let sortme = a::create_object();
    for &c in random.iter() {
        a::add_item_to_object(sortme, Some(&[c]), a::create_number(1.0));
    }
    a::sort_object(sortme);
    // check sorting
    let mut current = a::next(a::child(sortme));
    let mut i = 1;
    while i < 26 && !current.is_null() && !a::prev(current).is_null() {
        assert!(a::key(current).unwrap()[0] >= a::key(a::prev(current)).unwrap()[0]);
        current = a::next(current);
        i += 1;
    }
    a::delete(sortme);
}

#[test]
fn merge_tests() {
    let _g = lock();
    println!("JSON Merge Patch tests");
    for m in MERGES {
        let object_to_be_merged = pj(m[0]);
        let patch = pj(m[1]);
        let patchtext = pu(patch);
        let object_to_be_merged = a::merge_patch(object_to_be_merged, patch);
        let after = pu(object_to_be_merged);
        assert_eq!(after.as_deref(), Some(m[2]));
        drop(patchtext);
        a::delete(object_to_be_merged);
        a::delete(patch);
    }
}

/// When the patch is a subtree of the target, `merge_patch` must duplicate the patch before deleting the
/// target (heap-use-after-free regression test).
#[test]
fn merge_patch_should_not_read_freed_memory_when_patch_is_subtree() {
    let _g = lock();
    let target = pj("{\"a\":[1,2,3]}");
    let patch = a::object_item(target, Some(b"a"));
    assert!(!target.is_null());
    assert!(!patch.is_null());
    let result = a::merge_patch(target, patch);
    assert!(!result.is_null());
    assert!(a::is_array(result));
    assert_eq!(3, a::array_size(result));
    assert_eq!(1, a::value_int(a::array_item(result, 0)));
    assert_eq!(2, a::value_int(a::array_item(result, 1)));
    assert_eq!(3, a::value_int(a::array_item(result, 2)));
    a::delete(result);
}

#[test]
fn generate_merge_tests() {
    let _g = lock();
    for m in MERGES {
        let from = pj(m[0]);
        let to = pj(m[2]);
        let patch = a::generate_merge_patch(from, to);
        let from = a::merge_patch(from, patch);
        assert_eq!(pu(from).as_deref(), Some(m[2]));
        a::delete(from);
        a::delete(to);
        a::delete(patch);
    }
}

// =====================================================================================================================
// misc_utils_tests.c
// =====================================================================================================================

#[test]
fn cjson_utils_functions_shouldnt_crash_with_null_pointers() {
    let _g = lock();
    let item = a::create_string(Some(b"item"));
    assert!(!item.is_null());
    assert!(a::get_pointer(item, None).is_null());
    assert!(a::get_pointer(NULL, Some(b"pointer")).is_null());
    assert!(a::get_pointer_cs(NULL, Some(b"pointer")).is_null());
    assert!(a::get_pointer_cs(item, None).is_null());
    assert!(a::generate_patches(item, NULL).is_null());
    assert!(a::generate_patches(NULL, item).is_null());
    assert!(a::generate_patches_cs(item, NULL).is_null());
    assert!(a::generate_patches_cs(NULL, item).is_null());
    a::add_patch_to_array(item, Some(b"path"), Some(b"add"), NULL);
    a::add_patch_to_array(item, Some(b"path"), None, item);
    a::add_patch_to_array(item, None, Some(b"add"), item);
    a::add_patch_to_array(NULL, Some(b"path"), Some(b"add"), item);
    a::apply_patches(item, NULL);
    a::apply_patches(NULL, item);
    a::apply_patches_cs(item, NULL);
    a::apply_patches_cs(NULL, item);
    assert!(a::merge_patch(item, NULL).is_null());
    let item = a::create_string(Some(b"item"));
    assert!(a::merge_patch_cs(item, NULL).is_null());
    let item = a::create_string(Some(b"item"));
    // these calls are actually valid
    // cJSONUtils_MergePatch(NULL, item); cJSONUtils_MergePatchCaseSensitive(NULL, item);
    // cJSONUtils_GenerateMergePatch(item, NULL); cJSONUtils_GenerateMergePatch(NULL, item);
    // cJSONUtils_GenerateMergePatchCaseSensitive(item, NULL); cJSONUtils_GenerateMergePatchCaseSensitive(NULL, item);
    assert!(a::find_pointer_from_object_to(item, NULL).is_none());
    assert!(a::find_pointer_from_object_to(NULL, item).is_none());
    a::sort_object(NULL);
    a::sort_object_cs(NULL);
    a::delete(item);
}

// ---- RULEBOOK D3 / D19: the process locale (Stage 2 Judge addition, user 2026-10-11) -----------------------------

mod locale_probe {
    use std::ffi::{c_char, c_int, CStr};

    extern "C" {
        fn setlocale(category: c_int, locale: *const c_char) -> *const c_char;
    }
    #[cfg(target_os = "macos")]
    pub const LC_CTYPE: c_int = 2;
    #[cfg(target_os = "macos")]
    pub const LC_NUMERIC: c_int = 4;
    #[cfg(not(target_os = "macos"))]
    pub const LC_CTYPE: c_int = 0;
    #[cfg(not(target_os = "macos"))]
    pub const LC_NUMERIC: c_int = 1;

    /// Sets the first of `names` the OS accepts; returns its name.
    pub fn set_first(category: c_int, names: &[&'static CStr]) -> Option<&'static CStr> {
        names
            .iter()
            .copied()
            .find(|n| !unsafe { setlocale(category, n.as_ptr()) }.is_null())
    }

    pub fn reset(category: c_int) {
        unsafe { setlocale(category, c"C".as_ptr()) };
    }
}

/// D3: `1.5` parses to 1.5 and prints `1.5` after `setlocale(LC_NUMERIC, <comma locale>)`. The Stage 2 crate
/// always uses the C locale; the C original and the Faithful port follow libc (and `ENABLE_LOCALES`), so the
/// assertions run only for the real Stage 2 crate.
#[test]
fn locale_numeric_comma_locale_still_parses_and_prints_a_dot_d3() {
    use locale_probe as lp;
    let _g = lock();
    if a::C_LAYOUT {
        skipped("D3 comma-locale check: the Faithful port follows libc (feature `locales`)");
        return;
    }
    let names = [
        c"de_DE.UTF-8",
        c"fr_FR.UTF-8",
        c"ru_RU.UTF-8",
        c"nl_NL.UTF-8",
        c"de_DE.ISO8859-1",
    ];
    let Some(used) = lp::set_first(lp::LC_NUMERIC, &names) else {
        skipped("D3 comma-locale check: the OS has no comma locale");
        return;
    };
    let item = pj("1.5");
    let printed = a::print_unformatted(a::create_number(1.5)).map(|p| p.bytes().to_vec());
    let arr = pj("[1.5,-0.5e2]");
    let arr_printed = a::print_unformatted(arr).map(|p| p.bytes().to_vec());
    let comma = pj("1,5");
    let comma_is_null = comma.is_null();
    let (kind_ok, v) = (!item.is_null(), a::value_double(item));
    lp::reset(lp::LC_NUMERIC);
    assert!(kind_ok, "parse of 1.5 failed under {used:?}");
    assert_eq!(v, 1.5, "D3: 1.5 must parse to 1.5 under {used:?}");
    assert_eq!(
        printed.as_deref(),
        Some(&b"1.5"[..]),
        "D3: 1.5 must print as 1.5 under {used:?}"
    );
    assert_eq!(
        arr_printed.as_deref(),
        Some(&b"[1.5,-50]"[..]),
        "D3: array print under {used:?}"
    );
    assert!(
        comma_is_null || a::value_double(comma) == 1.0,
        "D3: `1,5` is not a number under {used:?}"
    );
    a::delete(item);
    a::delete(arr);
    a::delete(comma);
}

/// D19: under a Latin-1 `LC_CTYPE`, the keys `\xC9` and `\xE9` are NOT case-equal in Stage 2 (ASCII-only
/// `tolower`); `E` and `e` still are.
#[test]
fn locale_ctype_latin1_keys_are_not_case_equal_d19() {
    use locale_probe as lp;
    let _g = lock();
    if a::C_LAYOUT {
        skipped("D19 Latin-1 check: the Faithful port follows libc `tolower`");
        return;
    }
    let names = [c"de_DE.ISO8859-1", c"fr_FR.ISO8859-1", c"en_US.ISO8859-15"];
    let Some(used) = lp::set_first(lp::LC_CTYPE, &names) else {
        skipped("D19 Latin-1 check: the OS has no Latin-1 locale");
        return;
    };
    let o = pb(b"{\"\xC9\":1,\"E\":2}");
    let hit_e9 = a::object_item(o, Some(b"\xE9"));
    let hit_c9 = a::object_item(o, Some(b"\xC9"));
    let hit_e = a::object_item(o, Some(b"e"));
    let x = pb(b"{\"\xC9\":1}");
    let y = pb(b"{\"\xE9\":1}");
    let eq = a::compare(x, y, false);
    lp::reset(lp::LC_CTYPE);
    assert!(
        hit_e9.is_null(),
        "D19: key \\xE9 must not match \\xC9 under {used:?}"
    );
    assert!(!hit_c9.is_null(), "exact key must match under {used:?}");
    assert!(
        !hit_e.is_null(),
        "ASCII case-insensitive match must still work under {used:?}"
    );
    assert!(
        !eq,
        "D19: {{\"\\xC9\":1}} and {{\"\\xE9\":1}} must not compare equal under {used:?}"
    );
    a::delete(o);
    a::delete(x);
    a::delete(y);
}

// ---- Stage 2 review additions (stage2_port_review.md, BLOCK 1-4, D22) -----------------------------------------

/// BLOCK 1: `cJSONUtils_FindPointerFromObjectTo` copies the empty pointer with the checked
/// `cJSONUtils_strdup` (`cJSON_Utils.c:211`); a failing size-1 allocation returns NULL, no abort (not a D2 site).
#[test]
fn find_pointer_from_object_to_returns_null_when_the_root_copy_fails() {
    let _g = lock();
    let o = pb(b"{\"a\":{\"b\":1}}");
    let target = a::child(a::child(o));
    assert!(!target.is_null());
    a::hooks_on(1);
    let p = a::find_pointer_from_object_to(o, target);
    a::hooks_off();
    assert!(p.is_none(), "the size-1 strdup failed: NULL expected");
    a::delete(o);
}

/// BLOCK 2: `GeneratePatches([], [1])` allocates the path buffer (22 bytes, `cJSON_Utils.c:1175`) and writes
/// through it only inside the loops; with the buffer failing and no element to write, C returns the patch array.
#[test]
fn generate_patches_with_a_failing_path_buffer_and_nothing_to_write() {
    let _g = lock();
    let from = pb(b"[]");
    let to = pb(b"[1]");
    a::hooks_on(2);
    let p = a::generate_patches(from, to);
    a::hooks_off();
    assert!(
        !p.is_null(),
        "the array exists (allocation 1); the failed path buffer is never written"
    );
    assert_eq!(
        a::print_unformatted(p).unwrap().bytes(),
        b"[{\"op\":\"add\",\"path\":\"/-\",\"value\":1}]"
    );
    a::delete(p);
    a::delete(from);
    a::delete(to);
}

fn patch_with(members: &[(&[u8], Option<&'static std::ffi::CStr>, Option<&[u8]>)]) -> Item {
    let patch = a::create_object();
    for (key, reference, text) in members {
        let v = match (reference, text) {
            (_, Some(t)) => a::create_string(Some(t)),
            _ => a::create_string_reference(None),
        };
        a::add_item_to_object(patch, Some(key), v);
    }
    let arr = a::create_array();
    a::add_item_to_array(arr, patch);
    arr
}

/// D22 (defined in C): `test` with a NULL `path` resolves `get_item_from_pointer(NULL)` = NULL: status 1.
#[test]
fn apply_patches_test_with_null_path_is_status_1() {
    let _g = lock();
    let doc = pb(b"{\"a\":1}");
    let patches = patch_with(&[(b"op", None, Some(b"test")), (b"path", None, None)]);
    assert_eq!(a::apply_patches(doc, patches), 1);
    a::delete(patches);
    a::delete(doc);
}

/// D22 (defined in C): `copy` with a NULL `from` is status 5.
#[test]
fn apply_patches_copy_with_null_from_is_status_5() {
    let _g = lock();
    let doc = pb(b"{\"a\":1}");
    let patches = patch_with(&[
        (b"op", None, Some(b"copy")),
        (b"path", None, Some(b"/b")),
        (b"from", None, None),
    ]);
    assert_eq!(a::apply_patches(doc, patches), 5);
    a::delete(patches);
    a::delete(doc);
}

/// BLOCK 4: a buffer without a NUL after the slice: the parser's length is `strlen`-bounded (C: `strlen + 1`
/// stays inside the slice only when a NUL is there), so errors and the parse end are the same as with a NUL.
#[test]
fn parse_of_a_slice_without_nul_reports_the_same_errors() {
    let _g = lock();
    let r = a::parse_unterminated(b"[1");
    assert!(r.item.is_null());
    assert_eq!(r.err, ErrPos::At(2));
    let r = a::parse_unterminated(b"{\"a\"");
    assert!(r.item.is_null());
    assert_eq!(r.err, ErrPos::At(4));
    let r = a::parse_unterminated(b"[");
    assert!(r.item.is_null());
    assert_eq!(r.err, ErrPos::At(1));
}

#[test]
fn parse_with_opts_of_a_slice_without_nul_succeeds_with_the_end() {
    let _g = lock();
    let r = a::parse_with_opts_unterminated(b"1", true, true);
    assert!(!r.item.is_null());
    assert_eq!(r.end, Some(1));
    a::delete(r.item);
    let r = a::parse_with_opts_unterminated(b"[1] ", true, true);
    assert!(!r.item.is_null());
    assert_eq!(r.end, Some(4));
    a::delete(r.item);
}

/// Patch array of one operation; `None` makes that member a String item without `valuestring` (D22).
/// `from`: `None` = no member, `Some(None)` = NULL `valuestring`, `Some(Some(t))` = text.
fn patch_member(
    op: Option<&[u8]>,
    path: Option<&[u8]>,
    from: Option<Option<&[u8]>>,
    with_value: bool,
) -> Item {
    let text = |t: Option<&[u8]>| match t {
        Some(t) => a::create_string(Some(t)),
        None => a::create_string_reference(None),
    };
    let patch = a::create_object();
    a::add_item_to_object(patch, Some(b"op"), text(op));
    a::add_item_to_object(patch, Some(b"path"), text(path));
    if let Some(f) = from {
        a::add_item_to_object(patch, Some(b"from"), text(f));
    }
    if with_value {
        a::add_item_to_object(patch, Some(b"value"), a::create_number(1.0));
    }
    let arr = a::create_array();
    a::add_item_to_array(arr, patch);
    arr
}

/// D22, the Rust-only results for the cases C dereferences NULL (the review pins them so a refactor cannot move
/// them): NULL `op` 3; NULL `path` 13 for remove/replace and 9 for add/move/copy; `move` with NULL `from` 5;
/// a `move` with a NULL `path` loses its member as for any unresolvable `path`.
#[test]
fn d22_null_valuestring_results_are_pinned() {
    let _g = lock();
    if a::C_LAYOUT {
        skipped("D22 UB cases: C crashes");
        return;
    }
    let doc = b"{\"a\":1,\"b\":[1,2]}";
    let run = |patches: Item| -> (i32, Vec<u8>) {
        let d = pb(doc);
        let st = a::apply_patches(d, patches);
        let out = a::print_unformatted(d).unwrap().bytes().to_vec();
        a::delete(d);
        a::delete(patches);
        (st, out)
    };
    for path in [&b"/c"[..], b""] {
        assert_eq!(
            run(patch_member(None, Some(path), None, true)).0,
            3,
            "NULL op, path {path:?}"
        );
    }
    for op in [&b"remove"[..], b"replace"] {
        assert_eq!(
            run(patch_member(Some(op), None, None, true)).0,
            13,
            "NULL path, {op:?}"
        );
    }
    assert_eq!(
        run(patch_member(Some(b"add"), None, None, true)).0,
        9,
        "NULL path, add"
    );
    for op in [&b"move"[..], b"copy"] {
        assert_eq!(
            run(patch_member(Some(op), None, Some(Some(b"/a")), false)).0,
            9,
            "NULL path, {op:?}"
        );
    }
    assert_eq!(
        run(patch_member(Some(b"move"), Some(b"/c"), Some(None), false)).0,
        5,
        "NULL from, move"
    );
    assert_eq!(
        run(patch_member(Some(b"copy"), Some(b"/c"), Some(None), false)).0,
        5,
        "NULL from, copy"
    );
    let (st, out) = run(patch_member(Some(b"move"), None, Some(Some(b"/a")), false));
    assert_eq!(st, 9);
    assert_eq!(
        out, b"{\"b\":[1,2]}",
        "the moved member is lost (cleanup deletes the value)"
    );
}
