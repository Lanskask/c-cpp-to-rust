//! Faithful port of jsmn. Derived from c2rust 0.22.1 output (all four macro
//! configurations merged); C ABI kept. `strict` = JSMN_STRICT,
//! `parent-links` = JSMN_PARENT_LINKS. See docs/DECISIONS.md 24.
#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_int, c_uint};

pub type size_t = usize;
pub type jsmntype_t = c_uint;
pub const JSMN_UNDEFINED: jsmntype_t = 0;
pub const JSMN_OBJECT: jsmntype_t = 1;
pub const JSMN_ARRAY: jsmntype_t = 2;
pub const JSMN_STRING: jsmntype_t = 4;
pub const JSMN_PRIMITIVE: jsmntype_t = 8;

pub type jsmnerr = c_int;
pub const JSMN_ERROR_NOMEM: jsmnerr = -1;
pub const JSMN_ERROR_INVAL: jsmnerr = -2;
pub const JSMN_ERROR_PART: jsmnerr = -3;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmntok_t {
    pub type_0: jsmntype_t,
    pub start: c_int,
    pub end: c_int,
    pub size: c_int,
    #[cfg(feature = "parent-links")]
    pub parent: c_int,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmn_parser {
    pub pos: c_uint,
    pub toknext: c_uint,
    pub toksuper: c_int,
}

unsafe fn jsmn_alloc_token(parser: *mut jsmn_parser, tokens: *mut jsmntok_t, num_tokens: size_t) -> *mut jsmntok_t {
    if (*parser).toknext as size_t >= num_tokens {
        return core::ptr::null_mut();
    }
    let idx = (*parser).toknext;
    (*parser).toknext = (*parser).toknext.wrapping_add(1);
    let tok = tokens.offset(idx as isize);
    (*tok).end = -1;
    (*tok).start = -1;
    (*tok).size = 0;
    #[cfg(feature = "parent-links")]
    {
        (*tok).parent = -1;
    }
    tok
}

unsafe fn jsmn_fill_token(token: *mut jsmntok_t, type_0: jsmntype_t, start: c_int, end: c_int) {
    (*token).type_0 = type_0;
    (*token).start = start;
    (*token).end = end;
    (*token).size = 0;
}

unsafe fn jsmn_parse_primitive(
    parser: *mut jsmn_parser,
    js: *const c_char,
    len: size_t,
    tokens: *mut jsmntok_t,
    num_tokens: size_t,
) -> c_int {
    let start = (*parser).pos as c_int;
    // `found` replaces the C `goto found`; in strict mode running off the end is an error.
    let mut found = false;
    while ((*parser).pos as size_t) < len && *js.offset((*parser).pos as isize) as c_int != 0 {
        match *js.offset((*parser).pos as isize) as u8 {
            #[cfg(not(feature = "strict"))]
            b':' => {
                found = true;
                break;
            }
            b'\t' | b'\r' | b'\n' | b' ' | b',' | b']' | b'}' => {
                found = true;
                break;
            }
            _ => {}
        }
        let c = *js.offset((*parser).pos as isize) as c_int;
        if !(32..127).contains(&c) {
            (*parser).pos = start as c_uint;
            return JSMN_ERROR_INVAL;
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }
    #[cfg(feature = "strict")]
    if !found {
        (*parser).pos = start as c_uint;
        return JSMN_ERROR_PART;
    }
    #[cfg(not(feature = "strict"))]
    let _ = found;

    if tokens.is_null() {
        (*parser).pos = (*parser).pos.wrapping_sub(1);
        return 0;
    }
    let token = jsmn_alloc_token(parser, tokens, num_tokens);
    if token.is_null() {
        (*parser).pos = start as c_uint;
        return JSMN_ERROR_NOMEM;
    }
    jsmn_fill_token(token, JSMN_PRIMITIVE, start, (*parser).pos as c_int);
    #[cfg(feature = "parent-links")]
    {
        (*token).parent = (*parser).toksuper;
    }
    (*parser).pos = (*parser).pos.wrapping_sub(1);
    0
}

unsafe fn jsmn_parse_string(
    parser: *mut jsmn_parser,
    js: *const c_char,
    len: size_t,
    tokens: *mut jsmntok_t,
    num_tokens: size_t,
) -> c_int {
    let start = (*parser).pos as c_int;
    (*parser).pos = (*parser).pos.wrapping_add(1);
    while ((*parser).pos as size_t) < len && *js.offset((*parser).pos as isize) as c_int != 0 {
        let c = *js.offset((*parser).pos as isize) as c_int;
        if c == b'"' as c_int {
            if tokens.is_null() {
                return 0;
            }
            let token = jsmn_alloc_token(parser, tokens, num_tokens);
            if token.is_null() {
                (*parser).pos = start as c_uint;
                return JSMN_ERROR_NOMEM;
            }
            jsmn_fill_token(token, JSMN_STRING, start + 1, (*parser).pos as c_int);
            #[cfg(feature = "parent-links")]
            {
                (*token).parent = (*parser).toksuper;
            }
            return 0;
        }
        if c == b'\\' as c_int && ((*parser).pos.wrapping_add(1) as size_t) < len {
            (*parser).pos = (*parser).pos.wrapping_add(1);
            match *js.offset((*parser).pos as isize) as u8 {
                b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {}
                b'u' => {
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    let mut i = 0;
                    while i < 4
                        && ((*parser).pos as size_t) < len
                        && *js.offset((*parser).pos as isize) as c_int != 0
                    {
                        let h = *js.offset((*parser).pos as isize) as u8;
                        if !h.is_ascii_hexdigit() {
                            (*parser).pos = start as c_uint;
                            return JSMN_ERROR_INVAL;
                        }
                        (*parser).pos = (*parser).pos.wrapping_add(1);
                        i += 1;
                    }
                    (*parser).pos = (*parser).pos.wrapping_sub(1);
                }
                _ => {
                    (*parser).pos = start as c_uint;
                    return JSMN_ERROR_INVAL;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }
    (*parser).pos = start as c_uint;
    JSMN_ERROR_PART
}

#[no_mangle]
pub unsafe extern "C" fn jsmn_parse(
    parser: *mut jsmn_parser,
    js: *const c_char,
    len: size_t,
    tokens: *mut jsmntok_t,
    num_tokens: c_uint,
) -> c_int {
    let mut r: c_int;
    let mut i: c_int;
    let mut token: *mut jsmntok_t;
    let mut count = (*parser).toknext as c_int;

    while ((*parser).pos as size_t) < len && *js.offset((*parser).pos as isize) as c_int != 0 {
        let c = *js.offset((*parser).pos as isize) as u8;
        match c {
            b'{' | b'[' => {
                count += 1;
                if tokens.is_null() {
                    // C: `break` out of the switch, then pos++
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    continue;
                }
                token = jsmn_alloc_token(parser, tokens, num_tokens as size_t);
                if token.is_null() {
                    return JSMN_ERROR_NOMEM;
                }
                if (*parser).toksuper != -1 {
                    let t = tokens.offset((*parser).toksuper as isize);
                    #[cfg(feature = "strict")]
                    if (*t).type_0 == JSMN_OBJECT {
                        return JSMN_ERROR_INVAL;
                    }
                    (*t).size += 1;
                    #[cfg(feature = "parent-links")]
                    {
                        (*token).parent = (*parser).toksuper;
                    }
                }
                (*token).type_0 = if c == b'{' { JSMN_OBJECT } else { JSMN_ARRAY };
                (*token).start = (*parser).pos as c_int;
                (*parser).toksuper = (*parser).toknext.wrapping_sub(1) as c_int;
            }
            b'}' | b']' => {
                if !tokens.is_null() {
                    let type_0 = if c == b'}' { JSMN_OBJECT } else { JSMN_ARRAY };
                    #[cfg(feature = "parent-links")]
                    {
                        if (*parser).toknext < 1 {
                            return JSMN_ERROR_INVAL;
                        }
                        token = tokens.offset((*parser).toknext.wrapping_sub(1) as isize);
                        loop {
                            if (*token).start != -1 && (*token).end == -1 {
                                if (*token).type_0 != type_0 {
                                    return JSMN_ERROR_INVAL;
                                }
                                (*token).end = (*parser).pos.wrapping_add(1) as c_int;
                                (*parser).toksuper = (*token).parent;
                                break;
                            }
                            if (*token).parent == -1 {
                                if (*token).type_0 != type_0 || (*parser).toksuper == -1 {
                                    return JSMN_ERROR_INVAL;
                                }
                                break;
                            }
                            token = tokens.offset((*token).parent as isize);
                        }
                    }
                    #[cfg(not(feature = "parent-links"))]
                    {
                        i = (*parser).toknext.wrapping_sub(1) as c_int;
                        while i >= 0 {
                            token = tokens.offset(i as isize);
                            if (*token).start != -1 && (*token).end == -1 {
                                if (*token).type_0 != type_0 {
                                    return JSMN_ERROR_INVAL;
                                }
                                (*parser).toksuper = -1;
                                (*token).end = (*parser).pos.wrapping_add(1) as c_int;
                                break;
                            }
                            i -= 1;
                        }
                        if i == -1 {
                            return JSMN_ERROR_INVAL;
                        }
                        while i >= 0 {
                            token = tokens.offset(i as isize);
                            if (*token).start != -1 && (*token).end == -1 {
                                (*parser).toksuper = i;
                                break;
                            }
                            i -= 1;
                        }
                    }
                }
            }
            b'"' => {
                r = jsmn_parse_string(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -1 && !tokens.is_null() {
                    (*tokens.offset((*parser).toksuper as isize)).size += 1;
                }
            }
            b'\t' | b'\r' | b'\n' | b' ' => {}
            b':' => {
                (*parser).toksuper = (*parser).toknext.wrapping_sub(1) as c_int;
            }
            b',' => {
                if !tokens.is_null()
                    && (*parser).toksuper != -1
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 != JSMN_ARRAY
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 != JSMN_OBJECT
                {
                    #[cfg(feature = "parent-links")]
                    {
                        (*parser).toksuper = (*tokens.offset((*parser).toksuper as isize)).parent;
                    }
                    #[cfg(not(feature = "parent-links"))]
                    {
                        i = (*parser).toknext.wrapping_sub(1) as c_int;
                        while i >= 0 {
                            let t = tokens.offset(i as isize);
                            if ((*t).type_0 == JSMN_ARRAY || (*t).type_0 == JSMN_OBJECT)
                                && (*t).start != -1
                                && (*t).end == -1
                            {
                                (*parser).toksuper = i;
                                break;
                            }
                            i -= 1;
                        }
                    }
                }
            }
            #[cfg(feature = "strict")]
            b'-' | b'0'..=b'9' | b't' | b'f' | b'n' => {
                // In strict mode primitives must not be keys of the object.
                if !tokens.is_null() && (*parser).toksuper != -1 {
                    let t = tokens.offset((*parser).toksuper as isize);
                    if (*t).type_0 == JSMN_OBJECT || ((*t).type_0 == JSMN_STRING && (*t).size != 0) {
                        return JSMN_ERROR_INVAL;
                    }
                }
                r = jsmn_parse_primitive(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -1 && !tokens.is_null() {
                    (*tokens.offset((*parser).toksuper as isize)).size += 1;
                }
            }
            #[cfg(not(feature = "strict"))]
            _ => {
                r = jsmn_parse_primitive(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -1 && !tokens.is_null() {
                    (*tokens.offset((*parser).toksuper as isize)).size += 1;
                }
            }
            #[cfg(feature = "strict")]
            _ => return JSMN_ERROR_INVAL,
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }

    if !tokens.is_null() {
        i = (*parser).toknext.wrapping_sub(1) as c_int;
        while i >= 0 {
            let t = tokens.offset(i as isize);
            if (*t).start != -1 && (*t).end == -1 {
                return JSMN_ERROR_PART;
            }
            i -= 1;
        }
    }
    count
}

#[no_mangle]
pub unsafe extern "C" fn jsmn_init(parser: *mut jsmn_parser) {
    (*parser).pos = 0;
    (*parser).toknext = 0;
    (*parser).toksuper = -1;
}
