//! Stage 2 idiomatic port of `src/tinyjson.h` @ bcca77e.
//!
//! Zero `unsafe`, idiomatic Rust API per RULEBOOK section 6, same behaviour.
use std::collections::BTreeMap;

/// Error kind from exceptions.
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

/// Type discriminant (RULEBOOK §2 T8): explicit discriminants match C++ values.
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

/// Idiomatic `Json` value: an enum with embedded payloads (no `unsafe`).
#[derive(Clone, Debug, Default)]
pub enum Json {
    String(Vec<u8>),
    Integer(i64),
    Double(f64),
    Array(JsonArray),
    Boolean(bool),
    Object(JsonObject),
    #[default]
    Null,
}

impl Json {
    /// `json()`: construct a null value (also Default).
    pub fn new() -> Json {
        Json::Null
    }

    /// `json(std::string)`, `json(std::string&)`, `json(char[])`
    pub fn from_string(val: &[u8]) -> Json {
        Json::String(val.to_vec())
    }

    /// `json(double)`
    pub fn from_double(val: f64) -> Json {
        Json::Double(val)
    }

    /// `json(int)`: stored as `Integer(i64)` (Q17).
    pub fn from_int(val: i32) -> Json {
        Json::Integer(val as i64)
    }

    /// `json(long long)`
    pub fn from_long_long(val: i64) -> Json {
        Json::Integer(val)
    }

    /// `json(bool)`
    pub fn from_bool(val: bool) -> Json {
        Json::Boolean(val)
    }

    /// `json(json_array&)`, `json(json_array&&)`
    pub fn from_array(val: JsonArray) -> Json {
        Json::Array(val)
    }

    /// `json(json_object&)`, `json(json_object&&)`
    pub fn from_object(val: JsonObject) -> Json {
        Json::Object(val)
    }

    /// `type()`: return the type discriminant.
    pub fn json_type(&self) -> JsonType {
        match self {
            Json::String(_) => JsonType::String,
            Json::Integer(_) => JsonType::NumberInteger,
            Json::Double(_) => JsonType::NumberDouble,
            Json::Array(_) => JsonType::Array,
            Json::Boolean(_) => JsonType::Boolean,
            Json::Object(_) => JsonType::Object,
            Json::Null => JsonType::Null,
        }
    }

