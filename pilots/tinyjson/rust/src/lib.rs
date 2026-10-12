//! Faithful port (Stage 1) of `src/tinyjson.h` @ bcca77e.
//!
//! Signatures follow RULEBOOK section 5 and are binding (P7); the Judge adapter is the only file that calls this
//! crate.
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
        json {
            _type: json_t::null,
            _value: std::ptr::null_mut(),
        }
    }
    /// `json(std::string)`, `json(std::string&)`, `json(char[])`
    pub fn from_string(val: &[u8]) -> json {
        let boxed = Box::new(val.to_vec());
        json {
            _type: json_t::string,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }
    /// `json(double)`
    pub fn from_double(val: f64) -> json {
        let boxed = Box::new(val);
        json {
            _type: json_t::number_double,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }
    /// `json(int)`
    pub fn from_int(val: i32) -> json {
        json::from_long_long(val as i64)
    }
    /// `json(long long)`
    pub fn from_long_long(val: i64) -> json {
        let boxed = Box::new(val);
        json {
            _type: json_t::number_integer,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }
    /// `json(bool)`
    pub fn from_bool(val: bool) -> json {
        let boxed = Box::new(val);
        json {
            _type: json_t::boolean,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }
    /// `json(json_array&)`, `json(json_array&&)`
    pub fn from_array(val: json_array) -> json {
        let boxed = Box::new(val);
        json {
            _type: json_t::array,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }
    /// `json(json_object&)`, `json(json_object&&)`
    pub fn from_object(val: json_object) -> json {
        let boxed = Box::new(val);
        json {
            _type: json_t::object,
            _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
        }
    }

    /// `type()` (`type` is a Rust keyword)
    pub fn type_(&self) -> json_t {
        self._type
    }
    pub fn type_name(&self) -> &'static str {
        match self._type {
            json_t::string => "string",
            json_t::number_integer | json_t::number_double => "number",
            json_t::array => "array",
            json_t::boolean => "boolean",
            json_t::object => "object",
            json_t::null => "null",
            json_t::invalid => "invalid",
        }
    }

    /// `operator=`
    pub fn assign(&mut self, other: &json) {
        // Drop the current value
        let _ = self.clone_and_cleanup();

        // Copy the other's value
        match other._type {
            json_t::string => {
                let s = unsafe { &*(other._value as *const Vec<u8>) };
                let boxed = Box::new(s.clone());
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::number_integer => {
                let n = unsafe { &*(other._value as *const i64) };
                let boxed = Box::new(*n);
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::number_double => {
                let n = unsafe { &*(other._value as *const f64) };
                let boxed = Box::new(*n);
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::boolean => {
                let b = unsafe { &*(other._value as *const bool) };
                let boxed = Box::new(*b);
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::array => {
                let a = unsafe { &*(other._value as *const json_array) };
                let boxed = Box::new(a.clone());
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::object => {
                let o = unsafe { &*(other._value as *const json_object) };
                let boxed = Box::new(o.clone());
                self._value = Box::into_raw(boxed) as *mut std::ffi::c_void;
            }
            json_t::null | json_t::invalid => {
                self._value = std::ptr::null_mut();
            }
        }
        self._type = other._type;
    }

    fn clone_and_cleanup(&mut self) {
        // Get the clone to clean up
        let _ = self.clone();
        // Reset to null
        self._type = json_t::null;
        self._value = std::ptr::null_mut();
    }

    /// `operator==` - the C++ switch fallthrough as an explicit chain over `(self._type, o._type)` (RULEBOOK C4, D1, D2).
    /// A case whose right type matches returns its payload comparison at once (true or false); no match falls to the
    /// next case in order `array, object, null, string, boolean, number_integer, number_double, default`.
    pub fn eq(&self, o: &json) -> bool {
        // QUIRK(port): Q1 - container vs null is true (the fallthrough reaches the null case), null vs container false
        if self._type == json_t::array && o._type == json_t::array {
            let a = unsafe { &*(self._value as *const json_array) };
            let b = unsafe { &*(o._value as *const json_array) };
            return a == b;
        } else if self._type == json_t::object && o._type == json_t::object {
            let a = unsafe { &*(self._value as *const json_object) };
            let b = unsafe { &*(o._value as *const json_object) };
            return a == b;
        } else if (self._type == json_t::array || self._type == json_t::object
            || self._type == json_t::null)
            && o._type == json_t::null
        {
            return true;
        } else if self._type == json_t::string && o._type == json_t::string {
            let a = unsafe { &*(self._value as *const Vec<u8>) };
            let b = unsafe { &*(o._value as *const Vec<u8>) };
            return a == b;
        } else if self._type == json_t::boolean && o._type == json_t::boolean {
            let a = unsafe { &*(self._value as *const bool) };
            let b = unsafe { &*(o._value as *const bool) };
            return a == b;
        } else if self._type == json_t::number_integer && o._type == json_t::number_integer {
            let a = unsafe { &*(self._value as *const i64) };
            let b = unsafe { &*(o._value as *const i64) };
            return a == b;
        } else if self._type == json_t::number_integer && o._type == json_t::number_double {
            // DEVIATION(port): D2 - left integer, right double: the integer bit pattern is compared as a double
            let a = unsafe { &*(self._value as *const i64) };
            let b = unsafe { &*(o._value as *const f64) };
            return f64::from_bits(*a as u64) == *b;
        } else if self._type == json_t::number_double && o._type == json_t::number_double {
            let a = unsafe { &*(self._value as *const f64) };
            let b = unsafe { &*(o._value as *const f64) };
            return a == b;
        }
        // default: (and every UB pair of D1) -> false
        false
    }

    /// `operator!=`
    pub fn ne(&self, o: &json) -> bool {
        !self.eq(o)
    }

    pub fn get_string(&self) -> Result<Vec<u8>> {
        check_type_mismatch(self._type, json_t::string)?;
        let s = unsafe { &*(self._value as *const Vec<u8>) };
        Ok(s.clone())
    }
    pub fn get_integer(&self) -> Result<i64> {
        check_type_mismatch(self._type, json_t::number_integer)?;
        let n = unsafe { &*(self._value as *const i64) };
        Ok(*n)
    }
    pub fn get_double(&self) -> Result<f64> {
        check_type_mismatch(self._type, json_t::number_double)?;
        let n = unsafe { &*(self._value as *const f64) };
        Ok(*n)
    }
    pub fn get_bool(&self) -> Result<bool> {
        check_type_mismatch(self._type, json_t::boolean)?;
        let b = unsafe { &*(self._value as *const bool) };
        Ok(*b)
    }
    /// `get_null()` returns `nullptr` on success.
    pub fn get_null(&self) -> Result<()> {
        check_type_mismatch(self._type, json_t::null)?;
        Ok(())
    }
    pub fn get_object(&self) -> Result<json_object> {
        check_type_mismatch(self._type, json_t::object)?;
        let o = unsafe { &*(self._value as *const json_object) };
        Ok(o.clone())
    }
    pub fn get_array(&self) -> Result<json_array> {
        check_type_mismatch(self._type, json_t::array)?;
        let a = unsafe { &*(self._value as *const json_array) };
        Ok(a.clone())
    }

    pub fn size(&self) -> Result<usize> {
        match self._type {
            json_t::array => {
                let a = unsafe { &*(self._value as *const json_array) };
                Ok(a.len())
            }
            json_t::object => {
                let o = unsafe { &*(self._value as *const json_object) };
                Ok(o.len())
            }
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
    pub fn has_member(&self, member_name: &[u8]) -> Result<bool> {
        check_type_mismatch(self._type, json_t::object)?;
        let o = unsafe { &*(self._value as *const json_object) };
        Ok(o.contains_key(member_name))
    }
    pub fn add_member(&mut self, member_name: &[u8], member_value: json) -> Result<()> {
        check_type_mismatch(self._type, json_t::object)?;
        let o = unsafe { &mut *(self._value as *mut json_object) };
        o.insert(member_name.to_vec(), member_value);
        Ok(())
    }
    pub fn add_element(&mut self, elem: json) -> Result<()> {
        check_type_mismatch(self._type, json_t::array)?;
        let a = unsafe { &mut *(self._value as *mut json_array) };
        a.push(elem);
        Ok(())
    }

    /// `operator[](const char*)`: `key` is the C string without its NUL.
    pub fn index_key(&mut self, key: &[u8]) -> Result<&mut json> {
        check_type_mismatch(self._type, json_t::object)?;
        let o = unsafe { &mut *(self._value as *mut json_object) };
        if !o.contains_key(key) {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                // "key " + std::string(key) + " not found.": `std::string(const char*)` stops at the first NUL
                what: [b"key ".as_slice(), c_str(key), b" not found.".as_slice()].concat(),
            });
        }
        Ok(o.get_mut(key).unwrap())
    }
    /// `operator[](int)`
    pub fn index(&mut self, index: i32) -> Result<&mut json> {
        check_type_mismatch(self._type, json_t::array)?;
        let a = unsafe { &mut *(self._value as *mut json_array) };
        if index < 0 || (index as usize) >= a.len() {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!("index {} out of range.", index).into_bytes(),
            });
        }
        Ok(&mut a[index as usize])
    }

    /// `operator const std::string()`
    pub fn operator_string(&self) -> Result<Vec<u8>> {
        match self._type {
            json_t::string => {
                let s = unsafe { &*(self._value as *const Vec<u8>) };
                Ok(s.clone())
            }
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!(
                    "cannot cast {} to json string",
                    self.type_name()
                )
                .into_bytes(),
            }),
        }
    }
    /// `operator const double()`
    pub fn operator_double(&self) -> Result<f64> {
        match self._type {
            json_t::number_double => {
                let n = unsafe { &*(self._value as *const f64) };
                Ok(*n)
            }
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!(
                    "cannot cast {} to json number",
                    self.type_name()
                )
                .into_bytes(),
            }),
        }
    }
    /// `operator const long long()`
    pub fn operator_long_long(&self) -> Result<i64> {
        match self._type {
            json_t::number_integer => {
                let n = unsafe { &*(self._value as *const i64) };
                Ok(*n)
            }
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!(
                    "cannot cast {} to json number",
                    self.type_name()
                )
                .into_bytes(),
            }),
        }
    }
    /// `operator const bool()`
    pub fn operator_bool(&self) -> Result<bool> {
        match self._type {
            json_t::boolean => {
                let b = unsafe { &*(self._value as *const bool) };
                Ok(*b)
            }
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: format!(
                    "cannot cast {} to json boolean",
                    self.type_name()
                )
                .into_bytes(),
            }),
        }
    }

    /// `to_string()`; the `invalid` arm of the header is unreachable (RULEBOOK section 7).
    pub fn to_string(&self) -> Vec<u8> {
        match self._type {
            json_t::object => object_to_string(self),
            json_t::array => array_to_string(self),
            json_t::string => {
                let mut result = Vec::new();
                result.push(b'\"');
                let s = self.get_string().unwrap();
                result.extend_from_slice(&s);
                result.push(b'\"');
                result
            }
            json_t::number_integer => {
                let n = self.get_integer().unwrap();
                n.to_string().into_bytes()
            }
            json_t::number_double => {
                let n = self.get_double().unwrap();
                format_double(n).into_bytes()
            }
            json_t::boolean => {
                if self.get_bool().unwrap() {
                    b"true".to_vec()
                } else {
                    b"false".to_vec()
                }
            }
            json_t::null => b"null".to_vec(),
            json_t::invalid => panic!("invalid json type"),
        }
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
        match self._type {
            json_t::string => {
                let s = unsafe { &*(self._value as *const Vec<u8>) };
                let boxed = Box::new(s.clone());
                json {
                    _type: json_t::string,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::number_integer => {
                let n = unsafe { &*(self._value as *const i64) };
                let boxed = Box::new(*n);
                json {
                    _type: json_t::number_integer,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::number_double => {
                let n = unsafe { &*(self._value as *const f64) };
                let boxed = Box::new(*n);
                json {
                    _type: json_t::number_double,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::boolean => {
                let b = unsafe { &*(self._value as *const bool) };
                let boxed = Box::new(*b);
                json {
                    _type: json_t::boolean,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::array => {
                let a = unsafe { &*(self._value as *const json_array) };
                let boxed = Box::new(a.clone());
                json {
                    _type: json_t::array,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::object => {
                let o = unsafe { &*(self._value as *const json_object) };
                let boxed = Box::new(o.clone());
                json {
                    _type: json_t::object,
                    _value: Box::into_raw(boxed) as *mut std::ffi::c_void,
                }
            }
            json_t::null | json_t::invalid => json {
                _type: self._type,
                _value: std::ptr::null_mut(),
            },
        }
    }
}

impl Drop for json {
    fn drop(&mut self) {
        match self._type {
            json_t::array => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut json_array);
                }
            }
            json_t::object => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut json_object);
                }
            }
            json_t::string => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut Vec<u8>);
                }
            }
            json_t::boolean => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut bool);
                }
            }
            json_t::number_double => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut f64);
                }
            }
            json_t::number_integer => {
                unsafe {
                    let _ = Box::from_raw(self._value as *mut i64);
                }
            }
            json_t::null | json_t::invalid => {
                // No cleanup needed
            }
        }
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
    buf: Vec<u32>,
    _pos: usize,
}

