//! Faithful port (Stage 1) of `src/tinyjson.h` @ bcca77e.
//!
//! SKELETON written by the governor (ticket 04): real signatures, `todo!()` bodies. Signatures follow RULEBOOK
//! section 5 and are binding (P7); the Judge adapter is the only file that calls this crate.
#![allow(non_camel_case_types, non_snake_case, unused_variables)]

use std::collections::BTreeMap;

/// `enum json_t` (values 0..7 as in the header).
#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum json_t {
    string = 0,
    number_integer = 1,
    number_double = 2,
    array = 3,
    boolean = 4,
    object = 5,
    null = 6,
    invalid = 7,
}

/// The C++ exception class (RULEBOOK section 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    RuntimeError,
    InvalidArgument,
    OutOfRange,
}

/// A thrown exception: class plus the exact `what()` bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub what: Vec<u8>,
}

pub type Result<T> = std::result::Result<T, Error>;

pub type json_object = BTreeMap<Vec<u8>, json>;
pub type json_array = Vec<json>;

/// `class json`.
#[derive(Debug)]
pub struct json {
    _type: json_t,
    _value: *mut std::ffi::c_void,
}

impl json {
    /// `json()`
    pub fn new() -> json {
        todo!()
    }
    /// `json(std::string)`, `json(std::string&)`, `json(char[])`
    pub fn from_string(val: &[u8]) -> json {
        todo!()
    }
    /// `json(double)`
    pub fn from_double(val: f64) -> json {
        todo!()
    }
    /// `json(int)`
    pub fn from_int(val: i32) -> json {
        todo!()
    }
    /// `json(long long)`
    pub fn from_long_long(val: i64) -> json {
        todo!()
    }
    /// `json(bool)`
    pub fn from_bool(val: bool) -> json {
        todo!()
    }
    /// `json(json_array&)`, `json(json_array&&)`
    pub fn from_array(val: json_array) -> json {
        todo!()
    }
    /// `json(json_object&)`, `json(json_object&&)`
    pub fn from_object(val: json_object) -> json {
        todo!()
    }

    /// `type()` (`type` is a Rust keyword)
    pub fn type_(&self) -> json_t {
        todo!()
    }
    pub fn type_name(&self) -> &'static str {
        todo!()
    }

    /// `operator=`
    pub fn assign(&mut self, other: &json) {
        todo!()
    }
    /// `operator==`
    pub fn eq(&self, o: &json) -> bool {
        todo!()
    }
    /// `operator!=`
    pub fn ne(&self, o: &json) -> bool {
        todo!()
    }

    pub fn get_string(&self) -> Result<Vec<u8>> {
        todo!()
    }
    pub fn get_integer(&self) -> Result<i64> {
        todo!()
    }
    pub fn get_double(&self) -> Result<f64> {
        todo!()
    }
    pub fn get_bool(&self) -> Result<bool> {
        todo!()
    }
    /// `get_null()` returns `nullptr` on success.
    pub fn get_null(&self) -> Result<()> {
        todo!()
    }
    pub fn get_object(&self) -> Result<json_object> {
        todo!()
    }
    pub fn get_array(&self) -> Result<json_array> {
        todo!()
    }

    pub fn size(&self) -> Result<usize> {
        todo!()
    }
    pub fn has_member(&self, member_name: &[u8]) -> Result<bool> {
        todo!()
    }
    pub fn add_member(&mut self, member_name: &[u8], member_value: json) -> Result<()> {
        todo!()
    }
    pub fn add_element(&mut self, elem: json) -> Result<()> {
        todo!()
    }

    /// `operator[](const char*)`: `key` is the C string without its NUL.
    pub fn index_key(&mut self, key: &[u8]) -> Result<&mut json> {
        todo!()
    }
    /// `operator[](int)`
    pub fn index(&mut self, index: i32) -> Result<&mut json> {
        todo!()
    }

    /// `operator const std::string()`
    pub fn operator_string(&self) -> Result<Vec<u8>> {
        todo!()
    }
    /// `operator const double()`
    pub fn operator_double(&self) -> Result<f64> {
        todo!()
    }
    /// `operator const long long()`
    pub fn operator_long_long(&self) -> Result<i64> {
        todo!()
    }
    /// `operator const bool()`
    pub fn operator_bool(&self) -> Result<bool> {
        todo!()
    }

    /// `to_string()`; the `invalid` arm of the header is unreachable (RULEBOOK section 7).
    pub fn to_string(&self) -> Vec<u8> {
        todo!()
    }
}

