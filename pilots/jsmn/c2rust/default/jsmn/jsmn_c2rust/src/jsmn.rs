pub type __darwin_size_t = usize;
pub type size_t = __darwin_size_t;
pub type jsmntype_t = ::core::ffi::c_uint;
pub const JSMN_PRIMITIVE: jsmntype_t = 8;
pub const JSMN_STRING: jsmntype_t = 4;
pub const JSMN_ARRAY: jsmntype_t = 2;
pub const JSMN_OBJECT: jsmntype_t = 1;
pub const JSMN_UNDEFINED: jsmntype_t = 0;
pub type jsmnerr = ::core::ffi::c_int;
pub const JSMN_ERROR_PART: jsmnerr = -3;
pub const JSMN_ERROR_INVAL: jsmnerr = -2;
pub const JSMN_ERROR_NOMEM: jsmnerr = -1;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmntok {
    pub type_0: jsmntype_t,
    pub start: ::core::ffi::c_int,
    pub end: ::core::ffi::c_int,
    pub size: ::core::ffi::c_int,
}
pub type jsmntok_t = jsmntok;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct jsmn_parser {
    pub pos: ::core::ffi::c_uint,
    pub toknext: ::core::ffi::c_uint,
    pub toksuper: ::core::ffi::c_int,
}
pub const __DARWIN_NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL: *mut ::core::ffi::c_void = __DARWIN_NULL;
unsafe extern "C" fn jsmn_alloc_token(
    mut parser: *mut jsmn_parser,
    mut tokens: *mut jsmntok_t,
    num_tokens: size_t,
) -> *mut jsmntok_t {
    let mut tok: *mut jsmntok_t = ::core::ptr::null_mut::<jsmntok_t>();
    if (*parser).toknext as size_t >= num_tokens {
        return ::core::ptr::null_mut::<jsmntok_t>();
    }
    let fresh2 = (*parser).toknext;
    (*parser).toknext = (*parser).toknext.wrapping_add(1);
    tok = tokens.offset(fresh2 as isize) as *mut jsmntok_t;
    (*tok).end = -(1 as ::core::ffi::c_int);
    (*tok).start = (*tok).end;
    (*tok).size = 0 as ::core::ffi::c_int;
    return tok;
}
unsafe extern "C" fn jsmn_fill_token(
    mut token: *mut jsmntok_t,
    type_0: jsmntype_t,
    start: ::core::ffi::c_int,
    end: ::core::ffi::c_int,
) {
    (*token).type_0 = type_0;
    (*token).start = start;
    (*token).end = end;
    (*token).size = 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsmn_parse_primitive(
    mut parser: *mut jsmn_parser,
    mut js: *const ::core::ffi::c_char,
    len: size_t,
    mut tokens: *mut jsmntok_t,
    num_tokens: size_t,
) -> ::core::ffi::c_int {
    let mut token: *mut jsmntok_t = ::core::ptr::null_mut::<jsmntok_t>();
    let mut start: ::core::ffi::c_int = 0;
    start = (*parser).pos as ::core::ffi::c_int;
    while ((*parser).pos as size_t) < len
        && *js.offset((*parser).pos as isize) as ::core::ffi::c_int != '\0' as i32
    {
        match *js.offset((*parser).pos as isize) as ::core::ffi::c_int {
            58 | 9 | 13 | 10 | 32 | 44 | 93 | 125 => {
                break;
            }
            _ => {}
        }
        if (*js.offset((*parser).pos as isize) as ::core::ffi::c_int) < 32 as ::core::ffi::c_int
            || *js.offset((*parser).pos as isize) as ::core::ffi::c_int >= 127 as ::core::ffi::c_int
        {
            (*parser).pos = start as ::core::ffi::c_uint;
            return JSMN_ERROR_INVAL as ::core::ffi::c_int;
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }
    if tokens.is_null() {
        (*parser).pos = (*parser).pos.wrapping_sub(1);
        return 0 as ::core::ffi::c_int;
    }
    token = jsmn_alloc_token(parser, tokens, num_tokens);
    if token.is_null() {
        (*parser).pos = start as ::core::ffi::c_uint;
        return JSMN_ERROR_NOMEM as ::core::ffi::c_int;
    }
    jsmn_fill_token(
        token,
        JSMN_PRIMITIVE,
        start,
        (*parser).pos as ::core::ffi::c_int,
    );
    (*parser).pos = (*parser).pos.wrapping_sub(1);
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn jsmn_parse_string(
    mut parser: *mut jsmn_parser,
    mut js: *const ::core::ffi::c_char,
    len: size_t,
    mut tokens: *mut jsmntok_t,
    num_tokens: size_t,
) -> ::core::ffi::c_int {
    let mut token: *mut jsmntok_t = ::core::ptr::null_mut::<jsmntok_t>();
    let mut start: ::core::ffi::c_int = (*parser).pos as ::core::ffi::c_int;
    (*parser).pos = (*parser).pos.wrapping_add(1);
    while ((*parser).pos as size_t) < len
        && *js.offset((*parser).pos as isize) as ::core::ffi::c_int != '\0' as i32
    {
        let mut c: ::core::ffi::c_char = *js.offset((*parser).pos as isize);
        if c as ::core::ffi::c_int == '"' as i32 {
            if tokens.is_null() {
                return 0 as ::core::ffi::c_int;
            }
            token = jsmn_alloc_token(parser, tokens, num_tokens);
            if token.is_null() {
                (*parser).pos = start as ::core::ffi::c_uint;
                return JSMN_ERROR_NOMEM as ::core::ffi::c_int;
            }
            jsmn_fill_token(
                token,
                JSMN_STRING,
                start + 1 as ::core::ffi::c_int,
                (*parser).pos as ::core::ffi::c_int,
            );
            return 0 as ::core::ffi::c_int;
        }
        if c as ::core::ffi::c_int == '\\' as i32
            && ((*parser).pos.wrapping_add(1 as ::core::ffi::c_uint) as size_t) < len
        {
            let mut i: ::core::ffi::c_int = 0;
            (*parser).pos = (*parser).pos.wrapping_add(1);
            match *js.offset((*parser).pos as isize) as ::core::ffi::c_int {
                34 | 47 | 92 | 98 | 102 | 114 | 110 | 116 => {}
                117 => {
                    (*parser).pos = (*parser).pos.wrapping_add(1);
                    i = 0 as ::core::ffi::c_int;
                    while i < 4 as ::core::ffi::c_int
                        && ((*parser).pos as size_t) < len
                        && *js.offset((*parser).pos as isize) as ::core::ffi::c_int != '\0' as i32
                    {
                        if !(*js.offset((*parser).pos as isize) as ::core::ffi::c_int
                            >= 48 as ::core::ffi::c_int
                            && *js.offset((*parser).pos as isize) as ::core::ffi::c_int
                                <= 57 as ::core::ffi::c_int
                            || *js.offset((*parser).pos as isize) as ::core::ffi::c_int
                                >= 65 as ::core::ffi::c_int
                                && *js.offset((*parser).pos as isize) as ::core::ffi::c_int
                                    <= 70 as ::core::ffi::c_int
                            || *js.offset((*parser).pos as isize) as ::core::ffi::c_int
                                >= 97 as ::core::ffi::c_int
                                && *js.offset((*parser).pos as isize) as ::core::ffi::c_int
                                    <= 102 as ::core::ffi::c_int)
                        {
                            (*parser).pos = start as ::core::ffi::c_uint;
                            return JSMN_ERROR_INVAL as ::core::ffi::c_int;
                        }
                        (*parser).pos = (*parser).pos.wrapping_add(1);
                        i += 1;
                    }
                    (*parser).pos = (*parser).pos.wrapping_sub(1);
                }
                _ => {
                    (*parser).pos = start as ::core::ffi::c_uint;
                    return JSMN_ERROR_INVAL as ::core::ffi::c_int;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }
    (*parser).pos = start as ::core::ffi::c_uint;
    return JSMN_ERROR_PART as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn jsmn_parse(
    mut parser: *mut jsmn_parser,
    mut js: *const ::core::ffi::c_char,
    len: size_t,
    mut tokens: *mut jsmntok_t,
    num_tokens: ::core::ffi::c_uint,
) -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut token: *mut jsmntok_t = ::core::ptr::null_mut::<jsmntok_t>();
    let mut count: ::core::ffi::c_int = (*parser).toknext as ::core::ffi::c_int;
    while ((*parser).pos as size_t) < len
        && *js.offset((*parser).pos as isize) as ::core::ffi::c_int != '\0' as i32
    {
        let mut c: ::core::ffi::c_char = 0;
        let mut type_0: jsmntype_t = JSMN_UNDEFINED;
        c = *js.offset((*parser).pos as isize);
        match c as ::core::ffi::c_int {
            123 | 91 => {
                count += 1;
                if !tokens.is_null() {
                    token = jsmn_alloc_token(parser, tokens, num_tokens as size_t);
                    if token.is_null() {
                        return JSMN_ERROR_NOMEM as ::core::ffi::c_int;
                    }
                    if (*parser).toksuper != -(1 as ::core::ffi::c_int) {
                        let mut t: *mut jsmntok_t =
                            tokens.offset((*parser).toksuper as isize) as *mut jsmntok_t;
                        (*t).size += 1;
                    }
                    (*token).type_0 = (if c as ::core::ffi::c_int == '{' as i32 {
                        JSMN_OBJECT as ::core::ffi::c_int
                    } else {
                        JSMN_ARRAY as ::core::ffi::c_int
                    }) as jsmntype_t;
                    (*token).start = (*parser).pos as ::core::ffi::c_int;
                    (*parser).toksuper = (*parser).toknext.wrapping_sub(1 as ::core::ffi::c_uint)
                        as ::core::ffi::c_int;
                }
            }
            125 | 93 => {
                if !tokens.is_null() {
                    type_0 = (if c as ::core::ffi::c_int == '}' as i32 {
                        JSMN_OBJECT as ::core::ffi::c_int
                    } else {
                        JSMN_ARRAY as ::core::ffi::c_int
                    }) as jsmntype_t;
                    i = (*parser).toknext.wrapping_sub(1 as ::core::ffi::c_uint)
                        as ::core::ffi::c_int;
                    while i >= 0 as ::core::ffi::c_int {
                        token = tokens.offset(i as isize) as *mut jsmntok_t;
                        if (*token).start != -(1 as ::core::ffi::c_int)
                            && (*token).end == -(1 as ::core::ffi::c_int)
                        {
                            if (*token).type_0 as ::core::ffi::c_uint
                                != type_0 as ::core::ffi::c_uint
                            {
                                return JSMN_ERROR_INVAL as ::core::ffi::c_int;
                            }
                            (*parser).toksuper = -(1 as ::core::ffi::c_int);
                            (*token).end = (*parser).pos.wrapping_add(1 as ::core::ffi::c_uint)
                                as ::core::ffi::c_int;
                            break;
                        } else {
                            i -= 1;
                        }
                    }
                    if i == -(1 as ::core::ffi::c_int) {
                        return JSMN_ERROR_INVAL as ::core::ffi::c_int;
                    }
                    while i >= 0 as ::core::ffi::c_int {
                        token = tokens.offset(i as isize) as *mut jsmntok_t;
                        if (*token).start != -(1 as ::core::ffi::c_int)
                            && (*token).end == -(1 as ::core::ffi::c_int)
                        {
                            (*parser).toksuper = i;
                            break;
                        } else {
                            i -= 1;
                        }
                    }
                }
            }
            34 => {
                r = jsmn_parse_string(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 as ::core::ffi::c_int {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -(1 as ::core::ffi::c_int) && !tokens.is_null() {
                    let ref mut fresh0 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh0 += 1;
                }
            }
            9 | 13 | 10 | 32 => {}
            58 => {
                (*parser).toksuper =
                    (*parser).toknext.wrapping_sub(1 as ::core::ffi::c_uint) as ::core::ffi::c_int;
            }
            44 => {
                if !tokens.is_null()
                    && (*parser).toksuper != -(1 as ::core::ffi::c_int)
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as ::core::ffi::c_uint
                        != JSMN_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
                    && (*tokens.offset((*parser).toksuper as isize)).type_0 as ::core::ffi::c_uint
                        != JSMN_OBJECT as ::core::ffi::c_int as ::core::ffi::c_uint
                {
                    i = (*parser).toknext.wrapping_sub(1 as ::core::ffi::c_uint)
                        as ::core::ffi::c_int;
                    while i >= 0 as ::core::ffi::c_int {
                        if (*tokens.offset(i as isize)).type_0 as ::core::ffi::c_uint
                            == JSMN_ARRAY as ::core::ffi::c_int as ::core::ffi::c_uint
                            || (*tokens.offset(i as isize)).type_0 as ::core::ffi::c_uint
                                == JSMN_OBJECT as ::core::ffi::c_int as ::core::ffi::c_uint
                        {
                            if (*tokens.offset(i as isize)).start != -(1 as ::core::ffi::c_int)
                                && (*tokens.offset(i as isize)).end == -(1 as ::core::ffi::c_int)
                            {
                                (*parser).toksuper = i;
                                break;
                            }
                        }
                        i -= 1;
                    }
                }
            }
            _ => {
                r = jsmn_parse_primitive(parser, js, len, tokens, num_tokens as size_t);
                if r < 0 as ::core::ffi::c_int {
                    return r;
                }
                count += 1;
                if (*parser).toksuper != -(1 as ::core::ffi::c_int) && !tokens.is_null() {
                    let ref mut fresh1 = (*tokens.offset((*parser).toksuper as isize)).size;
                    *fresh1 += 1;
                }
            }
        }
        (*parser).pos = (*parser).pos.wrapping_add(1);
    }
    if !tokens.is_null() {
        i = (*parser).toknext.wrapping_sub(1 as ::core::ffi::c_uint) as ::core::ffi::c_int;
        while i >= 0 as ::core::ffi::c_int {
            if (*tokens.offset(i as isize)).start != -(1 as ::core::ffi::c_int)
                && (*tokens.offset(i as isize)).end == -(1 as ::core::ffi::c_int)
            {
                return JSMN_ERROR_PART as ::core::ffi::c_int;
            }
            i -= 1;
        }
    }
    return count;
}
#[no_mangle]
pub unsafe extern "C" fn jsmn_init(mut parser: *mut jsmn_parser) {
    (*parser).pos = 0 as ::core::ffi::c_uint;
    (*parser).toknext = 0 as ::core::ffi::c_uint;
    (*parser).toksuper = -(1 as ::core::ffi::c_int);
}