pub type u32_istream = u32_sstream;

impl u32_sstream {
    pub fn new(s: &[u32]) -> u32_sstream {
        u32_sstream { buf: s.to_vec(), _pos: 0 }
    }
    pub fn peek(&mut self) -> u32 {
        if self._pos < self.buf.len() {
            self.buf[self._pos]
        } else {
            u32::MAX // EOF
        }
    }
    pub fn get(&mut self) -> u32 {
        if self._pos < self.buf.len() {
            let ch = self.buf[self._pos];
            self._pos += 1;
            ch
        } else {
            u32::MAX // EOF
        }
    }
    /// `tellg()` after `clear()`: the read position in code points.
    pub fn tellg(&self) -> usize {
        self._pos
    }
}

/// Check type mismatch (RULEBOOK section 2 M2)
fn check_type_mismatch(t1: json_t, t2: json_t) -> Result<()> {
    if t1 != t2 {
        Err(Error {
            kind: ErrorKind::RuntimeError,
            what: format!(
                "expect type {}, but found type {}",
                t1 as i32,
                t2 as i32
            )
            .into_bytes(),
        })
    } else {
        Ok(())
    }
}

/// `trim(const std::string&&)` with the default whitespace `" \t\r\n"`.
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

/// Convert UTF-32 to UTF-8
pub fn U32ToU8(u32_str: &[u32]) -> Result<Vec<u8>> {
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
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"invalid utf32 string: wstring_convert::to_bytes".to_vec(),
                });
            }
        }
    }

    Ok(result)
}