    /// `type_name()`: returns "number" for both integer and double (Q20).
    pub fn type_name(&self) -> &'static str {
        match self {
            Json::String(_) => "string",
            Json::Integer(_) | Json::Double(_) => "number",
            Json::Array(_) => "array",
            Json::Boolean(_) => "boolean",
            Json::Object(_) => "object",
            Json::Null => "null",
        }
    }

    /// `get_string()`: return a borrow of the string bytes.
    pub fn get_string(&self) -> Result<&[u8]> {
        match self {
            Json::String(s) => Ok(s),
            _ => Err(mismatch_error(self.json_type(), JsonType::String)),
        }
    }

    /// `get_integer()`
    pub fn get_integer(&self) -> Result<i64> {
        match self {
            Json::Integer(n) => Ok(*n),
            _ => Err(mismatch_error(self.json_type(), JsonType::NumberInteger)),
        }
    }

    /// `get_double()`
    pub fn get_double(&self) -> Result<f64> {
        match self {
            Json::Double(n) => Ok(*n),
            _ => Err(mismatch_error(self.json_type(), JsonType::NumberDouble)),
        }
    }

    /// `get_bool()`
    pub fn get_bool(&self) -> Result<bool> {
        match self {
            Json::Boolean(b) => Ok(*b),
            _ => Err(mismatch_error(self.json_type(), JsonType::Boolean)),
        }
    }

    /// `get_null()`: returns `Ok(())` standing for `nullptr`.
    pub fn get_null(&self) -> Result<()> {
        match self {
            Json::Null => Ok(()),
            _ => Err(mismatch_error(self.json_type(), JsonType::Null)),
        }
    }

    /// `get_object()`: return a borrow of the object (C++ returned a copy, Q19).
    pub fn get_object(&self) -> Result<&JsonObject> {
        match self {
            Json::Object(o) => Ok(o),
            _ => Err(mismatch_error(self.json_type(), JsonType::Object)),
        }
    }

    /// `get_array()`: return a borrow of the array (C++ returned a copy, Q19).
    pub fn get_array(&self) -> Result<&JsonArray> {
        match self {
            Json::Array(a) => Ok(a),
            _ => Err(mismatch_error(self.json_type(), JsonType::Array)),
        }
    }

    /// `size()`: return the size of an array or object.
    pub fn size(&self) -> Result<usize> {
        match self {
            Json::Array(a) => Ok(a.len()),
            Json::Object(o) => Ok(o.len()),
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!(
                    "Unexpeced json type {}, expected array or object",
                    self.type_name()
                )
                .into_bytes(),
            }),
        }
    }

    /// `has_member()`: check if an object key exists.
    pub fn has_member(&self, member_name: &[u8]) -> Result<bool> {
        match self {
            Json::Object(o) => Ok(o.contains_key(member_name)),
            _ => Err(mismatch_error(self.json_type(), JsonType::Object)),
        }
    }

    /// `add_member()`: insert or overwrite a member (Q16).
    pub fn add_member(&mut self, member_name: &[u8], member_value: Json) -> Result<()> {
        match self {
            Json::Object(o) => {
                o.insert(member_name.to_vec(), member_value);
                Ok(())
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Object)),
        }
    }

    /// `add_element()`: append to an array.
    pub fn add_element(&mut self, elem: Json) -> Result<()> {
        match self {
            Json::Array(a) => {
                a.push(elem);
                Ok(())
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Array)),
        }
    }

    /// `operator[](const char*)`: return a mutable borrow of the member by key.
    pub fn get_key_mut(&mut self, key: &[u8]) -> Result<&mut Json> {
        match self {
            Json::Object(o) => {
                // BUG(port): Q5 `std::string(const char*)` stops at the first NUL
                // QUIRK(port): Q5
                let key = c_str(key);
                match o.get_mut(key) {
                    Some(member) => Ok(member),
                    None => Err(Error {
                        kind: ErrorKind::RuntimeError,
                        // "key " + std::string(key) + " not found.": stops at the first NUL
                        what: [b"key ".as_slice(), key, b" not found.".as_slice()].concat(),
                    }),
                }
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Object)),
        }
    }

    /// Read-only form of `operator[](const char*)`: return a borrow of the member by key.
    pub fn get_key(&self, key: &[u8]) -> Result<&Json> {
        match self {
            Json::Object(o) => {
                // BUG(port): Q5 `std::string(const char*)` stops at the first NUL
                // QUIRK(port): Q5
                let key = c_str(key);
                o.get(key).ok_or(Error {
                    kind: ErrorKind::RuntimeError,
                    what: [b"key ".as_slice(), key, b" not found.".as_slice()].concat(),
                })
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Object)),
        }
    }

    /// `operator[](int)`: return a mutable borrow of the element by index.
    pub fn get_index_mut(&mut self, index: i32) -> Result<&mut Json> {
        match self {
            Json::Array(a) => {
                if index < 0 || (index as usize) >= a.len() {
                    return Err(Error {
                        kind: ErrorKind::RuntimeError,
                        what: format!("index {} out of range.", index).into_bytes(),
                    });
                }
                Ok(&mut a[index as usize])
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Array)),
        }
    }

    /// `operator[](int)` const: return a borrow of the element by index.
    pub fn get_index(&self, index: i32) -> Result<&Json> {
        match self {
            Json::Array(a) => {
                if index < 0 || (index as usize) >= a.len() {
                    return Err(Error {
                        kind: ErrorKind::RuntimeError,
                        what: format!("index {} out of range.", index).into_bytes(),
                    });
                }
                Ok(&a[index as usize])
            }
            _ => Err(mismatch_error(self.json_type(), JsonType::Array)),
        }
    }

    /// `to_string()`: serialize the value to bytes.
    pub fn to_string(&self) -> Vec<u8> {
        match self {
            Json::String(s) => {
                let mut result = Vec::new();
                result.push(b'\"');
                // BUG(port): Q4 the string is written raw, quotes and control bytes are not escaped
                // QUIRK(port): Q4
                result.extend_from_slice(s);
                result.push(b'\"');
                result
            }
            Json::Integer(n) => n.to_string().into_bytes(),
            Json::Double(n) => format_double(*n).into_bytes(),
            Json::Boolean(b) => {
                if *b {
                    b"true".to_vec()
                } else {
                    b"false".to_vec()
                }
            }
            Json::Null => b"null".to_vec(),
            Json::Array(_) => array_to_string(self),
            Json::Object(_) => object_to_string(self),
        }
    }
}

/// The error of a conversion operator: `"cannot cast " + type_name() + " to json <what>"`.
fn cannot_cast(j: &Json, what: &str) -> Error {
    Error {
        kind: ErrorKind::RuntimeError,
        what: format!("cannot cast {} to json {}", j.type_name(), what).into_bytes(),
    }
}

/// `operator const std::string()` (CPP19)
impl TryFrom<&Json> for Vec<u8> {
    type Error = Error;
    fn try_from(j: &Json) -> Result<Vec<u8>> {
        match j {
            Json::String(s) => Ok(s.clone()),
            _ => Err(cannot_cast(j, "string")),
        }
    }
}

/// `operator const double()`
impl TryFrom<&Json> for f64 {
    type Error = Error;
    fn try_from(j: &Json) -> Result<f64> {
        match j {
            Json::Double(n) => Ok(*n),
            _ => Err(cannot_cast(j, "number")),
        }
    }
}

/// `operator const long long()`
impl TryFrom<&Json> for i64 {
    type Error = Error;
    fn try_from(j: &Json) -> Result<i64> {
        match j {
            Json::Integer(n) => Ok(*n),
            _ => Err(cannot_cast(j, "number")),
        }
    }
}

