//! The adapter: the only file that calls the tinyjson crate (judge.md section 5).
//!
//! Installed by `migration/harness/run.py` as `<crate>/tests/adapter/mod.rs`. `examples/dump.rs` and
//! `tests/upstream.rs` use nothing from the crate except the plain types and free functions below, so the same
//! three files judge the scratch stub now and the real crate later.
//!
//! This is the STAGE 2 version: it calls the RULEBOOK section 6 API (`Json::from_string`, `Parser::parse_value`, ...).
//! The public surface is the same as `adapter.rs` (Stage 1), so `dump.rs` and `upstream.rs` do not change. Installed
//! by `run.py --stage 2`.
//!
//! Plain types: `J` (a json value), `S` (a u32 stream), `AErr` (class + what() bytes), `R<T>`.
//! Sentinels as in C++: stream EOF is `EOF` (0xFFFFFFFF), `peek_next_non_space` returns -1 at EOF.
#![allow(dead_code)]

// Stage 2 validation: the section 6 API comes from a shim over the Stage 1 crate. When the Stage 2 crate exists,
// these two lines become `use tinyjson as tj;` and `stage2_shim.rs` is deleted.
mod stage2_shim;
use stage2_shim as tj;

/// A json value. The driver and the tests only move it around through the functions below.
pub type J = tj::Json;
/// `json_object` (keys are byte strings, ordered as `std::string` orders them).
pub type Obj = tj::JsonObject;
/// `json_array`
pub type Arr = tj::JsonArray;

/// An exception: `class` is the C++ class name the driver prints, `what` the exact `what()` bytes.
#[derive(Clone, Debug)]
pub struct AErr {
    pub class: &'static str,
    pub what: Vec<u8>,
}
pub type R<T> = Result<T, AErr>;

/// `std::char_traits<char32_t>::eof()` as the stream returns it.
pub const EOF: u32 = u32::MAX;

// json_t values
pub const T_STRING: i32 = 0;
pub const T_NUMBER_INTEGER: i32 = 1;
pub const T_NUMBER_DOUBLE: i32 = 2;
pub const T_ARRAY: i32 = 3;
pub const T_BOOLEAN: i32 = 4;
pub const T_OBJECT: i32 = 5;
pub const T_NULL: i32 = 6;
pub const T_INVALID: i32 = 7;

fn err(e: tj::Error) -> AErr {
    let class = match e.kind {
        tj::ErrorKind::RuntimeError => "runtime_error",
        tj::ErrorKind::InvalidArgument => "invalid_argument",
        tj::ErrorKind::OutOfRange => "out_of_range",
    };
    AErr {
        class,
        what: e.what,
    }
}

fn r<T>(v: tj::Result<T>) -> R<T> {
    v.map_err(err)
}

// ---- constructors and copies -------------------------------------------------------------------------------------

/// `json()`
pub fn j_null() -> J {
    J::new()
}
/// `json(std::string)`, `json(std::string&)`, `json(char[])` (the caller cuts a C string at its NUL)
pub fn j_str(s: &[u8]) -> J {
    J::from_string(s)
}
/// `json(int)`
pub fn j_int(v: i32) -> J {
    J::from_int(v)
}
/// `json(long long)`
pub fn j_ll(v: i64) -> J {
    J::from_long_long(v)
}
/// `json(double)`
pub fn j_dbl(v: f64) -> J {
    J::from_double(v)
}
/// `json(bool)`
pub fn j_bool(v: bool) -> J {
    J::from_bool(v)
}
/// `json(json_array&)` / `json(json_array&&)`
pub fn j_arr(v: Arr) -> J {
    J::from_array(v)
}
/// `json(json_object&)` / `json(json_object&&)`
pub fn j_obj(v: Obj) -> J {
    J::from_object(v)
}
/// copy constructor
pub fn copy(j: &J) -> J {
    j.clone()
}
/// `dst = src` (`operator=`)
pub fn assign(dst: &mut J, src: &J) {
    *dst = src.clone()
}
/// `a == b`
pub fn eq(a: &J, b: &J) -> bool {
    a == b
}
/// `a != b`
pub fn ne(a: &J, b: &J) -> bool {
    a != b
}

// ---- observers ---------------------------------------------------------------------------------------------------

pub fn type_code(j: &J) -> i32 {
    j.json_type() as i32
}
pub fn type_name(j: &J) -> String {
    j.type_name().to_string()
}
pub fn get_string(j: &J) -> R<Vec<u8>> {
    r(j.get_string().map(|v| v.to_vec()))
}
pub fn get_integer(j: &J) -> R<i64> {
    r(j.get_integer())
}
pub fn get_double(j: &J) -> R<f64> {
    r(j.get_double())
}
pub fn get_bool(j: &J) -> R<bool> {
    r(j.get_bool())
}
/// `get_null()`: `Ok(())` stands for `nullptr`.
pub fn get_null(j: &J) -> R<()> {
    r(j.get_null())
}
pub fn get_object(j: &J) -> R<Obj> {
    r(j.get_object().map(|v| v.clone()))
}
pub fn get_array(j: &J) -> R<Arr> {
    r(j.get_array().map(|v| v.clone()))
}
pub fn size(j: &J) -> R<usize> {
    r(j.size())
}
pub fn has_member(j: &J, name: &[u8]) -> R<bool> {
    r(j.has_member(name))
}
pub fn conv_string(j: &J) -> R<Vec<u8>> {
    r(Vec::<u8>::try_from(j))
}
pub fn conv_double(j: &J) -> R<f64> {
    r(f64::try_from(j))
}
pub fn conv_long_long(j: &J) -> R<i64> {
    r(i64::try_from(j))
}
pub fn conv_bool(j: &J) -> R<bool> {
    r(bool::try_from(j))
}
pub fn to_string(j: &J) -> Vec<u8> {
    j.to_string()
}

