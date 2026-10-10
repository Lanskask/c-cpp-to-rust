#![allow(
    dead_code,
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    unused_assignments,
    unused_mut
)]
#![feature(c_variadic, raw_ref_op)]
#[allow(unused_imports)]
use ::tests_c2rust;
extern "C" {
    fn printf(_: *const ::core::ffi::c_char, ...) -> ::core::ffi::c_int;
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn free(_: *mut ::core::ffi::c_void);
    fn memcpy(
        __dst: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __b: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __len: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    fn strncmp(
        __s1: *const ::core::ffi::c_char,
        __s2: *const ::core::ffi::c_char,
        __n: size_t,
    ) -> ::core::ffi::c_int;
}
pub type __builtin_va_list = *mut ::core::ffi::c_char;
pub type va_list = __builtin_va_list;
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
static mut test_passed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
static mut test_failed: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
unsafe extern "C" fn test(
    mut func: Option<unsafe extern "C" fn() -> ::core::ffi::c_int>,
    mut name: *const ::core::ffi::c_char,
) {
    let mut r: ::core::ffi::c_int = func.expect("non-null function pointer")();
    if r == 0 as ::core::ffi::c_int {
        test_passed += 1;
    } else {
        test_failed += 1;
        printf(
            b"FAILED: %s (at line %d)\n\0" as *const u8 as *const ::core::ffi::c_char,
            name,
            r,
        );
    };
}
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
unsafe extern "C" fn vtokeq(
    mut s: *const ::core::ffi::c_char,
    mut t: *mut jsmntok_t,
    mut numtok: ::core::ffi::c_ulong,
    mut ap: ::core::ffi::VaList,
) -> ::core::ffi::c_int {
    if numtok > 0 as ::core::ffi::c_ulong {
        let mut i: ::core::ffi::c_ulong = 0;
        let mut start: ::core::ffi::c_int = 0;
        let mut end: ::core::ffi::c_int = 0;
        let mut size: ::core::ffi::c_int = 0;
        let mut type_0: jsmntype_t = JSMN_UNDEFINED;
        let mut value: *mut ::core::ffi::c_char = ::core::ptr::null_mut::<::core::ffi::c_char>();
        size = -(1 as ::core::ffi::c_int);
        value = ::core::ptr::null_mut::<::core::ffi::c_char>();
        i = 0 as ::core::ffi::c_ulong;
        while i < numtok {
            type_0 = ap.arg::<jsmntype_t>();
            if type_0 as ::core::ffi::c_uint
                == JSMN_STRING as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                value = ap.arg::<*mut ::core::ffi::c_char>();
                size = ap.arg::<::core::ffi::c_int>();
                end = -(1 as ::core::ffi::c_int);
                start = end;
            } else if type_0 as ::core::ffi::c_uint
                == JSMN_PRIMITIVE as ::core::ffi::c_int as ::core::ffi::c_uint
            {
                value = ap.arg::<*mut ::core::ffi::c_char>();
                size = -(1 as ::core::ffi::c_int);
                end = size;
                start = end;
            } else {
                start = ap.arg::<::core::ffi::c_int>();
                end = ap.arg::<::core::ffi::c_int>();
                size = ap.arg::<::core::ffi::c_int>();
                value = ::core::ptr::null_mut::<::core::ffi::c_char>();
            }
            if (*t.offset(i as isize)).type_0 as ::core::ffi::c_uint
                != type_0 as ::core::ffi::c_uint
            {
                printf(
                    b"token %lu type is %d, not %d\n\0" as *const u8 as *const ::core::ffi::c_char,
                    i,
                    (*t.offset(i as isize)).type_0 as ::core::ffi::c_uint,
                    type_0 as ::core::ffi::c_uint,
                );
                return 0 as ::core::ffi::c_int;
            }
            if start != -(1 as ::core::ffi::c_int) && end != -(1 as ::core::ffi::c_int) {
                if (*t.offset(i as isize)).start != start {
                    printf(
                        b"token %lu start is %d, not %d\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                        i,
                        (*t.offset(i as isize)).start,
                        start,
                    );
                    return 0 as ::core::ffi::c_int;
                }
                if (*t.offset(i as isize)).end != end {
                    printf(
                        b"token %lu end is %d, not %d\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                        i,
                        (*t.offset(i as isize)).end,
                        end,
                    );
                    return 0 as ::core::ffi::c_int;
                }
            }
            if size != -(1 as ::core::ffi::c_int) && (*t.offset(i as isize)).size != size {
                printf(
                    b"token %lu size is %d, not %d\n\0" as *const u8 as *const ::core::ffi::c_char,
                    i,
                    (*t.offset(i as isize)).size,
                    size,
                );
                return 0 as ::core::ffi::c_int;
            }
            if !s.is_null() && !value.is_null() {
                let mut p: *const ::core::ffi::c_char =
                    s.offset((*t.offset(i as isize)).start as isize);
                if strlen(value)
                    != ((*t.offset(i as isize)).end - (*t.offset(i as isize)).start) as size_t
                    || strncmp(
                        p,
                        value,
                        ((*t.offset(i as isize)).end - (*t.offset(i as isize)).start) as size_t,
                    ) != 0 as ::core::ffi::c_int
                {
                    printf(
                        b"token %lu value is %.*s, not %s\n\0" as *const u8
                            as *const ::core::ffi::c_char,
                        i,
                        (*t.offset(i as isize)).end - (*t.offset(i as isize)).start,
                        s.offset((*t.offset(i as isize)).start as isize),
                        value,
                    );
                    return 0 as ::core::ffi::c_int;
                }
            }
            i = i.wrapping_add(1);
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn tokeq(
    mut s: *const ::core::ffi::c_char,
    mut tokens: *mut jsmntok_t,
    mut numtok: ::core::ffi::c_ulong,
    mut args: ...
) -> ::core::ffi::c_int {
    let mut ok: ::core::ffi::c_int = 0;
    let mut args_0: ::core::ffi::VaListImpl;
    args_0 = args.clone();
    ok = vtokeq(s, tokens, numtok, args_0.as_va_list());
    return ok;
}
unsafe extern "C" fn parse(
    mut s: *const ::core::ffi::c_char,
    mut status: ::core::ffi::c_int,
    mut numtok: ::core::ffi::c_ulong,
    mut args: ...
) -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = 0;
    let mut ok: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut args_0: ::core::ffi::VaListImpl;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut t: *mut jsmntok_t =
        malloc((numtok as size_t).wrapping_mul(::core::mem::size_of::<jsmntok_t>() as size_t))
            as *mut jsmntok_t;
    jsmn_init(&raw mut p);
    r = jsmn_parse(&raw mut p, s, strlen(s), t, numtok as ::core::ffi::c_uint);
    if r != status {
        printf(
            b"status is %d, not %d\n\0" as *const u8 as *const ::core::ffi::c_char,
            r,
            status,
        );
        return 0 as ::core::ffi::c_int;
    }
    if status >= 0 as ::core::ffi::c_int {
        args_0 = args.clone();
        ok = vtokeq(s, t, numtok, args_0.as_va_list());
    }
    free(t as *mut ::core::ffi::c_void);
    return ok;
}
#[no_mangle]
pub unsafe extern "C" fn test_empty() -> ::core::ffi::c_int {
    if parse(
        b"{}\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        1 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 10 as ::core::ffi::c_int;
    }
    if parse(
        b"[]\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        1 as ::core::ffi::c_ulong,
        JSMN_ARRAY as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 11 as ::core::ffi::c_int;
    }
    if parse(
        b"[{},{}]\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_ARRAY as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        7 as ::core::ffi::c_int,
        2 as ::core::ffi::c_int,
        JSMN_OBJECT as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        3 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        JSMN_OBJECT as ::core::ffi::c_int,
        4 as ::core::ffi::c_int,
        6 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 13 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_object() -> ::core::ffi::c_int {
    if parse(
        b"{\"a\":0}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        7 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"0\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 19 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":[]}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        8 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_ARRAY as ::core::ffi::c_int,
        5 as ::core::ffi::c_int,
        7 as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 21 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":{},\"b\":{}}\0" as *const u8 as *const ::core::ffi::c_char,
        5 as ::core::ffi::c_int,
        5 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        2 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        0 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"b\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 24 as ::core::ffi::c_int;
    }
    if parse(
        b"{\n \"Day\": 26,\n \"Month\": 9,\n \"Year\": 12\n }\0" as *const u8
            as *const ::core::ffi::c_char,
        7 as ::core::ffi::c_int,
        7 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        3 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"Day\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"26\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_STRING as ::core::ffi::c_int,
        b"Month\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"9\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_STRING as ::core::ffi::c_int,
        b"Year\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"12\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 28 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\": 0, \"b\": \"c\"}\0" as *const u8 as *const ::core::ffi::c_char,
        5 as ::core::ffi::c_int,
        5 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        2 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"0\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_STRING as ::core::ffi::c_int,
        b"b\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"c\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 31 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_array() -> ::core::ffi::c_int {
    if parse(
        b"[10]\0" as *const u8 as *const ::core::ffi::c_char,
        2 as ::core::ffi::c_int,
        2 as ::core::ffi::c_ulong,
        JSMN_ARRAY as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"10\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 54 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\": 1]\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 55 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_primitive() -> ::core::ffi::c_int {
    if parse(
        b"{\"boolVar\" : true }\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"boolVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"true\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 63 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"boolVar\" : false }\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"boolVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"false\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 65 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"nullVar\" : null }\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"nullVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"null\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 67 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"intVar\" : 12}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"intVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"12\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 69 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"floatVar\" : 12.345}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"floatVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"12.345\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 71 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_string() -> ::core::ffi::c_int {
    if parse(
        b"{\"strVar\" : \"hello world\"}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"strVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"hello world\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 77 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"strVar\" : \"escapes: \\/\\r\\n\\t\\b\\f\\\"\\\\\"}\0" as *const u8
            as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"strVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"escapes: \\/\\r\\n\\t\\b\\f\\\"\\\\\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 80 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"strVar\": \"\"}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"strVar\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 82 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":\"\\uAbcD\"}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"\\uAbcD\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 84 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":\"str\\u0000\"}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"str\\u0000\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 86 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":\"\\uFFFFstr\"}\0" as *const u8 as *const ::core::ffi::c_char,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"\\uFFFFstr\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 88 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":[\"\\u0280\"]}\0" as *const u8 as *const ::core::ffi::c_char,
        4 as ::core::ffi::c_int,
        4 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_ARRAY as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"\\u0280\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
    ) == 0
    {
        return 91 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":\"str\\uFFGFstr\"}\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 93 as ::core::ffi::c_int;
    }
    if parse(
        b"{\"a\":\"str\\u@FfF\"}\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 94 as ::core::ffi::c_int;
    }
    if parse(
        b"{{\"a\":[\"\\u028\"]}\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        4 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 95 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_partial_string() -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_ulong = 0;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut tok: [jsmntok_t; 5] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 5];
    let mut js: *const ::core::ffi::c_char =
        b"{\"x\": \"va\\\\ue\", \"y\": \"value y\"}\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    i = 1 as ::core::ffi::c_ulong;
    while i as size_t <= strlen(js) {
        r = jsmn_parse(
            &raw mut p,
            js,
            i as size_t,
            &raw mut tok as *mut jsmntok_t,
            (::core::mem::size_of::<[jsmntok_t; 5]>() as usize)
                .wrapping_div(::core::mem::size_of::<jsmntok_t>() as usize)
                as ::core::ffi::c_uint,
        );
        if i as size_t == strlen(js) {
            if !(r == 5 as ::core::ffi::c_int) {
                return 110 as ::core::ffi::c_int;
            }
            if tokeq(
                js,
                &raw mut tok as *mut jsmntok_t,
                5 as ::core::ffi::c_ulong,
                JSMN_OBJECT as ::core::ffi::c_int,
                -(1 as ::core::ffi::c_int),
                -(1 as ::core::ffi::c_int),
                2 as ::core::ffi::c_int,
                JSMN_STRING as ::core::ffi::c_int,
                b"x\0" as *const u8 as *const ::core::ffi::c_char,
                1 as ::core::ffi::c_int,
                JSMN_STRING as ::core::ffi::c_int,
                b"va\\\\ue\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
                JSMN_STRING as ::core::ffi::c_int,
                b"y\0" as *const u8 as *const ::core::ffi::c_char,
                1 as ::core::ffi::c_int,
                JSMN_STRING as ::core::ffi::c_int,
                b"value y\0" as *const u8 as *const ::core::ffi::c_char,
                0 as ::core::ffi::c_int,
            ) == 0
            {
                return 113 as ::core::ffi::c_int;
            }
        } else if !(r == JSMN_ERROR_PART as ::core::ffi::c_int) {
            return 115 as ::core::ffi::c_int;
        }
        i = i.wrapping_add(1);
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_partial_array() -> ::core::ffi::c_int {
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_array_nomem() -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    let mut r: ::core::ffi::c_int = 0;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut toksmall: [jsmntok_t; 10] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 10];
    let mut toklarge: [jsmntok_t; 10] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 10];
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"  [ 1, true, [123, \"hello\"]]\0" as *const u8 as *const ::core::ffi::c_char;
    i = 0 as ::core::ffi::c_int;
    while i < 6 as ::core::ffi::c_int {
        jsmn_init(&raw mut p);
        memset(
            &raw mut toksmall as *mut jsmntok_t as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<[jsmntok_t; 10]>() as size_t,
        );
        memset(
            &raw mut toklarge as *mut jsmntok_t as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<[jsmntok_t; 10]>() as size_t,
        );
        r = jsmn_parse(
            &raw mut p,
            js,
            strlen(js),
            &raw mut toksmall as *mut jsmntok_t,
            i as ::core::ffi::c_uint,
        );
        if !(r == JSMN_ERROR_NOMEM as ::core::ffi::c_int) {
            return 159 as ::core::ffi::c_int;
        }
        memcpy(
            &raw mut toklarge as *mut jsmntok_t as *mut ::core::ffi::c_void,
            &raw mut toksmall as *mut jsmntok_t as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[jsmntok_t; 10]>() as size_t,
        );
        r = jsmn_parse(
            &raw mut p,
            js,
            strlen(js),
            &raw mut toklarge as *mut jsmntok_t,
            10 as ::core::ffi::c_uint,
        );
        if !(r >= 0 as ::core::ffi::c_int) {
            return 164 as ::core::ffi::c_int;
        }
        if tokeq(
            js,
            &raw mut toklarge as *mut jsmntok_t,
            4 as ::core::ffi::c_ulong,
            JSMN_ARRAY as ::core::ffi::c_int,
            -(1 as ::core::ffi::c_int),
            -(1 as ::core::ffi::c_int),
            3 as ::core::ffi::c_int,
            JSMN_PRIMITIVE as ::core::ffi::c_int,
            b"1\0" as *const u8 as *const ::core::ffi::c_char,
            JSMN_PRIMITIVE as ::core::ffi::c_int,
            b"true\0" as *const u8 as *const ::core::ffi::c_char,
            JSMN_ARRAY as ::core::ffi::c_int,
            -(1 as ::core::ffi::c_int),
            -(1 as ::core::ffi::c_int),
            2 as ::core::ffi::c_int,
            JSMN_PRIMITIVE as ::core::ffi::c_int,
            b"123\0" as *const u8 as *const ::core::ffi::c_char,
            JSMN_STRING as ::core::ffi::c_int,
            b"hello\0" as *const u8 as *const ::core::ffi::c_char,
            0 as ::core::ffi::c_int,
        ) == 0
        {
            return 167 as ::core::ffi::c_int;
        }
        i += 1;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_unquoted_keys() -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = 0;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut tok: [jsmntok_t; 10] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 10];
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    jsmn_init(&raw mut p);
    js = b"key1: \"value\"\nkey2 : 123\0" as *const u8 as *const ::core::ffi::c_char;
    r = jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        &raw mut tok as *mut jsmntok_t,
        10 as ::core::ffi::c_uint,
    );
    if !(r >= 0 as ::core::ffi::c_int) {
        return 183 as ::core::ffi::c_int;
    }
    if tokeq(
        js,
        &raw mut tok as *mut jsmntok_t,
        4 as ::core::ffi::c_ulong,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"key1\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_STRING as ::core::ffi::c_int,
        b"value\0" as *const u8 as *const ::core::ffi::c_char,
        0 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"key2\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"123\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 185 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_issue_22() -> ::core::ffi::c_int {
    let mut r: ::core::ffi::c_int = 0;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut tokens: [jsmntok_t; 128] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 128];
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"{ \"height\":10, \"layers\":[ { \"data\":[6,6], \"height\":10, \"name\":\"Calque de Tile 1\", \"opacity\":1, \"type\":\"tilelayer\", \"visible\":true, \"width\":10, \"x\":0, \"y\":0 }], \"orientation\":\"orthogonal\", \"properties\": { }, \"tileheight\":32, \"tilesets\":[ { \"firstgid\":1, \"image\":\"..\\/images\\/tiles.png\", \"imageheight\":64, \"imagewidth\":160, \"margin\":0, \"name\":\"Tiles\", \"properties\":{}, \"spacing\":0, \"tileheight\":32, \"tilewidth\":32 }], \"tilewidth\":32, \"version\":1, \"width\":10 }\0"
        as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    r = jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        &raw mut tokens as *mut jsmntok_t,
        128 as ::core::ffi::c_uint,
    );
    if !(r >= 0 as ::core::ffi::c_int) {
        return 209 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_issue_27() -> ::core::ffi::c_int {
    let mut js: *const ::core::ffi::c_char =
        b"{ \"name\" : \"Jack\", \"age\" : 27 } { \"name\" : \"Anna\", \0" as *const u8
            as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_PART as ::core::ffi::c_int,
        8 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 216 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_input_length() -> ::core::ffi::c_int {
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    let mut r: ::core::ffi::c_int = 0;
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut tokens: [jsmntok_t; 10] = [jsmntok {
        type_0: JSMN_UNDEFINED,
        start: 0,
        end: 0,
        size: 0,
    }; 10];
    js = b"{\"a\": 0}garbage\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    r = jsmn_parse(
        &raw mut p,
        js,
        8 as size_t,
        &raw mut tokens as *mut jsmntok_t,
        10 as ::core::ffi::c_uint,
    );
    if !(r == 3 as ::core::ffi::c_int) {
        return 230 as ::core::ffi::c_int;
    }
    if tokeq(
        js,
        &raw mut tokens as *mut jsmntok_t,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        -(1 as ::core::ffi::c_int),
        -(1 as ::core::ffi::c_int),
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"0\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 232 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_count() -> ::core::ffi::c_int {
    let mut p: jsmn_parser = jsmn_parser {
        pos: 0,
        toknext: 0,
        toksuper: 0,
    };
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"{}\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 1 as ::core::ffi::c_int)
    {
        return 242 as ::core::ffi::c_int;
    }
    js = b"[]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 1 as ::core::ffi::c_int)
    {
        return 246 as ::core::ffi::c_int;
    }
    js = b"[[]]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 2 as ::core::ffi::c_int)
    {
        return 250 as ::core::ffi::c_int;
    }
    js = b"[[], []]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 3 as ::core::ffi::c_int)
    {
        return 254 as ::core::ffi::c_int;
    }
    js = b"[[], []]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 3 as ::core::ffi::c_int)
    {
        return 258 as ::core::ffi::c_int;
    }
    js = b"[[], [[]], [[], []]]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 7 as ::core::ffi::c_int)
    {
        return 262 as ::core::ffi::c_int;
    }
    js = b"[\"a\", [[], []]]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 5 as ::core::ffi::c_int)
    {
        return 266 as ::core::ffi::c_int;
    }
    js = b"[[], \"[], [[]]\", [[]]]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 5 as ::core::ffi::c_int)
    {
        return 270 as ::core::ffi::c_int;
    }
    js = b"[1, 2, 3]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 4 as ::core::ffi::c_int)
    {
        return 274 as ::core::ffi::c_int;
    }
    js = b"[1, 2, [3, \"a\"], null]\0" as *const u8 as *const ::core::ffi::c_char;
    jsmn_init(&raw mut p);
    if !(jsmn_parse(
        &raw mut p,
        js,
        strlen(js),
        ::core::ptr::null_mut::<jsmntok_t>(),
        0 as ::core::ffi::c_uint,
    ) == 7 as ::core::ffi::c_int)
    {
        return 278 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_nonstrict() -> ::core::ffi::c_int {
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"a: 0garbage\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        2 as ::core::ffi::c_int,
        2 as ::core::ffi::c_ulong,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"a\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"0garbage\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 287 as ::core::ffi::c_int;
    }
    js = b"Day : 26\nMonth : Sep\n\nYear: 12\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        6 as ::core::ffi::c_int,
        6 as ::core::ffi::c_ulong,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"Day\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"26\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"Month\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"Sep\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"Year\0" as *const u8 as *const ::core::ffi::c_char,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"12\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 292 as ::core::ffi::c_int;
    }
    js = b"\"key {1\": 1234\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        2 as ::core::ffi::c_int,
        2 as ::core::ffi::c_ulong,
        JSMN_STRING as ::core::ffi::c_int,
        b"key {1\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"1234\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 296 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_unmatched_brackets() -> ::core::ffi::c_int {
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"\"key 1\": 1234}\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        2 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 305 as ::core::ffi::c_int;
    }
    js = b"{\"key 1\": 1234\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_PART as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 307 as ::core::ffi::c_int;
    }
    js = b"{\"key 1\": 1234}}\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 309 as ::core::ffi::c_int;
    }
    js = b"\"key 1\"}: 1234\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_INVAL as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 311 as ::core::ffi::c_int;
    }
    js = b"{\"key {1\": 1234}\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        16 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"key {1\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"1234\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 314 as ::core::ffi::c_int;
    }
    js = b"{\"key 1\":{\"key 2\": 1234}\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        JSMN_ERROR_PART as ::core::ffi::c_int,
        5 as ::core::ffi::c_ulong,
    ) == 0
    {
        return 316 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[no_mangle]
pub unsafe extern "C" fn test_object_key() -> ::core::ffi::c_int {
    let mut js: *const ::core::ffi::c_char = ::core::ptr::null::<::core::ffi::c_char>();
    js = b"{\"key\": 1}\0" as *const u8 as *const ::core::ffi::c_char;
    if parse(
        js,
        3 as ::core::ffi::c_int,
        3 as ::core::ffi::c_ulong,
        JSMN_OBJECT as ::core::ffi::c_int,
        0 as ::core::ffi::c_int,
        10 as ::core::ffi::c_int,
        1 as ::core::ffi::c_int,
        JSMN_STRING as ::core::ffi::c_int,
        b"key\0" as *const u8 as *const ::core::ffi::c_char,
        1 as ::core::ffi::c_int,
        JSMN_PRIMITIVE as ::core::ffi::c_int,
        b"1\0" as *const u8 as *const ::core::ffi::c_char,
    ) == 0
    {
        return 325 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe fn main_0() -> ::core::ffi::c_int {
    test(
        Some(test_empty as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for a empty JSON objects/arrays\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_object as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for a JSON objects\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_array as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for a JSON arrays\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_primitive as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test primitive JSON data types\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_string as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test string JSON data types\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_partial_string as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test partial JSON string parsing\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_partial_array as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test partial array reading\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_array_nomem as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test array reading with a smaller number of tokens\0" as *const u8
            as *const ::core::ffi::c_char,
    );
    test(
        Some(test_unquoted_keys as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test unquoted keys (like in JavaScript)\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_input_length as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test strings that are not null-terminated\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_issue_22 as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test issue #22\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_issue_27 as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test issue #27\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_count as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test tokens count estimation\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_nonstrict as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for non-strict mode\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_unmatched_brackets as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for unmatched brackets\0" as *const u8 as *const ::core::ffi::c_char,
    );
    test(
        Some(test_object_key as unsafe extern "C" fn() -> ::core::ffi::c_int),
        b"test for key type\0" as *const u8 as *const ::core::ffi::c_char,
    );
    printf(
        b"\nPASSED: %d\nFAILED: %d\n\0" as *const u8 as *const ::core::ffi::c_char,
        test_passed,
        test_failed,
    );
    return (test_failed > 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
}
pub fn main() {
    unsafe { ::std::process::exit(main_0() as i32) }
}
