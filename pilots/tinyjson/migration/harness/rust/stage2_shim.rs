//! Stage 2 validation scaffolding: the RULEBOOK section 6 API on top of the Stage 1 crate (`tinyjson` as `tj`).
//!
//! Installed by `run.py --stage 2` as `tests/adapter/stage2_shim.rs`. It exists only to prove that the Stage 2 Judge
//! is green on a correct port and red on a mutant before the Stage 2 crate exists. It is deleted when the Stage 2
//! crate lands (the adapter then says `use tinyjson as tj;`).
//!
//! Design of the borrowed getters (`get_string`, `get_object`, `get_array`, `get_key`, `get_index`): `Json` is the
//! section 6 enum itself, not a newtype over `tj::json`. The Stage 1 `json` keeps its payload behind a private
//! `void*` and its getters return clones, so no `&` into it can be handed out soundly. Instead every method turns
//! `self` into a Stage 1 value (`to_tj`), calls the Stage 1 method, and turns the result back (`from_tj`) or, for a
//! borrowing getter, takes the `Ok`/`Err` decision from Stage 1 and returns a reference into the enum payload that
//! `&self` owns. Behaviour (errors, equality, text) always comes from Stage 1; only the data layout is the enum's.
//! Cost: methods whose Stage 1 behaviour depends only on the type, the scalar payload, or the number of children
//! and the keys use `shallow` (one level, children replaced by null); only `to_string`, `==` and the parser convert
//! the whole value. No `unsafe`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use tinyjson as tj;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    RuntimeError,
    InvalidArgument,
    OutOfRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error {
    pub kind: ErrorKind,
    pub what: Vec<u8>,
}

fn conv_err(e: tj::Error) -> Error {
    let kind = match e.kind {
        tj::ErrorKind::RuntimeError => ErrorKind::RuntimeError,
        tj::ErrorKind::InvalidArgument => ErrorKind::InvalidArgument,
        tj::ErrorKind::OutOfRange => ErrorKind::OutOfRange,
    };
    Error { kind, what: e.what }
}