// ---- mutators ----------------------------------------------------------------------------------------------------

pub fn add_member(j: &mut J, name: &[u8], v: J) -> R<()> {
    r(j.add_member(name, v))
}
pub fn add_element(j: &mut J, v: J) -> R<()> {
    r(j.add_element(v))
}
/// `j[const char*]`: `key` is the C string without its NUL
pub fn index_key<'a>(j: &'a mut J, key: &[u8]) -> R<&'a mut J> {
    r(j.get_key_mut(key))
}
/// `j[int]`
pub fn index<'a>(j: &'a mut J, i: i32) -> R<&'a mut J> {
    r(j.get_index_mut(i))
}

// ---- parser, whole documents -------------------------------------------------------------------------------------

/// `parser::parse(const char*)`. The slice is passed as it is, with no NUL appended and none removed: the crate
/// must stop at the first NUL (Q10), as C++ does (judge.md section 5a, "input without a trailing NUL").
pub fn parse(s: &[u8]) -> R<J> {
    r(tj::Parser::parse(s))
}

// ---- u32 streams -------------------------------------------------------------------------------------------------

/// `u32_sstream`
pub struct S(tj::U32Stream);

impl S {
    pub fn new(src: &[u32]) -> S {
        S(tj::U32Stream::new(src))
    }
    /// the position the C++ driver reads with `clear(); tellg()`
    pub fn pos(&self) -> i64 {
        self.0.tellg() as i64
    }
    /// the next code point without reading it, `EOF` at the end (C++: `clear(); peek()`)
    pub fn peek(&mut self) -> u32 {
        self.0.peek()
    }
}

/// `U32ToU8`
pub fn u32_to_u8(s: &[u32]) -> R<Vec<u8>> {
    r(tj::u32_to_u8(s))
}
/// `U8ToU32`
pub fn u8_to_u32(s: &[u8]) -> R<Vec<u32>> {
    r(tj::u8_to_u32(s))
}
/// `trim(std::string)` with the default whitespace
pub fn trim(s: &[u8]) -> Vec<u8> {
    tj::trim(s)
}

pub fn parse_value(s: &mut S) -> R<J> {
    r(tj::Parser::parse_value(&mut s.0))
}
pub fn parse_object(s: &mut S) -> R<J> {
    r(tj::Parser::parse_object(&mut s.0))
}
pub fn parse_array(s: &mut S) -> R<J> {
    r(tj::Parser::parse_array(&mut s.0))
}
pub fn parse_string(s: &mut S) -> R<J> {
    r(tj::Parser::parse_string(&mut s.0))
}
pub fn parse_number(s: &mut S) -> R<J> {
    r(tj::Parser::parse_number(&mut s.0))
}
pub fn parse_bool(s: &mut S) -> R<J> {
    r(tj::Parser::parse_bool(&mut s.0))
}
pub fn parse_null(s: &mut S) -> R<J> {
    r(tj::Parser::parse_null(&mut s.0))
}
pub fn parse_member(s: &mut S) -> R<Vec<u8>> {
    r(tj::Parser::parse_member(&mut s.0))
}
pub fn escape_char(s: &mut S) -> R<u32> {
    r(tj::Parser::escape_char(&mut s.0))
}
pub fn parse_hex(s: &mut S) -> R<u32> {
    r(tj::Parser::parse_hex(&mut s.0))
}
pub fn get_next_non_space(s: &mut S) -> u32 {
    tj::Parser::get_next_non_space(&mut s.0)
}
/// returns -1 at EOF, as the C++ `int` does (D7)
pub fn peek_next_non_space(s: &mut S) -> i32 {
    tj::Parser::peek_next_non_space(&mut s.0)
}
pub fn skip_space(s: &mut S) {
    tj::Parser::skip_space(&mut s.0)
}
pub fn skip_char(s: &mut S, expected: u32) -> R<()> {
    r(tj::Parser::skip_char(&mut s.0, expected))
}

// ---- scalar helpers ----------------------------------------------------------------------------------------------

pub fn to_bool(s: &[u8]) -> R<bool> {
    r(tj::Parser::to_bool(s))
}
pub fn to_integer(s: &[u8]) -> R<i64> {
    r(tj::Parser::to_integer(s))
}
pub fn to_double(s: &[u8]) -> R<f64> {
    r(tj::Parser::to_double(s))
}