/// `operator const bool()`
impl TryFrom<&Json> for bool {
    type Error = Error;
    fn try_from(j: &Json) -> Result<bool> {
        match j {
            Json::Boolean(b) => Ok(*b),
            _ => Err(cannot_cast(j, "boolean")),
        }
    }
}

/// The constructors of CPP10 as `From`.
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

impl PartialEq for Json {
    fn eq(&self, other: &Json) -> bool {
        match (self, other) {
            // Q1: a container compared with null is true (the fallthrough reaches the null case)
            // QUIRK(port): Q1
            (Json::Array(_), Json::Null) | (Json::Object(_), Json::Null) => true,
            (Json::Array(a), Json::Array(b)) => a == b,
            (Json::Object(a), Json::Object(b)) => a == b,
            (Json::String(a), Json::String(b)) => a == b,
            (Json::Boolean(a), Json::Boolean(b)) => a == b,
            (Json::Integer(a), Json::Integer(b)) => a == b,
            // D2: left integer, right double: compare the integer bit pattern as a double
            // DEVIATION(port): D2
            (Json::Integer(a), Json::Double(b)) => f64::from_bits(*a as u64) == *b,
            (Json::Double(a), Json::Double(b)) => a == b,
            (Json::Null, Json::Null) => true,
            // default: every UB pair of D1 is false in both stages
            // DEVIATION(port): D1
            _ => false,
        }
    }
}

/// `stoll(str, &pos)` (RULEBOOK §2 I9): parse a signed integer.
// DEVIATION(port): D8
fn stoll(s: &[u8]) -> Result<(i64, usize)> {
    let mut i = 0;
    while i < s.len() && scan_isspace(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut acc: i128 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        acc = acc * 10 + (s[i] - b'0') as i128;
        if acc > (1i128 << 80) {
            acc = 1i128 << 80;
        }
        i += 1;
    }
    if i == start {
        return Err(Error {
            kind: ErrorKind::InvalidArgument,
            what: b"stoll".to_vec(),
        });
    }
    let v = if neg { -acc } else { acc };
    if v < i64::MIN as i128 || v > i64::MAX as i128 {
        return Err(Error {
            kind: ErrorKind::OutOfRange,
            what: b"stoll".to_vec(),
        });
    }
    Ok((v as i64, i))
}