/// Convert UTF-8 to UTF-32
pub fn U8ToU32(u8_str: &[u8]) -> Result<Vec<u32>> {
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
            std::str::from_utf8(&u8_str[..e.valid_up_to()]).unwrap()
        }
    };

    for ch in s.chars() {
        result.push(ch as u32);
    }

    Ok(result)
}

/// DEVIATION(port): D5 - emulates the libstdc++ `wbuffer_convert<codecvt_utf8<char32_t>>` input window rule.
/// Bytes are decoded in windows of at most 29 bytes. An ill-formed byte drops its whole window and ends the stream
/// without an error; a valid but incomplete sequence at the window end starts the next window; an incomplete sequence
/// at the very end of the input is dropped.
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
                let text = std::str::from_utf8(&window[..valid]).unwrap();
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

/// Format a double using %f format (6 decimals)
fn format_double(n: f64) -> String {
    // D9: NaN prints `nan` for both signs, then `inf` / `-inf`; the digits come from `{:.6}`
    if n.is_nan() {
        return "nan".to_string();
    }
    if n.is_infinite() {
        return if n < 0.0 { "-inf".to_string() } else { "inf".to_string() };
    }
    format!("{:.6}", n)
}

/// `std::string::c_str()` read as a C string: the bytes before the first NUL (Q5)
fn c_str(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == 0) {
        Some(n) => &s[..n],
        None => s,
    }
}

