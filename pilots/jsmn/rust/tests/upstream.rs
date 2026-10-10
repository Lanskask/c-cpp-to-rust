//! Translation of upstream `test/tests.c` (+ `test.h`, `testutil.h`) through the Judge adapter.
//! Same inputs, same expected values; `#ifdef JSMN_STRICT` becomes `#[cfg(feature = "strict")]`. All 16
//! upstream tests are always present (as in C, where a test whose body is `#ifdef`'d away passes empty).
//! Variadic expectations become the `E` slice (idiom V2). Changing this file is a Judge change (own commit).
mod adapter;

use adapter::{ARRAY, ERROR_INVAL, ERROR_NOMEM, ERROR_PART, OBJECT, P, PRIMITIVE, STRING};

/// One expected token, as the C variadic list in `vtokeq` reads it. `-1` means "do not check".
#[derive(Clone, Copy)]
enum E {
    Obj(i64, i64, i64),
    Arr(i64, i64, i64),
    Str(&'static str, i64),
    Prim(&'static str),
}

use E::{Arr, Obj, Prim, Str};

/// `vtokeq`: compare the first `exp.len()` tokens of `p` with `exp`. `Err` carries the C diagnostic.
fn tokeq(js: &[u8], p: &P, exp: &[E]) -> Result<(), String> {
    let toks = p.tokens(exp.len());
    for (i, (t, e)) in toks.iter().zip(exp).enumerate() {
        let (kind, start, end, size, value) = match *e {
            Obj(s, en, sz) => (OBJECT, s, en, sz, None),
            Arr(s, en, sz) => (ARRAY, s, en, sz, None),
            Str(v, sz) => (STRING, -1, -1, sz, Some(v)),
            Prim(v) => (PRIMITIVE, -1, -1, -1, Some(v)),
        };
        if t.kind != kind {
            return Err(format!("token {i} type is {}, not {kind}", t.kind));
        }
        if start != -1 && end != -1 {
            if t.start != start {
                return Err(format!("token {i} start is {}, not {start}", t.start));
            }
            if t.end != end {
                return Err(format!("token {i} end is {}, not {end}", t.end));
            }
        }
        if size != -1 && t.size != size {
            return Err(format!("token {i} size is {}, not {size}", t.size));
        }
        if let Some(v) = value {
            let got = &js[t.start as usize..t.end as usize];
            if got != v.as_bytes() {
                return Err(format!(
                    "token {i} value is {}, not {v}",
                    String::from_utf8_lossy(got)
                ));
            }
        }
    }
    Ok(())
}

/// `parse` from testutil.h: fresh parser, `numtok` tokens, expect `status`, then (status >= 0) the tokens.
fn parse(js: &[u8], status: i32, numtok: usize, exp: &[E]) -> bool {
    let mut p = P::new();
    let r = p.parse(js, Some(numtok));
    if r != status {
        eprintln!("status is {r}, not {status}");
        return false;
    }
    if status >= 0 {
        if let Err(msg) = tokeq(js, &p, exp) {
            eprintln!("{msg}");
            return false;
        }
    }
    true
}

macro_rules! check {
    ($cond:expr) => {
        assert!($cond, "check failed: {}", stringify!($cond));
    };
}

#[test]
fn test_empty() {
    check!(parse(b"{}", 1, 1, &[Obj(0, 2, 0)]));
    check!(parse(b"[]", 1, 1, &[Arr(0, 2, 0)]));
    check!(parse(
        b"[{},{}]",
        3,
        3,
        &[Arr(0, 7, 2), Obj(1, 3, 0), Obj(4, 6, 0)]
    ));
}

#[test]
fn test_object() {
    check!(parse(
        b"{\"a\":0}",
        3,
        3,
        &[Obj(0, 7, 1), Str("a", 1), Prim("0")]
    ));
    check!(parse(
        b"{\"a\":[]}",
        3,
        3,
        &[Obj(0, 8, 1), Str("a", 1), Arr(5, 7, 0)]
    ));
    check!(parse(
        b"{\"a\":{},\"b\":{}}",
        5,
        5,
        &[
            Obj(-1, -1, 2),
            Str("a", 1),
            Obj(-1, -1, 0),
            Str("b", 1),
            Obj(-1, -1, 0)
        ]
    ));
    check!(parse(
        b"{\n \"Day\": 26,\n \"Month\": 9,\n \"Year\": 12\n }",
        7,
        7,
        &[
            Obj(-1, -1, 3),
            Str("Day", 1),
            Prim("26"),
            Str("Month", 1),
            Prim("9"),
            Str("Year", 1),
            Prim("12")
        ]
    ));
    check!(parse(
        b"{\"a\": 0, \"b\": \"c\"}",
        5,
        5,
        &[
            Obj(-1, -1, 2),
            Str("a", 1),
            Prim("0"),
            Str("b", 1),
            Str("c", 0)
        ]
    ));

    #[cfg(feature = "strict")]
    {
        check!(parse(b"{\"a\"\n0}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{\"a\", 0}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{\"a\": {2}}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{\"a\": {2: 3}}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{\"a\": {\"a\": 2 3}}", ERROR_INVAL, 5, &[]));
    }
}

#[test]
fn test_array() {
    check!(parse(b"[10]", 2, 2, &[Arr(-1, -1, 1), Prim("10")]));
    check!(parse(b"{\"a\": 1]", ERROR_INVAL, 3, &[]));
}

#[test]
fn test_primitive() {
    check!(parse(
        b"{\"boolVar\" : true }",
        3,
        3,
        &[Obj(-1, -1, 1), Str("boolVar", 1), Prim("true")]
    ));
    check!(parse(
        b"{\"boolVar\" : false }",
        3,
        3,
        &[Obj(-1, -1, 1), Str("boolVar", 1), Prim("false")]
    ));
    check!(parse(
        b"{\"nullVar\" : null }",
        3,
        3,
        &[Obj(-1, -1, 1), Str("nullVar", 1), Prim("null")]
    ));
    check!(parse(
        b"{\"intVar\" : 12}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("intVar", 1), Prim("12")]
    ));
    check!(parse(
        b"{\"floatVar\" : 12.345}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("floatVar", 1), Prim("12.345")]
    ));
}

#[test]
fn test_string() {
    check!(parse(
        b"{\"strVar\" : \"hello world\"}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("strVar", 1), Str("hello world", 0)]
    ));
    check!(parse(
        b"{\"strVar\" : \"escapes: \\/\\r\\n\\t\\b\\f\\\"\\\\\"}",
        3,
        3,
        &[
            Obj(-1, -1, 1),
            Str("strVar", 1),
            Str("escapes: \\/\\r\\n\\t\\b\\f\\\"\\\\", 0)
        ]
    ));
    check!(parse(
        b"{\"strVar\": \"\"}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("strVar", 1), Str("", 0)]
    ));
    check!(parse(
        b"{\"a\":\"\\uAbcD\"}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("a", 1), Str("\\uAbcD", 0)]
    ));
    check!(parse(
        b"{\"a\":\"str\\u0000\"}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("a", 1), Str("str\\u0000", 0)]
    ));
    check!(parse(
        b"{\"a\":\"\\uFFFFstr\"}",
        3,
        3,
        &[Obj(-1, -1, 1), Str("a", 1), Str("\\uFFFFstr", 0)]
    ));
    check!(parse(
        b"{\"a\":[\"\\u0280\"]}",
        4,
        4,
        &[
            Obj(-1, -1, 1),
            Str("a", 1),
            Arr(-1, -1, 1),
            Str("\\u0280", 0)
        ]
    ));

    check!(parse(b"{\"a\":\"str\\uFFGFstr\"}", ERROR_INVAL, 3, &[]));
    check!(parse(b"{\"a\":\"str\\u@FfF\"}", ERROR_INVAL, 3, &[]));
    check!(parse(b"{{\"a\":[\"\\u028\"]}", ERROR_INVAL, 4, &[]));
}

#[test]
fn test_partial_string() {
    let js: &[u8] = b"{\"x\": \"va\\\\ue\", \"y\": \"value y\"}";
    let mut p = P::new();
    for i in 1..=js.len() {
        let r = p.parse(&js[..i], Some(5));
        if i == js.len() {
            check!(r == 5);
            check!(tokeq(
                js,
                &p,
                &[
                    Obj(-1, -1, 2),
                    Str("x", 1),
                    Str("va\\\\ue", 0),
                    Str("y", 1),
                    Str("value y", 0)
                ]
            )
            .is_ok());
        } else {
            check!(r == ERROR_PART);
        }
    }
}

#[test]
fn test_partial_array() {
    #[cfg(feature = "strict")]
    {
        let js: &[u8] = b"[ 1, true, [123, \"hello\"]]";
        let mut p = P::new();
        for i in 1..=js.len() {
            let r = p.parse(&js[..i], Some(10));
            if i == js.len() {
                check!(r == 6);
                check!(tokeq(
                    js,
                    &p,
                    &[
                        Arr(-1, -1, 3),
                        Prim("1"),
                        Prim("true"),
                        Arr(-1, -1, 2),
                        Prim("123"),
                        Str("hello", 0)
                    ]
                )
                .is_ok());
            } else {
                check!(r == ERROR_PART);
            }
        }
    }
}

#[test]
fn test_array_nomem() {
    let js: &[u8] = b"  [ 1, true, [123, \"hello\"]]";
    for i in 0..6 {
        // C: toksmall/toklarge zeroed, parse into toksmall[..i], copy it to toklarge, resume with 10 slots.
        // One persistent zeroed buffer behaves the same: slots past `i` are untouched by the failed parse.
        let mut p = P::new();
        p.clear_tokens();
        let r = p.parse(js, Some(i));
        check!(r == ERROR_NOMEM);

        let r = p.parse(js, Some(10));
        check!(r >= 0);
        check!(tokeq(
            js,
            &p,
            &[
                Arr(-1, -1, 3),
                Prim("1"),
                Prim("true"),
                Arr(-1, -1, 2),
                Prim("123"),
                Str("hello", 0)
            ][..4]
        )
        .is_ok());
    }
}

#[test]
fn test_unquoted_keys() {
    #[cfg(not(feature = "strict"))]
    {
        let js: &[u8] = b"key1: \"value\"\nkey2 : 123";
        let mut p = P::new();
        let r = p.parse(js, Some(10));
        check!(r >= 0);
        check!(tokeq(
            js,
            &p,
            &[Prim("key1"), Str("value", 0), Prim("key2"), Prim("123")]
        )
        .is_ok());
    }
}

#[test]
fn test_issue_22() {
    let js: &[u8] = b"{ \"height\":10, \"layers\":[ { \"data\":[6,6], \"height\":10, \
\"name\":\"Calque de Tile 1\", \"opacity\":1, \"type\":\"tilelayer\", \
\"visible\":true, \"width\":10, \"x\":0, \"y\":0 }], \
\"orientation\":\"orthogonal\", \"properties\": { }, \"tileheight\":32, \
\"tilesets\":[ { \"firstgid\":1, \"image\":\"..\\/images\\/tiles.png\", \
\"imageheight\":64, \"imagewidth\":160, \"margin\":0, \
\"name\":\"Tiles\", \
\"properties\":{}, \"spacing\":0, \"tileheight\":32, \"tilewidth\":32 \
}], \
\"tilewidth\":32, \"version\":1, \"width\":10 }";
    let mut p = P::new();
    check!(p.parse(js, Some(128)) >= 0);
}

#[test]
fn test_issue_27() {
    let js: &[u8] = b"{ \"name\" : \"Jack\", \"age\" : 27 } { \"name\" : \"Anna\", ";
    check!(parse(js, ERROR_PART, 8, &[]));
}

#[test]
fn test_input_length() {
    let js: &[u8] = b"{\"a\": 0}garbage";
    let mut p = P::new();
    let r = p.parse(&js[..8], Some(10));
    check!(r == 3);
    check!(tokeq(js, &p, &[Obj(-1, -1, 1), Str("a", 1), Prim("0")]).is_ok());
}

#[test]
fn test_count() {
    let cases: [(&[u8], i32); 10] = [
        (b"{}", 1),
        (b"[]", 1),
        (b"[[]]", 2),
        (b"[[], []]", 3),
        (b"[[], []]", 3),
        (b"[[], [[]], [[], []]]", 7),
        (b"[\"a\", [[], []]]", 5),
        (b"[[], \"[], [[]]\", [[]]]", 5),
        (b"[1, 2, 3]", 4),
        (b"[1, 2, [3, \"a\"], null]", 7),
    ];
    for (js, want) in cases {
        let mut p = P::new();
        check!(p.parse(js, None) == want);
    }
}

#[test]
fn test_nonstrict() {
    #[cfg(not(feature = "strict"))]
    {
        check!(parse(b"a: 0garbage", 2, 2, &[Prim("a"), Prim("0garbage")]));
        check!(parse(
            b"Day : 26\nMonth : Sep\n\nYear: 12",
            6,
            6,
            &[
                Prim("Day"),
                Prim("26"),
                Prim("Month"),
                Prim("Sep"),
                Prim("Year"),
                Prim("12")
            ]
        ));
        // nested {s don't cause a parse error.
        check!(parse(
            b"\"key {1\": 1234",
            2,
            2,
            &[Str("key {1", 1), Prim("1234")]
        ));
    }
}

#[test]
fn test_unmatched_brackets() {
    check!(parse(b"\"key 1\": 1234}", ERROR_INVAL, 2, &[]));
    check!(parse(b"{\"key 1\": 1234", ERROR_PART, 3, &[]));
    check!(parse(b"{\"key 1\": 1234}}", ERROR_INVAL, 3, &[]));
    check!(parse(b"\"key 1\"}: 1234", ERROR_INVAL, 3, &[]));
    check!(parse(
        b"{\"key {1\": 1234}",
        3,
        3,
        &[Obj(0, 16, 1), Str("key {1", 1), Prim("1234")]
    ));
    check!(parse(b"{\"key 1\":{\"key 2\": 1234}", ERROR_PART, 5, &[]));
}

#[test]
fn test_object_key() {
    check!(parse(
        b"{\"key\": 1}",
        3,
        3,
        &[Obj(0, 10, 1), Str("key", 1), Prim("1")]
    ));
    #[cfg(feature = "strict")]
    {
        check!(parse(b"{true: 1}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{1: 1}", ERROR_INVAL, 3, &[]));
        check!(parse(b"{{\"key\": 1}: 2}", ERROR_INVAL, 5, &[]));
        check!(parse(b"{[1,2]: 2}", ERROR_INVAL, 5, &[]));
    }
}