/// `stod(str, &pos)` (RULEBOOK §2 I8): parse a floating-point number.
// DEVIATION(port): D8
fn stod(s: &[u8]) -> Result<(f64, usize)> {
    let n = s.len();
    let mut i = 0;
    while i < n && scan_isspace(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < n && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let sign = |v: f64| if neg { -v } else { v };

    // inf / infinity
    if scan_starts_ci(s, i, b"inf") {
        let end = if scan_starts_ci(s, i, b"infinity") {
            i + 8
        } else {
            i + 3
        };
        return Ok((sign(f64::INFINITY), end));
    }
    // nan / nan(...)
    if scan_starts_ci(s, i, b"nan") {
        let mut end = i + 3;
        let mut payload: u64 = 0;
        if end < n && s[end] == b'(' {
            if let Some(rel) = s[end + 1..].iter().position(|&c| c == b')') {
                let content = &s[end + 1..end + 1 + rel];
                if let Some(p) = scan_nan_payload(content) {
                    payload = p & 0x000F_FFFF_FFFF_FFFF;
                }
                end = end + 1 + rel + 1;
            }
        }
        let mut bits = 0x7FF8_0000_0000_0000u64 | payload;
        if neg {
            bits |= 1u64 << 63;
        }
        return Ok((f64::from_bits(bits), end));
    }

    // hex float
    if i + 1 < n && s[i] == b'0' && (s[i + 1] == b'x' || s[i + 1] == b'X') {
        let mut j = i + 2;
        let mut digits: Vec<u8> = Vec::new();
        let mut n_frac = 0usize;
        while j < n && s[j].is_ascii_hexdigit() {
            // invariant: the loop condition is_ascii_hexdigit() makes to_digit(16) succeed
            digits.push((s[j] as char).to_digit(16).unwrap() as u8);
            j += 1;
        }
        let mut end_ok = !digits.is_empty();
        let mut k = j;
        if k < n && s[k] == b'.' {
            k += 1;
            let mut fd = 0usize;
            while k < n && s[k].is_ascii_hexdigit() {
                // invariant: the loop condition is_ascii_hexdigit() makes to_digit(16) succeed
                digits.push((s[k] as char).to_digit(16).unwrap() as u8);
                fd += 1;
                k += 1;
            }
            if !digits.is_empty() {
                n_frac = fd;
                j = k;
                end_ok = true;
            }
        }
        if end_ok {
            let mut exp: i64 = 0;
            if j < n && (s[j] == b'p' || s[j] == b'P') {
                let mut m = j + 1;
                let mut eneg = false;
                if m < n && (s[m] == b'+' || s[m] == b'-') {
                    eneg = s[m] == b'-';
                    m += 1;
                }
                if m < n && s[m].is_ascii_digit() {
                    let mut ev: i64 = 0;
                    while m < n && s[m].is_ascii_digit() {
                        ev = std::cmp::min(ev * 10 + (s[m] - b'0') as i64, 1_000_000);
                        m += 1;
                    }
                    exp = if eneg { -ev } else { ev };
                    j = m;
                }
            }
            return match scan_hex_value(&digits, n_frac, exp) {
                Ok(v) => Ok((sign(v), j)),
                Err(kind) => Err(Error {
                    kind,
                    what: b"stod".to_vec(),
                }),
            };
        }
        // otherwise only the leading "0" is a number (handled by the decimal scan below)
    }

    // decimal
    let start = i;
    let mut j = i;
    let mut nonzero = false;
    let mut ndig = 0usize;
    while j < n && s[j].is_ascii_digit() {
        nonzero |= s[j] != b'0';
        ndig += 1;
        j += 1;
    }
    if j < n && s[j] == b'.' {
        let mut k = j + 1;
        let mut fd = 0usize;
        while k < n && s[k].is_ascii_digit() {
            nonzero |= s[k] != b'0';
            fd += 1;
            k += 1;
        }
        if ndig + fd > 0 {
            ndig += fd;
            j = k;
        }
    }
    if ndig == 0 {
        return Err(Error {
            kind: ErrorKind::InvalidArgument,
            what: b"stod".to_vec(),
        });
    }
    if j < n && (s[j] == b'e' || s[j] == b'E') {
        let mut m = j + 1;
        if m < n && (s[m] == b'+' || s[m] == b'-') {
            m += 1;
        }
        if m < n && s[m].is_ascii_digit() {
            while m < n && s[m].is_ascii_digit() {
                m += 1;
            }
            j = m;
        }
    }
    let text = std::str::from_utf8(&s[start..j]).unwrap_or("0");
    let v: f64 = match text.parse::<f64>() {
        Ok(v) => v,
        Err(_) => {
            return Err(Error {
                kind: ErrorKind::InvalidArgument,
                what: b"stod".to_vec(),
            })
        }
    };
    if v.is_infinite() {
        return Err(Error {
            kind: ErrorKind::OutOfRange,
            what: b"stod".to_vec(),
        });
    }
    if nonzero && v.abs() < f64::MIN_POSITIVE {
        return Err(Error {
            kind: ErrorKind::OutOfRange,
            what: b"stod".to_vec(),
        });
    }
    Ok((sign(v), j))
}

/// The error of `check_type_mismatch`, for match arms whose Ok type is not `()`.
fn mismatch_error(t1: JsonType, t2: JsonType) -> Error {
    Error {
        kind: ErrorKind::RuntimeError,
        // the arguments are swapped and print enum integers, as the C++ does
        // QUIRK(port): Q3
        what: format!("expect type {}, but found type {}", t1 as i32, t2 as i32).into_bytes(),
    }
}

/// `trim(const std::string&&)` with the default whitespace `" \t\r\n"`.
// BUG(port): Q11 token trim removes only space, \t, \r and \n
// QUIRK(port): Q11
pub fn trim(s: &[u8]) -> Vec<u8> {
    const TRIM_WHITESPACE: &[u8] = b" \t\r\n";
    trim_impl(s, TRIM_WHITESPACE)
}

fn trim_impl(s: &[u8], whitespace: &[u8]) -> Vec<u8> {
    let mut start = 0;
    let mut end = s.len();

    while start < end && whitespace.contains(&s[start]) {
        start += 1;
    }

    while end > start && whitespace.contains(&s[end - 1]) {
        end -= 1;
    }

    s[start..end].to_vec()
}

/// Convert UTF-32 code points to UTF-8 bytes.
pub fn u32_to_u8(u32_str: &[u32]) -> Result<Vec<u8>> {
    let mut result = Vec::new();

    for &code_point in u32_str {
        match std::char::from_u32(code_point) {
            Some(ch) => {
                let mut buf = [0u8; 4];
                let len = ch.encode_utf8(&mut buf).len();
                result.extend_from_slice(&buf[..len]);
            }
            None => {
                // C++: catch(range_error) -> throw runtime_error("invalid utf32 string: " + ex.what())
                // DEVIATION(port): D4
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"invalid utf32 string: wstring_convert::to_bytes".to_vec(),
                });
            }
        }
    }

    Ok(result)
}