/// Object to string
fn object_to_string(obj: &json) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(b'{');

    let jobj = obj.get_object().unwrap();
    let mut first = true;
    for (k, v) in jobj.iter() {
        if !first {
            result.push(b',');
        }
        first = false;

        result.push(b'"');
        result.extend_from_slice(c_str(k)); // QUIRK(port): Q5 - `.c_str()` stops at the first NUL
        result.push(b'"');
        result.extend_from_slice(b" : ");
        result.extend_from_slice(c_str(&v.to_string())); // QUIRK(port): Q5
    }

    result.push(b'}');
    result
}

/// Array to string
fn array_to_string(arr: &json) -> Vec<u8> {
    let mut result = Vec::new();
    result.push(b'[');

    let jarray = arr.get_array().unwrap();
    for (i, item) in jarray.iter().enumerate() {
        result.extend_from_slice(c_str(&item.to_string())); // QUIRK(port): Q5
        if i < jarray.len() - 1 {
            result.push(b',');
        }
    }

    result.push(b']');
    result
}

/// Helper: is a character ASCII space
fn isspace32(c: u32) -> bool {
    matches!(c, 9 | 10 | 11 | 12 | 13 | 32) // \t \n \v \f \r space
}

/// Helper: is a character ASCII lowercase
fn tolower32(c: u32) -> u32 {
    if c <= 0x7F {
        (c as u8).to_ascii_lowercase() as u32
    } else {
        c
    }
}