impl Default for json {
    fn default() -> json {
        json::new()
    }
}

/// copy constructor (`tinyjson.h:227-256`), hand-written as CPP14 asks; the adapter's `copy` calls it.
impl Clone for json {
    fn clone(&self) -> json {
        todo!()
    }
}

impl PartialEq for json {
    fn eq(&self, other: &json) -> bool {
        json::eq(self, other)
    }
}

/// `u32_sstream` / `u32_istream`: a stream of code points with a read position. `EOF` is `u32::MAX`.
#[derive(Clone, Debug)]
pub struct u32_sstream {
    _pos: usize,
}

pub type u32_istream = u32_sstream;

impl u32_sstream {
    pub fn new(s: &[u32]) -> u32_sstream {
        todo!()
    }
    pub fn peek(&mut self) -> u32 {
        todo!()
    }
    pub fn get(&mut self) -> u32 {
        todo!()
    }
    /// `tellg()` after `clear()`: the read position in code points.
    pub fn tellg(&self) -> usize {
        todo!()
    }
}

/// `trim(const std::string&&)` with the default whitespace `" \t\r\n"`.
pub fn trim(s: &[u8]) -> Vec<u8> {
    todo!()
}
pub fn U32ToU8(u32: &[u32]) -> Result<Vec<u8>> {
    todo!()
}
pub fn U8ToU32(u8: &[u8]) -> Result<Vec<u32>> {
    todo!()
}

/// `std::stoll(str, &pos)` (`tinyjson.h:929-941`, RULEBOOK section 2 I9): the value and `pos`. Sonnet task.
fn stoll(s: &[u8]) -> Result<(i64, usize)> {
    todo!()
}

/// `std::stod(str, &pos)` (`tinyjson.h:943-954`, RULEBOOK section 2 I8): the value and `pos`. Sonnet task.
fn stod(s: &[u8]) -> Result<(f64, usize)> {
    todo!()
}

/// `class parser` (static functions only).
pub struct parser;

impl parser {
    /// `parse(const char*)`: `s` is the buffer as it is; reading stops at the first NUL (Q10).
    pub fn parse(s: &[u8]) -> Result<json> {
        todo!()
    }
    pub fn parse_value(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_object(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_member(strm: &mut u32_istream) -> Result<Vec<u8>> {
        todo!()
    }
    pub fn parse_array(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_bool(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_null(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_string(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn parse_number(strm: &mut u32_istream) -> Result<json> {
        todo!()
    }
    pub fn to_bool(s: &[u8]) -> Result<bool> {
        todo!()
    }
    pub fn to_integer(s: &[u8]) -> Result<i64> {
        todo!()
    }
    pub fn to_double(s: &[u8]) -> Result<f64> {
        todo!()
    }
    pub fn escape_char(strm: &mut u32_istream) -> Result<u32> {
        todo!()
    }
    pub fn get_next_non_space(strm: &mut u32_istream) -> u32 {
        todo!()
    }
    pub fn peek_next_non_space(strm: &mut u32_istream) -> i32 {
        todo!()
    }
    pub fn skip_char(strm: &mut u32_istream, expected: u32) -> Result<()> {
        todo!()
    }
    pub fn skip_space(strm: &mut u32_istream) {
        todo!()
    }
    pub fn parse_hex(strm: &mut u32_istream) -> Result<u32> {
        todo!()
    }
}

// PORT STATUS: stage=1 confidence=low todos=all deviations=-