/// Convert UTF-8 bytes to UTF-32 code points.
pub fn u8_to_u32(u8_str: &[u8]) -> Result<Vec<u32>> {
    let mut result = Vec::new();

    let s = match std::str::from_utf8(u8_str) {
        Ok(s) => s,
        Err(e) => {
            if e.error_len().is_some() {
                // C++: catch(exception) -> throw runtime_error("invalid utf32 string: " + ex.what())
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"invalid utf32 string: wstring_convert::from_bytes".to_vec(),
                });
            }
            // CPP28: a valid but incomplete sequence at the very end is dropped without error
            // valid_up_to() is a valid UTF-8 prefix by construction, so this never falls back to ""
            std::str::from_utf8(&u8_str[..e.valid_up_to()]).unwrap_or_default()
        }
    };

    for ch in s.chars() {
        result.push(ch as u32);
    }

    Ok(result)
}

/// D5: emulates the libstdc++ `wbuffer_convert<codecvt_utf8<char32_t>>` input window rule.
fn wbuffer_convert_decode(input: &[u8]) -> Vec<u32> {
    const WINDOW: usize = 29;
    let mut out: Vec<u32> = Vec::new();
    let mut pos = 0usize;
    while pos < input.len() {
        let end = std::cmp::min(pos + WINDOW, input.len());
        let window = &input[pos..end];
        match std::str::from_utf8(window) {
            Ok(text) => {
                out.extend(text.chars().map(|c| c as u32));
                pos = end;
            }
            Err(e) => {
                if e.error_len().is_some() {
                    // ill-formed: every character of this window is dropped, nothing after is read
                    return out;
                }
                // valid prefix, then an incomplete sequence cut by the window end
                let valid = e.valid_up_to();
                // valid_up_to() is a valid UTF-8 prefix by construction, so this never falls back to ""
                let text = std::str::from_utf8(&window[..valid]).unwrap_or_default();
                out.extend(text.chars().map(|c| c as u32));
                if end == input.len() {
                    // incomplete sequence at the very end of the input: dropped
                    return out;
                }
                pos += valid;
            }
        }
    }
    out
}

/// Format a double using %f format (6 decimals, D9).
fn format_double(n: f64) -> String {
    // D9: NaN prints `nan` for both signs, then `inf` / `-inf`; the digits come from `{:.6}`
    // DEVIATION(port): D9
    if n.is_nan() {
        return "nan".to_string();
    }
    if n.is_infinite() {
        return if n < 0.0 {
            "-inf".to_string()
        } else {
            "inf".to_string()
        };
    }
    format!("{:.6}", n)
}

/// `std::string::c_str()` read as a C string: the bytes before the first NUL (Q5).
fn c_str(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == 0) {
        Some(n) => &s[..n],
        None => s,
    }
}

/// Object to string.
fn object_to_string(obj: &Json) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(b'{');

    let jobj = match obj {
        Json::Object(o) => o,
        _ => return result, // should never happen
    };

    let mut first = true;
    for (k, v) in jobj.iter() {
        if !first {
            result.push(b',');
        }
        first = false;

        result.push(b'"');
        // `.c_str()` stops at the first NUL
        // BUG(port): Q5 key printed through c_str
        // QUIRK(port): Q5
        // QUIRK(port): Q4
        result.extend_from_slice(c_str(k));
        result.push(b'"');
        result.extend_from_slice(b" : ");
        // BUG(port): Q5 nested result printed through c_str
        // QUIRK(port): Q5
        result.extend_from_slice(c_str(&v.to_string()));
    }

    result.push(b'}');
    result
}

/// Array to string.
fn array_to_string(arr: &Json) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(b'[');

    let jarray = match arr {
        Json::Array(a) => a,
        _ => return result, // should never happen
    };

    for (i, item) in jarray.iter().enumerate() {
        // BUG(port): Q5 nested result printed through c_str
        // QUIRK(port): Q5
        result.extend_from_slice(c_str(&item.to_string()));
        if i < jarray.len() - 1 {
            result.push(b',');
        }
    }

    result.push(b']');
    result
}

/// Helper: is a character ASCII space.
fn isspace32(c: u32) -> bool {
    matches!(c, 9 | 10 | 11 | 12 | 13 | 32) // \t \n \v \f \r space
}

/// Helper: is a character ASCII lowercase.
fn tolower32(c: u32) -> u32 {
    if c <= 0x7F {
        (c as u8).to_ascii_lowercase() as u32
    } else {
        c
    }
}

/// Helper: is a hex digit.
fn isxdigit32(c: u32) -> bool {
    c <= 0x7F && (c as u8).is_ascii_hexdigit()
}

/// C `isspace` in the C locale.
fn scan_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// Does `s[i..]` start with `word` (ASCII case-insensitive)?
fn scan_starts_ci(s: &[u8], i: usize, word: &[u8]) -> bool {
    s.len() >= i + word.len() && s[i..i + word.len()].eq_ignore_ascii_case(word)
}