/// Helper: is a hex digit
fn isxdigit32(c: u32) -> bool {
    c <= 0x7F && (c as u8).is_ascii_hexdigit()
}

/// C `isspace` in the C locale.
fn scan_isspace(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// Error value for the scanners.
fn scan_err(kind: ErrorKind, what: &[u8]) -> Error {
    Error { kind, what: what.to_vec() }
}

/// Does `s[i..]` start with `word` (ASCII case-insensitive)?
fn scan_starts_ci(s: &[u8], i: usize, word: &[u8]) -> bool {
    s.len() >= i + word.len() && s[i..i + word.len()].eq_ignore_ascii_case(word)
}

/// `strtoull(content, base 0)` as glibc `nan(n-char-seq)` uses it: `Some(value mod 2^64)` when it consumes the whole
/// content, else `None`.
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
        if i + 2 < content.len() + 0 && content[i + 1] == b'x' && content[i + 2].is_ascii_hexdigit() {
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
        // strtoull consumed nothing (empty content is the only accepted case: endp == str == end)
        return if content.is_empty() { Some(0) } else { None };
    }
    if i != content.len() {
        return None;
    }
    Some(if neg { acc.wrapping_neg() } else { acc })
}

/// Exact, correctly rounded hex float. `digits` are hex digit values, `frac_start` is the number of digits that stand
/// before the point, `exp` the binary exponent from `p`. Returns `Ok(abs value)`, or the error kind.
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

/// `std::stoll(str, &pos)` (`tinyjson.h:929-941`, RULEBOOK section 2 I9): the value and `pos`. Sonnet task.
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
        return Err(scan_err(ErrorKind::InvalidArgument, b"stoll"));
    }
    let v = if neg { -acc } else { acc };
    if v < i64::MIN as i128 || v > i64::MAX as i128 {
        return Err(scan_err(ErrorKind::OutOfRange, b"stoll"));
    }
    Ok((v as i64, i))
}