fn cv<T>(v: tj::Result<T>) -> Result<T> {
    v.map_err(conv_err)
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JsonType {
    String = 0,
    NumberInteger = 1,
    NumberDouble = 2,
    Array = 3,
    Boolean = 4,
    Object = 5,
    Null = 6,
    Invalid = 7,
}

pub type JsonObject = BTreeMap<Vec<u8>, Json>;
pub type JsonArray = Vec<Json>;

#[derive(Clone, Debug)]
pub enum Json {
    String(Vec<u8>),
    Integer(i64),
    Double(f64),
    Array(JsonArray),
    Boolean(bool),
    Object(JsonObject),
    Null,
}

// ---- conversion to and from the Stage 1 value ---------------------------------------------------------------------

fn to_tj(j: &Json) -> tj::json {
    match j {
        Json::String(s) => tj::json::from_string(s),
        Json::Integer(i) => tj::json::from_long_long(*i),
        Json::Double(d) => tj::json::from_double(*d),
        Json::Array(a) => tj::json::from_array(a.iter().map(to_tj).collect()),
        Json::Boolean(b) => tj::json::from_bool(*b),
        Json::Object(o) => tj::json::from_object(o.iter().map(|(k, v)| (k.clone(), to_tj(v))).collect()),
        Json::Null => tj::json::new(),
    }
}

/// One level of `j`: scalars as they are, containers with the same keys / length and `null` children. Stage 1
/// results that do not read a child value (type errors, `size`, `has_member`, index errors) are the same on it.
fn shallow(j: &Json) -> tj::json {
    match j {
        Json::Array(a) => tj::json::from_array(a.iter().map(|_| tj::json::new()).collect()),
        Json::Object(o) => tj::json::from_object(o.keys().map(|k| (k.clone(), tj::json::new())).collect()),
        other => to_tj(other),
    }
}

fn from_tj(t: &tj::json) -> Json {
    // Every getter below is called on the arm that `type_()` names, so none of them fails.
    match t.type_() as i32 {
        0 => Json::String(t.get_string().expect("string")),
        1 => Json::Integer(t.get_integer().expect("integer")),
        2 => Json::Double(t.get_double().expect("double")),
        3 => Json::Array(t.get_array().expect("array").iter().map(from_tj).collect()),
        4 => Json::Boolean(t.get_bool().expect("bool")),
        5 => Json::Object(t.get_object().expect("object").iter().map(|(k, v)| (k.clone(), from_tj(v))).collect()),
        6 => Json::Null,
        n => panic!("shim: Stage 1 value of type {n}"),
    }
}

impl Json {
    pub fn new() -> Json {
        Json::Null
    }
    pub fn from_string(v: &[u8]) -> Json {
        Json::String(v.to_vec())
    }
    pub fn from_double(v: f64) -> Json {
        Json::Double(v)
    }
    pub fn from_int(v: i32) -> Json {
        Json::Integer(v as i64)
    }
    pub fn from_long_long(v: i64) -> Json {
        Json::Integer(v)
    }
    pub fn from_bool(v: bool) -> Json {
        Json::Boolean(v)
    }
    pub fn from_array(v: JsonArray) -> Json {
        Json::Array(v)
    }
    pub fn from_object(v: JsonObject) -> Json {
        Json::Object(v)
    }
    pub fn json_type(&self) -> JsonType {
        match shallow(self).type_() as i32 {
            0 => JsonType::String,
            1 => JsonType::NumberInteger,
            2 => JsonType::NumberDouble,
            3 => JsonType::Array,
            4 => JsonType::Boolean,
            5 => JsonType::Object,
            6 => JsonType::Null,
            _ => JsonType::Invalid,
        }
    }
    pub fn type_name(&self) -> &'static str {
        shallow(self).type_name()
    }
    pub fn get_string(&self) -> Result<&[u8]> {
        cv(shallow(self).get_string())?;
        match self {
            Json::String(s) => Ok(s),
            _ => unreachable!("Stage 1 accepted a non-string"),
        }
    }
    pub fn get_integer(&self) -> Result<i64> {
        cv(shallow(self).get_integer())
    }
    pub fn get_double(&self) -> Result<f64> {
        cv(shallow(self).get_double())
    }
    pub fn get_bool(&self) -> Result<bool> {
        cv(shallow(self).get_bool())
    }
    pub fn get_null(&self) -> Result<()> {
        cv(shallow(self).get_null())
    }
    pub fn get_object(&self) -> Result<&JsonObject> {
        cv(shallow(self).get_object())?;
        match self {
            Json::Object(o) => Ok(o),
            _ => unreachable!("Stage 1 accepted a non-object"),
        }
    }
    pub fn get_array(&self) -> Result<&JsonArray> {
        cv(shallow(self).get_array())?;
        match self {
            Json::Array(a) => Ok(a),
            _ => unreachable!("Stage 1 accepted a non-array"),
        }
    }
    pub fn size(&self) -> Result<usize> {
        cv(shallow(self).size())
    }
    pub fn has_member(&self, name: &[u8]) -> Result<bool> {
        cv(shallow(self).has_member(name))
    }
    pub fn add_member(&mut self, name: &[u8], value: Json) -> Result<()> {
        cv(shallow(self).add_member(name, tj::json::new()))?;
        match self {
            Json::Object(o) => {
                o.insert(name.to_vec(), value);
                Ok(())
            }
            _ => unreachable!("Stage 1 accepted a non-object"),
        }
    }
    pub fn add_element(&mut self, elem: Json) -> Result<()> {
        cv(shallow(self).add_element(tj::json::new()))?;
        match self {
            Json::Array(a) => {
                a.push(elem);
                Ok(())
            }
            _ => unreachable!("Stage 1 accepted a non-array"),
        }
    }
    pub fn get_key(&self, key: &[u8]) -> Result<&Json> {
        cv(shallow(self).index_key(key).map(|_| ()))?;
        match self {
            Json::Object(o) => Ok(o.get(key).expect("Stage 1 found the key")),
            _ => unreachable!("Stage 1 accepted a non-object"),
        }
    }
    pub fn get_key_mut(&mut self, key: &[u8]) -> Result<&mut Json> {
        cv(shallow(self).index_key(key).map(|_| ()))?;
        match self {
            Json::Object(o) => Ok(o.get_mut(key).expect("Stage 1 found the key")),
            _ => unreachable!("Stage 1 accepted a non-object"),
        }
    }
    pub fn get_index(&self, index: i32) -> Result<&Json> {
        cv(shallow(self).index(index).map(|_| ()))?;
        match self {
            Json::Array(a) => Ok(&a[index as usize]),
            _ => unreachable!("Stage 1 accepted a non-array"),
        }
    }
    pub fn get_index_mut(&mut self, index: i32) -> Result<&mut Json> {
        cv(shallow(self).index(index).map(|_| ()))?;
        match self {
            Json::Array(a) => Ok(&mut a[index as usize]),
            _ => unreachable!("Stage 1 accepted a non-array"),
        }
    }
    pub fn to_string(&self) -> Vec<u8> {
        to_tj(self).to_string()
    }
}

impl Default for Json {
    fn default() -> Json {
        Json::Null
    }
}

impl PartialEq for Json {
    fn eq(&self, o: &Json) -> bool {
        to_tj(self).eq(&to_tj(o))
    }
}