/// `strtoull(content, base 0)` as glibc `nan(n-char-seq)` uses it.
fn scan_nan_payload(content: &[u8]) -> Option<u64> {
    let mut i = 0;
    while i < content.len() && scan_isspace(content[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < content.len() && (content[i] == b'+' || content[i] == b'-') {
        neg = content[i] == b'-';
        i += 1;
    }
    let mut base: u64 = 10;
    if i < content.len() && content[i] == b'0' {
        if i + 2 < content.len() && content[i + 1] == b'x' && content[i + 2].is_ascii_hexdigit() {
            base = 16;
            i += 2;
        } else {
            base = 8;
        }
    }
    let mut acc: u64 = 0;
    let mut any = false;
    while i < content.len() {
        let d = match content[i] {
            c @ b'0'..=b'9' => (c - b'0') as u64,
            c @ b'a'..=b'z' => (c - b'a') as u64 + 10,
            c @ b'A'..=b'Z' => (c - b'A') as u64 + 10,
            _ => break,
        };
        if d >= base {
            break;
        }
        acc = acc.wrapping_mul(base).wrapping_add(d);
        any = true;
        i += 1;
    }
    if !any {
        return if content.is_empty() { Some(0) } else { None };
    }
    if i != content.len() {
        return None;
    }
    Some(if neg { acc.wrapping_neg() } else { acc })
}

/// Exact, correctly rounded hex float.
fn scan_hex_value(digits: &[u8], n_frac: usize, exp: i64) -> std::result::Result<f64, ErrorKind> {
    let mut mant: u128 = 0;
    let mut kept = 0usize;
    let mut sticky = false;
    let mut e: i64 = exp - 4 * n_frac as i64;
    let mut started = false;
    for (idx, &d) in digits.iter().enumerate() {
        let _ = idx;
        if !started && d == 0 {
            continue;
        }
        started = true;
        if kept < 30 {
            mant = (mant << 4) | d as u128;
            kept += 1;
        } else {
            if d != 0 {
                sticky = true;
            }
            e += 4;
        }
    }
    if mant == 0 {
        return Ok(0.0);
    }
    mant <<= 8;
    e -= 8;
    if sticky {
        mant |= 1;
    }
    let bitlen = 128 - mant.leading_zeros() as i64;
    let top = e + bitlen - 1;
    let lsb = std::cmp::max(top - 52, -1074);
    let shift = lsb - e;
    let (mut q, inexact): (u128, bool) = if shift <= 0 {
        (mant << (-shift) as u32, false)
    } else if shift > 128 {
        (0, true)
    } else if shift == 128 {
        let half: u128 = 1u128 << 127;
        let rem = mant;
        let mut q = 0u128;
        if rem > half {
            q = 1;
        }
        (q, rem != 0)
    } else {
        let sh = shift as u32;
        let mut q = mant >> sh;
        let rem = mant & ((1u128 << sh) - 1);
        let half = 1u128 << (sh - 1);
        if rem > half || (rem == half && (q & 1) == 1) {
            q += 1;
        }
        (q, rem != 0)
    };
    let mut l = lsb;
    if q == 0 {
        return Err(ErrorKind::OutOfRange);
    }
    let mut bl = 128 - q.leading_zeros() as i64;
    if bl > 53 {
        q >>= 1;
        l += 1;
        bl -= 1;
    }
    if bl + l > 1024 {
        return Err(ErrorKind::OutOfRange);
    }
    if bl + l <= -1022 {
        if inexact {
            return Err(ErrorKind::OutOfRange);
        }
        return Ok(f64::from_bits(q as u64));
    }
    let expfield = (bl + l - 1 + 1023) as u64;
    let frac = ((q << (53 - bl) as u32) as u64) & ((1u64 << 52) - 1);
    Ok(f64::from_bits((expfield << 52) | frac))
}

/// `u32_sstream` / `u32_istream`: a stream of code points with a read position.
pub struct U32Stream {
    buf: Vec<u32>,
    pos: usize,
}

impl U32Stream {
    pub const EOF: u32 = u32::MAX;

    pub fn new(s: &[u32]) -> U32Stream {
        U32Stream {
            buf: s.to_vec(),
            pos: 0,
        }
    }

    pub fn peek(&mut self) -> u32 {
        if self.pos < self.buf.len() {
            self.buf[self.pos]
        } else {
            U32Stream::EOF
        }
    }

    pub fn get(&mut self) -> u32 {
        if self.pos < self.buf.len() {
            let ch = self.buf[self.pos];
            self.pos += 1;
            ch
        } else {
            U32Stream::EOF
        }
    }

    /// `tellg()` after `clear()`: the read position in code points.
    pub fn tellg(&self) -> usize {
        self.pos
    }
}

/// `class parser` (static functions only).
pub struct Parser;

impl Parser {
    /// `parse(const char*)`: `s` is the buffer as it is; reading stops at the first NUL (Q10).
    pub fn parse(s: &[u8]) -> Result<Json> {
        // Stop at first NUL byte
        let input = if let Some(nul_pos) = s.iter().position(|&b| b == 0) {
            &s[..nul_pos]
        } else {
            s
        };

        // wbuffer_convert window decoder, not u8_to_u32
        // DEVIATION(port): D5
        let u32_str = wbuffer_convert_decode(input);
        let mut strm = U32Stream::new(&u32_str);

        let first_char = Parser::peek_next_non_space(&mut strm);

        let ret_val = if first_char == (b'{' as i32) {
            Parser::parse_object(&mut strm)?
        } else if first_char == (b'[' as i32) {
            Parser::parse_array(&mut strm)?
        } else {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"invalid json format".to_vec(),
            });
        };

        // Expecting EOF
        if Parser::peek_next_non_space(&mut strm) != -1 {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"invalid json format".to_vec(),
            });
        }

        Ok(ret_val)
    }

    pub fn parse_value(strm: &mut U32Stream) -> Result<Json> {
        let c = Parser::peek_next_non_space(strm);
        // P2: `c` is a code point (or -1 for EOF), compared whole
        match c {
            0x22 => Parser::parse_string(strm),
            0x5B => Parser::parse_array(strm),
            0x30..=0x39 | 0x2D | 0x2E => Parser::parse_number(strm),
            0x7B => Parser::parse_object(strm),
            0x54 | 0x74 | 0x46 | 0x66 => Parser::parse_bool(strm),
            0x6E | 0x4E => Parser::parse_null(strm),
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"unexpected character".to_vec(),
            }),
        }
    }

    pub fn parse_object(strm: &mut U32Stream) -> Result<Json> {
        let mut return_val = Json::from_object(JsonObject::new());

        // skip the first '{' character
        Parser::skip_char(strm, b'{' as u32)?;

        let mut c = Parser::peek_next_non_space(strm);
        while c != -1 {
            if c == (b'"' as i32) {
                let member = Parser::parse_member(strm)?;
                Parser::skip_char(strm, b':' as u32)?;
                let val = Parser::parse_value(strm)?;
                return_val.add_member(&member, val)?;
            } else if c == (b'}' as i32) {
                break;
            } else if c == (b',' as i32) {
                Parser::get_next_non_space(strm);
            } else {
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"invalid object format".to_vec(),
                });
            }
            c = Parser::peek_next_non_space(strm);
        }

        Parser::skip_char(strm, b'}' as u32)?;
        Ok(return_val)
    }

    pub fn parse_member(strm: &mut U32Stream) -> Result<Vec<u8>> {
        let mut return_val = Vec::new();

        // go past the opening double quote
        Parser::skip_char(strm, b'"' as u32)?;

        let mut c = strm.peek();
        while c != U32Stream::EOF && c != (b'"' as u32) {
            if c == (b'\\' as u32) {
                let ch = Parser::escape_char(strm)?;
                return_val.push(ch);
            } else {
                let ch = strm.get();
                return_val.push(ch);
            }
            c = strm.peek();
        }

        // go past the closing double quote
        Parser::skip_char(strm, b'"' as u32)?;

        u32_to_u8(&return_val)
    }

    pub fn parse_array(strm: &mut U32Stream) -> Result<Json> {
        let mut vector_val = JsonArray::new();

        // Go past the opening '['
        Parser::skip_char(strm, b'[' as u32)?;
        let mut c = Parser::peek_next_non_space(strm);

        // Empty array
        if c == (b']' as i32) {
            Parser::skip_char(strm, b']' as u32)?;
            return Ok(Json::from_array(vector_val));
        }

        loop {
            let val = Parser::parse_value(strm)?;
            vector_val.push(val);
            c = Parser::peek_next_non_space(strm);

            if c == (b',' as i32) {
                Parser::skip_char(strm, b',' as u32)?;
            } else if c == (b']' as i32) || c == -1 {
                break;
            }
        }

        // skip the closing square bracket
        Parser::skip_char(strm, b']' as u32)?;

        Ok(Json::from_array(vector_val))
    }

    pub fn parse_bool(strm: &mut U32Stream) -> Result<Json> {
        let mut val_str = Vec::new();

        let mut c = strm.peek();
        while c != U32Stream::EOF && c != (b',' as u32) && c != (b'}' as u32) && c != (b']' as u32)
        {
            c = strm.get();
            let lower = tolower32(c);
            val_str.push(lower);
            c = strm.peek();
        }

        let bool_str = trim(&u32_to_u8(&val_str)?);
        let val_bool = Parser::to_bool(&bool_str)?;
        Ok(Json::from_bool(val_bool))
    }

    pub fn parse_null(strm: &mut U32Stream) -> Result<Json> {
        let mut val_str = Vec::new();

        let mut c = strm.peek();
        while c != U32Stream::EOF && c != (b',' as u32) && c != (b'}' as u32) && c != (b']' as u32)
        {
            c = strm.get();
            let lower = tolower32(c);
            val_str.push(lower);
            c = strm.peek();
        }

        let null_str = trim(&u32_to_u8(&val_str)?);

        if null_str == b"null" {
            Ok(Json::new())
        } else {
            Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"unexpected null string".to_vec(),
            })
        }
    }

    pub fn parse_string(strm: &mut U32Stream) -> Result<Json> {
        let mut string_val = Vec::new();

        // skip the open double quote
        Parser::skip_char(strm, b'"' as u32)?;

        let mut c = strm.peek();

        while c != U32Stream::EOF && c != (b'"' as u32) {
            if c == (b'\\' as u32) {
                let ch = Parser::escape_char(strm)?;
                string_val.push(ch);
            } else {
                let ch = strm.get();
                string_val.push(ch);
            }
            c = strm.peek();
        }

        // skip the closing double quote
        Parser::skip_char(strm, b'"' as u32)?;

        Ok(Json::from_string(&u32_to_u8(&string_val)?))
    }

    pub fn parse_number(strm: &mut U32Stream) -> Result<Json> {
        let mut num_str = Vec::new();

        let mut c = strm.peek();
        while c != U32Stream::EOF && c != (b',' as u32) && c != (b']' as u32) && c != (b'}' as u32)
        {
            c = strm.get();
            num_str.push(c);
            c = strm.peek();
        }

        let nums = trim(&u32_to_u8(&num_str)?);

        // try integer first, if not then double
        let is_integer = !nums.iter().any(|&b| matches!(b, b'.' | b'e' | b'E' | b'+'));

        if is_integer && nums.iter().all(|&b| b"0123456789-".contains(&b)) {
            let num = Parser::to_integer(&nums)?;
            Ok(Json::from_long_long(num))
        } else {
            let num = Parser::to_double(&nums)?;
            Ok(Json::from_double(num))
        }
    }

    pub fn to_bool(s: &[u8]) -> Result<bool> {
        // std::transform + std::tolower (C locale: ASCII only), then the two comparisons
        let lower: Vec<u8> = s.iter().map(|c| c.to_ascii_lowercase()).collect();
        match lower.as_slice() {
            b"true" => Ok(true),
            b"false" => Ok(false),
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"invalid boolean string".to_vec(),
            }),
        }
    }

    pub fn to_integer(s: &[u8]) -> Result<i64> {
        let (num, pos) = stoll(s)?;

        if pos != s.len() {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"Unexpected number(integer) format.".to_vec(),
            });
        }

        Ok(num)
    }

    pub fn to_double(s: &[u8]) -> Result<f64> {
        let (num, pos) = stod(s)?;

        if pos != s.len() {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"Unexpected number(double) format.".to_vec(),
            });
        }

        Ok(num)
    }

    pub fn escape_char(strm: &mut U32Stream) -> Result<u32> {
        Parser::skip_char(strm, b'\\' as u32)?;

        let c = strm.get();

        let result = match c {
            34 => 34,                        // '"'
            92 => 92,                        // '\\'
            47 => 47,                        // '/'
            98 => 8,                         // 'b' -> backspace
            102 => 12,                       // 'f' -> form feed
            110 => 10,                       // 'n' -> newline
            114 => 13,                       // 'r' -> carriage return
            116 => 9,                        // 't' -> tab
            117 => Parser::parse_hex(strm)?, // 'u' -> unicode
            _ => {
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"backslash is followed by invalid character".to_vec(),
                });
            }
        };

        Ok(result)
    }

    pub fn get_next_non_space(strm: &mut U32Stream) -> u32 {
        Parser::skip_space(strm);
        strm.get()
    }

    pub fn peek_next_non_space(strm: &mut U32Stream) -> i32 {
        Parser::skip_space(strm);
        let ch = strm.peek();
        if ch == U32Stream::EOF {
            -1
        } else {
            ch as i32
        }
    }

    pub fn skip_char(strm: &mut U32Stream, expected: u32) -> Result<()> {
        Parser::skip_space(strm);
        let next = strm.peek();

        if next != expected || next == U32Stream::EOF {
            // C++ builds the text in a char32_t stream and passes it through U32ToU8 (so a surrogate
            // or a value above U+10FFFF throws the D4 error instead)
            let mut msg: Vec<u32> = "expected char '".chars().map(|c| c as u32).collect();
            msg.push(expected);
            msg.extend("' not found".chars().map(|c| c as u32));
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: u32_to_u8(&msg)?,
            });
        }

        // discard the expected char by get()
        strm.get();
        Ok(())
    }

    pub fn skip_space(strm: &mut U32Stream) {
        let mut next = strm.peek();
        while next != U32Stream::EOF && isspace32(next) {
            strm.get();
            next = strm.peek();
        }
    }

    pub fn parse_hex(strm: &mut U32Stream) -> Result<u32> {
        // C++ parse_hex fails to parse hex digits (no char32_t numeric facets); the uninitialised result is
        // typically U+0001. We reproduce that deterministically; the 4 hex digits are still consumed.
        // DEVIATION(port): D3
        for _ in 0..4 {
            let c = strm.get();
            if c == U32Stream::EOF || !isxdigit32(c) {
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"not hex number".to_vec(),
                });
            }
        }
        // Return the modal uninitialized value from the Reference build
        Ok(0x0001)
    }
}

// PORT STATUS: stage=2 confidence=high todos=0 deviations=D1,D2,D3,D4,D5,D8,D9