/// `std::stod(str, &pos)` (`tinyjson.h:943-954`, RULEBOOK section 2 I8): the value and `pos`. Sonnet task.
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
        let end = if scan_starts_ci(s, i, b"infinity") { i + 8 } else { i + 3 };
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
            digits.push((s[j] as char).to_digit(16).unwrap() as u8);
            j += 1;
        }
        let mut end_ok = !digits.is_empty();
        let mut k = j;
        if k < n && s[k] == b'.' {
            k += 1;
            let mut fd = 0usize;
            while k < n && s[k].is_ascii_hexdigit() {
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
                Err(kind) => Err(scan_err(kind, b"stod")),
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
        return Err(scan_err(ErrorKind::InvalidArgument, b"stod"));
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
        Err(_) => return Err(scan_err(ErrorKind::InvalidArgument, b"stod")),
    };
    if v.is_infinite() {
        return Err(scan_err(ErrorKind::OutOfRange, b"stod"));
    }
    if nonzero && v.abs() < f64::MIN_POSITIVE {
        return Err(scan_err(ErrorKind::OutOfRange, b"stod"));
    }
    Ok((sign(v), j))
}

/// `class parser` (static functions only).
pub struct parser;

impl parser {
    /// `parse(const char*)`: `s` is the buffer as it is; reading stops at the first NUL (Q10).
    pub fn parse(s: &[u8]) -> Result<json> {
        // Stop at first NUL byte
        let input = if let Some(nul_pos) = s.iter().position(|&b| b == 0) {
            &s[..nul_pos]
        } else {
            s
        };

        // DEVIATION(port): D5 - wbuffer_convert window decoder, not U8ToU32
        let u32_str = wbuffer_convert_decode(input);
        let mut strm = u32_sstream::new(&u32_str);

        let first_char = parser::peek_next_non_space(&mut strm);

        let ret_val = if first_char == '{' as i32 {
            parser::parse_object(&mut strm)?
        } else if first_char == '[' as i32 {
            parser::parse_array(&mut strm)?
        } else {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"invalid json format".to_vec(),
            });
        };

        // Expecting EOF
        if parser::peek_next_non_space(&mut strm) != -1 {
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"invalid json format".to_vec(),
            });
        }

        Ok(ret_val)
    }

    pub fn parse_value(strm: &mut u32_istream) -> Result<json> {
        let c = parser::peek_next_non_space(strm);
        match c as u8 {
            b'"' => parser::parse_string(strm),
            b'[' => parser::parse_array(strm),
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'-' | b'.' => parser::parse_number(strm),
            b'{' => parser::parse_object(strm),
            b'T' | b't' | b'F' | b'f' => parser::parse_bool(strm),
            b'n' | b'N' => parser::parse_null(strm),
            _ => Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"unexpected character".to_vec(),
            }),
        }
    }

    pub fn parse_object(strm: &mut u32_istream) -> Result<json> {
        let mut return_val = json::from_object(json_object::new());

        // skip the first '{' character
        parser::skip_char(strm, b'{' as u32)?;

        let mut c = parser::peek_next_non_space(strm);
        while c != -1 {
            if c == b'"' as i32 {
                let member = parser::parse_member(strm)?;
                parser::skip_char(strm, b':' as u32)?;
                let val = parser::parse_value(strm)?;
                return_val.add_member(&member, val)?;
            } else if c == b'}' as i32 {
                break;
            } else if c == b',' as i32 {
                parser::get_next_non_space(strm);
            } else {
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"invalid object format".to_vec(),
                });
            }
            c = parser::peek_next_non_space(strm);
        }

        parser::skip_char(strm, b'}' as u32)?;
        Ok(return_val)
    }

    pub fn parse_member(strm: &mut u32_istream) -> Result<Vec<u8>> {
        let mut returnVal = Vec::new();

        // go past the opening double quote
        parser::skip_char(strm, b'"' as u32)?;

        let mut c = strm.peek();
        while c != u32::MAX && c != b'"' as u32 {
            if c == b'\\' as u32 {
                let ch = parser::escape_char(strm)?;
                if let Some(ch_char) = std::char::from_u32(ch) {
                    let mut buf = [0u8; 4];
                    let len = ch_char.encode_utf8(&mut buf).len();
                    returnVal.extend_from_slice(&buf[..len]);
                }
            } else {
                let ch = strm.get();
                if let Some(ch_char) = std::char::from_u32(ch) {
                    let mut buf = [0u8; 4];
                    let len = ch_char.encode_utf8(&mut buf).len();
                    returnVal.extend_from_slice(&buf[..len]);
                }
            }
            c = strm.peek();
        }

        // go past the closing double quote
        parser::skip_char(strm, b'"' as u32)?;

        Ok(returnVal)
    }

    pub fn parse_array(strm: &mut u32_istream) -> Result<json> {
        let mut vector_val = json_array::new();

        // Go past the opening '['
        parser::skip_char(strm, b'[' as u32)?;
        let mut c = parser::peek_next_non_space(strm);

        // Empty array
        if c == b']' as i32 {
            parser::skip_char(strm, b']' as u32)?;
            return Ok(json::from_array(vector_val));
        }

        loop {
            let val = parser::parse_value(strm)?;
            vector_val.push(val);
            c = parser::peek_next_non_space(strm);

            if c == b',' as i32 {
                parser::skip_char(strm, b',' as u32)?;
            } else if c == b']' as i32 {
                break;
            } else if c == -1 {
                break;
            }
        }

        // skip the closing square bracket
        parser::skip_char(strm, b']' as u32)?;

        Ok(json::from_array(vector_val))
    }

    pub fn parse_bool(strm: &mut u32_istream) -> Result<json> {
        let mut val_str = Vec::new();

        let mut c = strm.peek();
        while c != u32::MAX && c != b',' as u32 && c != b'}' as u32 && c != b']' as u32 {
            c = strm.get();
            let lower = tolower32(c);
            if let Some(ch) = std::char::from_u32(lower) {
                let mut buf = [0u8; 4];
                let len = ch.encode_utf8(&mut buf).len();
                val_str.extend_from_slice(&buf[..len]);
            }
            c = strm.peek();
        }

        let bool_str = trim(&val_str);
        let val_bool = parser::to_bool(&bool_str)?;
        Ok(json::from_bool(val_bool))
    }

    pub fn parse_null(strm: &mut u32_istream) -> Result<json> {
        let mut val_str = Vec::new();

        let mut c = strm.peek();
        while c != u32::MAX && c != b',' as u32 && c != b'}' as u32 && c != b']' as u32 {
            c = strm.get();
            let lower = tolower32(c);
            if let Some(ch) = std::char::from_u32(lower) {
                let mut buf = [0u8; 4];
                let len = ch.encode_utf8(&mut buf).len();
                val_str.extend_from_slice(&buf[..len]);
            }
            c = strm.peek();
        }

        let null_str = trim(&val_str);

        if null_str == b"null" {
            Ok(json::new())
        } else {
            Err(Error {
                kind: ErrorKind::RuntimeError,
                what: b"unexpected null string".to_vec(),
            })
        }
    }

    pub fn parse_string(strm: &mut u32_istream) -> Result<json> {
        let mut string_val = Vec::new();

        // skip the open double quote
        parser::skip_char(strm, b'"' as u32)?;

        let mut c = strm.peek();

        while c != u32::MAX && c != b'"' as u32 {
            if c == b'\\' as u32 {
                let ch = parser::escape_char(strm)?;
                if let Some(ch_char) = std::char::from_u32(ch) {
                    let mut buf = [0u8; 4];
                    let len = ch_char.encode_utf8(&mut buf).len();
                    string_val.extend_from_slice(&buf[..len]);
                }
            } else {
                let ch = strm.get();
                if let Some(ch_char) = std::char::from_u32(ch) {
                    let mut buf = [0u8; 4];
                    let len = ch_char.encode_utf8(&mut buf).len();
                    string_val.extend_from_slice(&buf[..len]);
                }
            }
            c = strm.peek();
        }

        // skip the closing double quote
        parser::skip_char(strm, b'"' as u32)?;

        Ok(json::from_string(&string_val))
    }

    pub fn parse_number(strm: &mut u32_istream) -> Result<json> {
        let mut num_str = Vec::new();

        let mut c = strm.peek();
        while c != u32::MAX && c != b',' as u32 && c != b']' as u32 && c != b'}' as u32 {
            c = strm.get();
            if let Some(ch) = std::char::from_u32(c) {
                let mut buf = [0u8; 4];
                let len = ch.encode_utf8(&mut buf).len();
                num_str.extend_from_slice(&buf[..len]);
            }
            c = strm.peek();
        }

        let nums = trim(&num_str);

        // try integer first, if not then double
        let is_integer = !nums.iter().any(|&b| {
            matches!(b, b'.' | b'e' | b'E' | b'+')
        });

        if is_integer && nums.iter().all(|&b| b"0123456789-".contains(&b)) {
            let num = parser::to_integer(&nums)?;
            Ok(json::from_long_long(num))
        } else {
            let num = parser::to_double(&nums)?;
            Ok(json::from_double(num))
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

    pub fn escape_char(strm: &mut u32_istream) -> Result<u32> {
        parser::skip_char(strm, b'\\' as u32)?;

        let c = strm.get();

        let result = match c {
            34 => 34, // '"'
            92 => 92, // '\\'
            47 => 47, // '/'
            98 => 8, // 'b' -> backspace
            102 => 12, // 'f' -> form feed
            110 => 10, // 'n' -> newline
            114 => 13, // 'r' -> carriage return
            116 => 9, // 't' -> tab
            117 => parser::parse_hex(strm)?, // 'u' -> unicode
            _ => {
                return Err(Error {
                    kind: ErrorKind::RuntimeError,
                    what: b"backslash is followed by invalid character".to_vec(),
                });
            }
        };

        Ok(result)
    }

    pub fn get_next_non_space(strm: &mut u32_istream) -> u32 {
        parser::skip_space(strm);
        strm.get()
    }

    pub fn peek_next_non_space(strm: &mut u32_istream) -> i32 {
        parser::skip_space(strm);
        let ch = strm.peek();
        if ch == u32::MAX {
            -1
        } else {
            ch as i32
        }
    }

    pub fn skip_char(strm: &mut u32_istream, expected: u32) -> Result<()> {
        parser::skip_space(strm);
        let next = strm.peek();

        if next != expected || next == u32::MAX {
            let msg = format!(
                "expected char '{}' not found",
                if let Some(ch) = std::char::from_u32(expected) {
                    ch.to_string()
                } else {
                    format!("U+{:04X}", expected)
                }
            );
            return Err(Error {
                kind: ErrorKind::RuntimeError,
                what: msg.into_bytes(),
            });
        }

        // discard the expected char by get()
        strm.get();
        Ok(())
    }

    pub fn skip_space(strm: &mut u32_istream) {
        let mut next = strm.peek();
        while next != u32::MAX && isspace32(next) {
            strm.get();
            next = strm.peek();
        }
    }

    pub fn parse_hex(strm: &mut u32_istream) -> Result<u32> {
        // DEVIATION(port): D3 - C++ parse_hex fails to parse hex digits due to missing char32_t facets;
        // uninitialized memory typically gives U+0001. We reproduce this behavior deterministically.
        // The 4 hex digits are still consumed from the stream.
        for _ in 0..4 {
            let c = strm.get();
            if c == u32::MAX || !isxdigit32(c) {
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

// PORT STATUS: stage=1 confidence=high todos=0 deviations=D2,D3,D4,D5,D9