impl From<&[u8]> for Json {
    fn from(v: &[u8]) -> Json {
        Json::from_string(v)
    }
}
impl From<Vec<u8>> for Json {
    fn from(v: Vec<u8>) -> Json {
        Json::String(v)
    }
}
impl From<f64> for Json {
    fn from(v: f64) -> Json {
        Json::from_double(v)
    }
}
impl From<i32> for Json {
    fn from(v: i32) -> Json {
        Json::from_int(v)
    }
}
impl From<i64> for Json {
    fn from(v: i64) -> Json {
        Json::from_long_long(v)
    }
}
impl From<bool> for Json {
    fn from(v: bool) -> Json {
        Json::from_bool(v)
    }
}
impl From<JsonArray> for Json {
    fn from(v: JsonArray) -> Json {
        Json::Array(v)
    }
}
impl From<JsonObject> for Json {
    fn from(v: JsonObject) -> Json {
        Json::Object(v)
    }
}

impl TryFrom<&Json> for Vec<u8> {
    type Error = Error;
    fn try_from(j: &Json) -> Result<Vec<u8>> {
        cv(to_tj(j).operator_string())
    }
}
impl TryFrom<&Json> for f64 {
    type Error = Error;
    fn try_from(j: &Json) -> Result<f64> {
        cv(to_tj(j).operator_double())
    }
}
impl TryFrom<&Json> for i64 {
    type Error = Error;
    fn try_from(j: &Json) -> Result<i64> {
        cv(to_tj(j).operator_long_long())
    }
}
impl TryFrom<&Json> for bool {
    type Error = Error;
    fn try_from(j: &Json) -> Result<bool> {
        cv(to_tj(j).operator_bool())
    }
}

// ---- streams and parser -------------------------------------------------------------------------------------------

pub struct U32Stream(tj::u32_sstream);

impl U32Stream {
    pub const EOF: u32 = u32::MAX;
    pub fn new(s: &[u32]) -> U32Stream {
        U32Stream(tj::u32_sstream::new(s))
    }
    pub fn peek(&mut self) -> u32 {
        self.0.peek()
    }
    pub fn get(&mut self) -> u32 {
        self.0.get()
    }
    pub fn tellg(&self) -> usize {
        self.0.tellg()
    }
}

pub struct Parser;

fn pj(v: tj::Result<tj::json>) -> Result<Json> {
    v.map(|t| from_tj(&t)).map_err(conv_err)
}

impl Parser {
    pub fn parse(s: &[u8]) -> Result<Json> {
        pj(tj::parser::parse(s))
    }
    pub fn parse_value(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_value(&mut strm.0))
    }
    pub fn parse_object(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_object(&mut strm.0))
    }
    pub fn parse_array(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_array(&mut strm.0))
    }
    pub fn parse_bool(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_bool(&mut strm.0))
    }
    pub fn parse_null(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_null(&mut strm.0))
    }
    pub fn parse_string(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_string(&mut strm.0))
    }
    pub fn parse_number(strm: &mut U32Stream) -> Result<Json> {
        pj(tj::parser::parse_number(&mut strm.0))
    }
    pub fn parse_member(strm: &mut U32Stream) -> Result<Vec<u8>> {
        cv(tj::parser::parse_member(&mut strm.0))
    }
    pub fn parse_hex(strm: &mut U32Stream) -> Result<u32> {
        cv(tj::parser::parse_hex(&mut strm.0))
    }
    pub fn escape_char(strm: &mut U32Stream) -> Result<u32> {
        cv(tj::parser::escape_char(&mut strm.0))
    }
    pub fn get_next_non_space(strm: &mut U32Stream) -> u32 {
        tj::parser::get_next_non_space(&mut strm.0)
    }
    pub fn peek_next_non_space(strm: &mut U32Stream) -> i32 {
        tj::parser::peek_next_non_space(&mut strm.0)
    }
    pub fn skip_char(strm: &mut U32Stream, expected: u32) -> Result<()> {
        cv(tj::parser::skip_char(&mut strm.0, expected))
    }
    pub fn skip_space(strm: &mut U32Stream) {
        tj::parser::skip_space(&mut strm.0)
    }
    pub fn to_bool(s: &[u8]) -> Result<bool> {
        cv(tj::parser::to_bool(s))
    }
    pub fn to_integer(s: &[u8]) -> Result<i64> {
        cv(tj::parser::to_integer(s))
    }
    pub fn to_double(s: &[u8]) -> Result<f64> {
        cv(tj::parser::to_double(s))
    }
}

pub fn trim(s: &[u8]) -> Vec<u8> {
    tj::trim(s)
}
pub fn u32_to_u8(s: &[u32]) -> Result<Vec<u8>> {
    cv(tj::U32ToU8(s))
}
pub fn u8_to_u32(s: &[u8]) -> Result<Vec<u32>> {
    cv(tj::U8ToU32(s))
}
