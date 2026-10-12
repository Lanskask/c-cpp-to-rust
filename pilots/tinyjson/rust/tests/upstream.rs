// The 22 upstream Catch test cases of tinyjson (src/test/tinyjsonTests.cpp), translated one-to-one into Rust
// (judge.md section 5 item 3). Same inputs, same expected values, through the adapter; one #[test] per TEST_CASE,
// named after it. Installed by run.py as <crate>/tests/upstream.rs next to <crate>/tests/adapter/mod.rs.
//
// Notes on the translation:
// - `U"..."` literals become `&str` converted to code points by `u32s`; `std::string` expectations become byte
//   slices (`&str` compared as bytes).
// - REQUIRE_THROWS(expr, Contains(..)): Catch passes the whole argument list as the expression, so the matcher is
//   evaluated and ignored; only "it throws" is checked. Kept as is (`throws`).
// - DEVIATION(port): D3. Five cases assert the decoding of a `\uXXXX` escape. On the reference build (GCC 16,
//   libstdc++, -O0) `parse_hex` returns an uninitialised value, U+0001 in the modal case (RULEBOOK D3, section 12);
//   those cases fail there (17 of 22 pass). The Rust port must give the reference value, so each of them keeps the
//   upstream expectation in a comment and asserts the reference-measured value. No other expected value changed.
//   Measured caveat (RUN-NOTES "Judge, Rust side"): inside the reference's own Catch binary the garbage is not
//   one value. With every D3 expectation set to U+0001, the reference passes 19 of 22: the three that still fail
//   there get U+0000 for some escapes (Hex Parsing: the 4th and 5th, Escape Parsing: both, the Serialization key:
//   not found). The value depends on stack residue of the tests that ran before. U+0001 is the modal value and the
//   one the pinned C++ driver gives, so it is the value asserted here (rule D3, RULEBOOK section 12).

mod adapter;

use adapter as a;
use adapter::{AErr, Arr, Obj, J, S};

/// D3: the value of every `\uXXXX` escape on the reference build (a single byte 0x01 once encoded as UTF-8).
const D3: u8 = 0x01;

fn u32s(s: &str) -> Vec<u32> {
    s.chars().map(|c| c as u32).collect()
}

fn stream(s: &str) -> S {
    S::new(&u32s(s))
}

fn gs(j: &J) -> Vec<u8> {
    a::get_string(j).unwrap()
}

fn gi(j: &J) -> i64 {
    a::get_integer(j).unwrap()
}

fn gd(j: &J) -> f64 {
    a::get_double(j).unwrap()
}

fn gb(j: &J) -> bool {
    a::get_bool(j).unwrap()
}

fn is_null(j: &J) -> bool {
    a::get_null(j).is_ok()
}

fn sz(j: &J) -> usize {
    a::size(j).unwrap()
}

fn ty(j: &J) -> i32 {
    a::type_code(j)
}

fn at(j: &mut J, i: i32) -> &mut J {
    a::index(j, i).unwrap()
}

fn key<'a>(j: &'a mut J, k: &str) -> &'a mut J {
    a::index_key(j, k.as_bytes()).unwrap()
}

fn add_m(j: &mut J, k: &str, v: J) {
    a::add_member(j, k.as_bytes(), v).unwrap();
}

fn s(v: &str) -> J {
    a::j_str(v.as_bytes())
}

/// REQUIRE_THROWS: the call returned an error.
fn throws<T>(r: Result<T, AErr>) -> AErr {
    match r {
        Err(e) => e,
        Ok(_) => panic!("expected an exception"),
    }
}

/// REQUIRE_THROWS_WITH(expr, Contains(needle))
fn throws_with<T>(r: Result<T, AErr>, needle: &str) {
    let e = throws(r);
    let what = String::from_utf8_lossy(&e.what).to_string();
    assert!(what.contains(needle), "what() = {:?} does not contain {:?}", what, needle);
}

// ---------------------------------------------------------------------------------------------------------------

#[test]
fn simple_json_constructor() {
    // strings
    let s1 = s("json string object");
    assert_eq!(gs(&s1), b"json string object");

    let s2 = s("");
    assert_eq!(gs(&s2), b"");

    let s3 = s("  ");
    assert_eq!(gs(&s3), b"  ");

    // numbers
    let i1 = a::j_int(3256);
    assert_eq!(gi(&i1), 3256);

    let i2 = a::j_int(-245);
    assert_eq!(gi(&i2), -245);

    let i3 = a::j_dbl(0.2356);
    assert_eq!(gd(&i3), 0.2356);

    let i4 = a::j_dbl(-0.255356);
    assert_eq!(gd(&i4), -0.255356);

    let i5 = a::j_int(0);
    assert_eq!(gi(&i5), 0);

    let i6 = a::j_int(-0);
    assert_eq!(gi(&i6), 0);

    // bools
    let b1 = a::j_bool(true);
    assert!(gb(&b1));

    let b2 = a::j_bool(false);
    assert!(!gb(&b2));

    // null
    let n1 = a::j_null();
    assert!(is_null(&n1));

    // arrays
    let elms: Arr = vec![
        a::j_int(2134),
        a::j_int(781450),
        a::j_dbl(0.187601),
        a::j_int(-984578),
        a::j_int(0),
        a::j_int(1),
        a::j_dbl(24.059),
    ];
    let mut a1 = a::j_arr(elms);
    assert_eq!(gi(at(&mut a1, 0)), 2134);
    assert_eq!(gi(at(&mut a1, 1)), 781450);
    assert_eq!(gd(at(&mut a1, 2)), 0.187601);
    assert_eq!(gi(at(&mut a1, 3)), -984578);
    assert_eq!(gi(at(&mut a1, 4)), 0);
    assert_eq!(gi(at(&mut a1, 5)), 1);
    assert_eq!(gd(at(&mut a1, 6)), 24.059);

    // object
    let mut o1 = a::j_obj(Obj::new());
    add_m(&mut o1, "p1", s("helloworld"));
    add_m(&mut o1, "p2", a::j_dbl(3.1415926));
    add_m(&mut o1, "p3", a::j_ll(9876543210)); // upstream: 9876543210 (long, D7), patched to 9876543210LL
    add_m(&mut o1, "p4", a::j_obj(Obj::new()));

    a::add_member(key(&mut o1, "p4"), b"p4_1", a::j_bool(true)).unwrap();
    a::add_member(key(&mut o1, "p4"), b"p4_2", a::j_dbl(-0.1356456)).unwrap();

    assert_eq!(gs(key(&mut o1, "p1")), b"helloworld");
    assert_eq!(gd(key(&mut o1, "p2")), 3.1415926);
    assert_eq!(gi(key(&mut o1, "p3")), 9876543210);

    assert!(gb(key(key(&mut o1, "p4"), "p4_1")));
    assert_eq!(gd(key(key(&mut o1, "p4"), "p4_2")), -0.1356456);
}

#[test]
fn tiny_json_string_parsing() {
    let mut u32strm = stream("\"2021\u{4E16}\u{754C}World \u{4F60}\u{597D}012 Hello!\"");
    let r = a::parse_string(&mut u32strm).unwrap();
    assert_eq!(gs(&r), "2021世界World 你好012 Hello!".as_bytes());

    let mut ss1 = stream("\"hello \\\\ world \"");
    let s1 = a::parse_string(&mut ss1).unwrap();
    assert_eq!(gs(&s1), b"hello \\ world ");

    let mut ss2 = stream("\"hello \\\" world \"");
    let s2 = a::parse_string(&mut ss2).unwrap();
    assert_eq!(gs(&s2), b"hello \" world ");

    let mut ss3 = stream("\" \\thello \\\"\\r world \\n \\/ \\b \\f \"");
    let s3 = a::parse_string(&mut ss3).unwrap();
    assert_eq!(gs(&s3), b" \thello \"\r world \n / \x08 \x0c ");

    let mut ss4 = stream("\"\"");
    let s4 = a::parse_string(&mut ss4).unwrap();
    assert_eq!(gs(&s4), b"");
}

#[test]
fn tiny_json_hex_parsing() {
    let mut u32strm = stream("\"2021\\u4E16\\u754CWorld \\u4F60\\u597D012 Hello! \\u0026\"");
    let s1 = a::parse_string(&mut u32strm).unwrap();
    // upstream: REQUIRE(s1.get_string() == "2021世界World 你好012 Hello! &");
    // DEVIATION(port): D3, every \uXXXX decodes to U+0001 on the reference build
    let mut want: Vec<u8> = Vec::new();
    want.extend_from_slice(b"2021");
    want.extend_from_slice(&[D3, D3]);
    want.extend_from_slice(b"World ");
    want.extend_from_slice(&[D3, D3]);
    want.extend_from_slice(b"012 Hello! ");
    want.push(D3);
    assert_eq!(gs(&s1), want);
}

#[test]
fn tiny_json_escape_parsing() {
    let mut u32strm = stream("\"\\\"a\\b\\/c\\bd\\fe  \\fg\\n\\r;\\t\\u4E16\\u754C\"");
    let s1 = a::parse_string(&mut u32strm).unwrap();
    // upstream: REQUIRE(s1.get_string() == "\"a\b/c\bd\fe  \fg\n\r;\t世界");
    // DEVIATION(port): D3, the two \u escapes decode to U+0001 each
    let mut want: Vec<u8> = b"\"a\x08/c\x08d\x0ce  \x0cg\n\r;\t".to_vec();
    want.extend_from_slice(&[D3, D3]);
    assert_eq!(gs(&s1), want);
}

#[test]
fn tiny_json_bool_parsing() {
    let mut bs1 = stream("true");
    let b1 = a::parse_bool(&mut bs1).unwrap();
    assert!(gb(&b1));

    let mut bs2 = stream("tRue ");
    let b2 = a::parse_bool(&mut bs2).unwrap();
    assert!(gb(&b2));

    let mut bs3 = stream(" FALSE");
    let b3 = a::parse_bool(&mut bs3).unwrap();
    assert!(!gb(&b3));

    let mut bs4 = stream(" falsE ");
    let b4 = a::parse_bool(&mut bs4).unwrap();
    assert!(!gb(&b4));
}

#[test]
fn tiny_json_null_parsing() {
    let mut ns1 = stream("null");
    let n1 = a::parse_null(&mut ns1).unwrap();
    assert!(is_null(&n1));

    let mut ns2 = stream(" NULL ");
    let n2 = a::parse_null(&mut ns2).unwrap();
    assert!(is_null(&n2));

    let mut ns3 = stream(" nUlL");
    let n3 = a::parse_null(&mut ns3).unwrap();
    assert!(is_null(&n3));
}

#[test]
fn tiny_json_number_parsing() {
    let mut ns1 = stream("0.124    \t");
    let n1 = a::parse_number(&mut ns1).unwrap();
    assert_eq!(0.124, gd(&n1));

    let mut ns2 = stream("-0.10921234567824 ");
    let n2 = a::parse_number(&mut ns2).unwrap();
    assert_eq!(-0.10921234567824, gd(&n2));

    let mut ns3 = stream("921.234567824");
    let n3 = a::parse_number(&mut ns3).unwrap();
    assert_eq!(921.234567824, gd(&n3));

    let mut ns4 = stream("  .987123654");
    let n4 = a::parse_number(&mut ns4).unwrap();
    assert_eq!(0.987123654, gd(&n4));

    let mut ns5 = stream(".23545E-34  ");
    let n5 = a::parse_number(&mut ns5).unwrap();
    assert_eq!(0.23545E-34, gd(&n5));

    let mut ns6 = stream("8.7894e+34");
    let n6 = a::parse_number(&mut ns6).unwrap();
    assert_eq!(8.7894e+34, gd(&n6));

    let mut ns7 = stream("9874563121555444");
    let n7 = a::parse_number(&mut ns7).unwrap();
    assert_eq!(9874563121555444, gi(&n7));

    let mut ns8 = stream("7895484569216311245.006");
    let n8 = a::parse_number(&mut ns8).unwrap();
    assert_eq!(7895484569216311245.006, gd(&n8));

    let mut ns9 = stream("00000");
    let n9 = a::parse_number(&mut ns9).unwrap();
    assert_eq!(0, gi(&n9));

    let mut ns10 = stream("-426981544458458");
    let n10 = a::parse_number(&mut ns10).unwrap();
    assert_eq!(-426981544458458, gi(&n10));
}

#[test]
fn tiny_json_hex_char_parsing() {
    // DEVIATION(port): D3. Upstream expects '$' (0x24), 'G' (0x47) and U+754C; the reference build gives the
    // uninitialised value, U+0001 in the modal case (measured `36 == 1` on the first assertion).
    let mut hs1 = stream("0024");
    let h1 = a::parse_hex(&mut hs1).unwrap();
    // upstream: REQUIRE(U'$' == h1);
    assert_eq!(D3 as u32, h1);

    let mut hs2 = stream("0047");
    let h2 = a::parse_hex(&mut hs2).unwrap();
    // upstream: REQUIRE(U'G' == h2);
    assert_eq!(D3 as u32, h2);

    // 754C
    let mut hs3 = stream("754C");
    let h3 = a::parse_hex(&mut hs3).unwrap();
    // upstream: REQUIRE(U'\x754C' == h3);
    assert_eq!(D3 as u32, h3);
}

#[test]
fn tiny_json_array_parsing() {
    let mut as1 = stream(" [124, -2.534, \"hello world  \", null, false ]");
    let mut a1 = a::parse_array(&mut as1).unwrap();

    assert_eq!(5, sz(&a1));
    assert_eq!(a::T_ARRAY, ty(&a1));
    assert_eq!(124, gi(at(&mut a1, 0)));
    assert_eq!(-2.534, gd(at(&mut a1, 1)));
    assert_eq!(b"hello world  ".to_vec(), gs(at(&mut a1, 2)));
    assert!(is_null(at(&mut a1, 3)));
    assert!(!gb(at(&mut a1, 4)));

    let mut as2 = stream(" [  ] ");
    let a2 = a::parse_array(&mut as2).unwrap();

    assert_eq!(0, sz(&a2));
    assert_eq!(a::T_ARRAY, ty(&a2));

    let mut as3 = stream(" [ \" elem1\" ] ");
    let mut a3 = a::parse_array(&mut as3).unwrap();

    assert_eq!(1, sz(&a3));
    assert_eq!(a::T_ARRAY, ty(&a3));
    assert_eq!(b" elem1".to_vec(), gs(at(&mut a3, 0)));

    let mut as4 = stream(" [ \" elem1\", {}, {\"p1\" : 498723123} ] ");
    let mut a4 = a::parse_array(&mut as4).unwrap();

    assert_eq!(3, sz(&a4));
    assert_eq!(a::T_ARRAY, ty(&a4));
    assert_eq!(b" elem1".to_vec(), gs(at(&mut a4, 0)));
    assert_eq!(0, sz(at(&mut a4, 1)));
    assert_eq!(1, sz(at(&mut a4, 2)));
    assert_eq!(498723123, gi(key(at(&mut a4, 2), "p1")));

    let mut as5 = stream(" [[]  ] ");
    let mut a5 = a::parse_array(&mut as5).unwrap();

    assert_eq!(ty(&a5), a::T_ARRAY);
    assert_eq!(sz(&a5), 1);
    assert_eq!(ty(at(&mut a5, 0)), a::T_ARRAY);
    assert_eq!(sz(at(&mut a5, 0)), 0);

    let mut as6 = stream(" [[1984.2020], 2020.1984  ] ");
    let mut a6 = a::parse_array(&mut as6).unwrap();

    assert_eq!(ty(&a6), a::T_ARRAY);
    assert_eq!(sz(&a6), 2);
    assert_eq!(gd(at(&mut a6, 1)), 2020.1984);
    assert_eq!(ty(at(&mut a6, 0)), a::T_ARRAY);
    assert_eq!(sz(at(&mut a6, 0)), 1);
    assert_eq!(gd(at(at(&mut a6, 0), 0)), 1984.2020);

    let mut as7 = stream(" [[1984, \"1984\"], 2020, 1984 ] ");
    let mut a7 = a::parse_array(&mut as7).unwrap();
    assert_eq!(ty(&a7), a::T_ARRAY);
    assert_eq!(sz(&a7), 3);
    assert_eq!(ty(at(&mut a7, 0)), a::T_ARRAY);
    assert_eq!(gi(at(at(&mut a7, 0), 0)), 1984);
    assert_eq!(gs(at(at(&mut a7, 0), 1)), b"1984");
    assert_eq!(gi(at(&mut a7, 1)), 2020);
    assert_eq!(gi(at(&mut a7, 2)), 1984);
}

#[test]
fn tiny_json_object_parsing() {
    let mut os1 = stream(" {}");
    let o1 = a::parse_object(&mut os1).unwrap();
    assert_eq!(ty(&o1), a::T_OBJECT);
    assert_eq!(sz(&o1), 0);

    let mut os2 = stream(" {\"p1\" : 987135}");
    let mut o2 = a::parse_object(&mut os2).unwrap();
    assert_eq!(ty(&o2), a::T_OBJECT);
    assert_eq!(sz(&o2), 1);
    assert_eq!(gi(key(&mut o2, "p1")), 987135);

    let mut os3 = stream(" {\"p1\" : 987135, \"P_1\" : true}");
    let mut o3 = a::parse_object(&mut os3).unwrap();
    assert_eq!(ty(&o3), a::T_OBJECT);
    assert_eq!(sz(&o3), 2);
    assert_eq!(gi(key(&mut o3, "p1")), 987135);
    assert!(gb(key(&mut o3, "P_1")));

    let mut os4 = stream(" {\"p1\" : -0.26897415, \"P_1\" : { \"_53245\": -235235 } }");
    let mut o4 = a::parse_object(&mut os4).unwrap();
    assert_eq!(ty(&o4), a::T_OBJECT);
    assert_eq!(sz(&o4), 2);
    assert_eq!(gd(key(&mut o4, "p1")), -0.26897415);
    assert_eq!(ty(key(&mut o4, "P_1")), a::T_OBJECT);
    assert_eq!(gi(key(key(&mut o4, "P_1"), "_53245")), -235235);
}

#[test]
fn tiny_json_member_parsing() {
    let mut ss1 = stream("\"\u{4F60}\u{202F2} hello\"");
    let member_name = a::parse_member(&mut ss1).unwrap();
    assert_eq!(member_name, "你𠋲 hello".as_bytes());

    let mut ss2 = stream("\"_hello world!\"");
    let member_name = a::parse_member(&mut ss2).unwrap();
    assert_eq!(member_name, b"_hello world!");

    let mut ss3 = stream("\"__\\u0069_\\u005E\"");
    let member_name = a::parse_member(&mut ss3).unwrap();
    // upstream: REQUIRE(member_name == "__i_^");
    // DEVIATION(port): D3, each \uXXXX decodes to U+0001
    assert_eq!(member_name, [b'_', b'_', D3, b'_', D3]);

    let mut ss4 = stream("\"hello \\\"world\\\"\"");
    let member_name = a::parse_member(&mut ss4).unwrap();
    assert_eq!(member_name, b"hello \"world\"");

    let mut ss5 = stream("\"hello { world }\"");
    let member_name = a::parse_member(&mut ss5).unwrap();
    assert_eq!(member_name, b"hello { world }");

    let mut ss6 = stream("\"hello ,{ world }\"");
    let member_name = a::parse_member(&mut ss6).unwrap();
    assert_eq!(member_name, b"hello ,{ world }");

    let mut ss7 = stream("\"[hello ,{ world }]\"");
    let member_name = a::parse_member(&mut ss7).unwrap();
    assert_eq!(member_name, b"[hello ,{ world }]");

    let mut ss8 = stream("\"[\\thello ,{ world }]\"");
    let member_name = a::parse_member(&mut ss8).unwrap();
    assert_eq!(member_name, b"[\thello ,{ world }]");
}

#[test]
fn simple_json_serialization() {
    let mut aj = a::parse(b"{}").unwrap();

    add_m(&mut aj, "p1", a::j_dbl(13525.4235));
    add_m(&mut aj, "p2", s("abcdefg hijklmnop rsq  uvwxyz "));
    add_m(&mut aj, "_p3_", a::j_bool(false));
    add_m(&mut aj, "p4_____________", a::j_dbl(-0.9876540321));
    add_m(&mut aj, "_______p5", a::j_arr(vec![s("abc"), a::j_dbl(0.31424), a::j_bool(false)]));
    add_m(&mut aj, "示例一", a::j_bool(false));
    add_m(&mut aj, "你好 hello", s("world\\u0039"));
    add_m(&mut aj, "世界__\\u0069_\\u005E", s("你好"));

    let a1 = a::to_string(&aj);
    let mut b = a::parse(&a1).unwrap();

    assert_eq!(sz(&b), 8);
    assert_eq!(gd(key(&mut b, "p1")), 13525.4235);
    assert_eq!(gs(key(&mut b, "p2")), b"abcdefg hijklmnop rsq  uvwxyz ");
    assert!(!gb(key(&mut b, "_p3_")));
    // Approx(-0.9876540321).margin(0.00001)
    assert!((gd(key(&mut b, "p4_____________")) - (-0.9876540321)).abs() <= 0.00001);

    assert_eq!(ty(key(&mut b, "_______p5")), a::T_ARRAY);
    assert_eq!(gs(at(key(&mut b, "_______p5"), 0)), b"abc");
    assert_eq!(gd(at(key(&mut b, "_______p5"), 1)), 0.31424);
    assert!(!gb(at(key(&mut b, "_______p5"), 2)));

    assert!(!gb(key(&mut b, "示例一")));
    // upstream: REQUIRE(b["你好 hello"].get_string() == "world9");
    // DEVIATION(port): D3, `\u0039` in the stored text decodes to U+0001 on parse. This is the assertion that
    // fails first on the reference; the ones below it never ran there, so their values are derived by the same rule.
    let mut want = b"world".to_vec();
    want.push(D3);
    assert_eq!(gs(key(&mut b, "你好 hello")), want);
    // upstream: REQUIRE(b["世界__i_^"].get_string() == "你好");  (the key is "世界__\x01_\x01" after D3)
    let mut k = "世界__".as_bytes().to_vec();
    k.extend_from_slice(&[D3, b'_', D3]);
    assert_eq!(gs(a::index_key(&mut b, &k).unwrap()), "你好".as_bytes());

    let c = a::j_obj(Obj::new());
    assert_eq!(a::to_string(&c), b"{}");

    let d = a::j_arr(Vec::new());
    assert_eq!(a::to_string(&d), b"[]");
}

#[test]
fn simple_json_json_class_test() {
    let a_ = a::j_null();
    let b_ = a::j_null();
    assert!(a::eq(&a_, &b_));

    let a1 = a::j_int(1984);
    let b1 = a::j_int(1984);
    assert!(a::eq(&a1, &b1));

    let mut a2 = a::j_dbl(-0.326345);
    let b2 = a::j_dbl(-0.326345);
    assert!(a::eq(&a2, &b2));

    a::assign(&mut a2, &a::j_dbl(0.326345));
    assert!(a::ne(&a2, &b2));

    let mut a3 = a::j_bool(true);
    let b3 = a::j_bool(true);
    assert!(a::eq(&a3, &b3));

    a::assign(&mut a3, &a::j_bool(false));
    assert!(a::ne(&a3, &b3));

    let mut a4 = s("hello world");
    let b4 = s("hello world");
    assert!(a::eq(&a4, &b4));

    a::assign(&mut a4, &s("hello world!"));
    assert!(a::ne(&a4, &b4));

    let mut a5 = a::j_obj(Obj::new());
    let mut b5 = a::j_obj(Obj::new());
    assert!(a::eq(&a5, &b5));

    add_m(&mut a5, "p1", s("hello1984"));
    assert!(a::ne(&a5, &b5));

    add_m(&mut b5, "p1", s("hello1984"));
    assert!(a::eq(&a5, &b5));

    let mut a6 = a::j_arr(Vec::new());
    let b6 = a::j_arr(Vec::new());
    assert!(a::eq(&a6, &b6));

    a::add_element(&mut a6, a::j_int(1984)).unwrap();
    assert!(a::ne(&a6, &b6));

    assert!(a::ne(&a1, &a2));
    assert!(a::ne(&a1, &a3));
    assert!(a::ne(&a1, &a4));
    assert!(a::ne(&a1, &a5));
    assert!(a::ne(&a1, &a6));

    assert!(a::ne(&a2, &a3));
    assert!(a::ne(&a2, &a4));
    assert!(a::ne(&a2, &a5));
    assert!(a::ne(&a2, &a6));

    assert!(a::ne(&a3, &a4));
    assert!(a::ne(&a3, &a5));
    assert!(a::ne(&a3, &a6));

    assert!(a::ne(&a4, &a5));
    assert!(a::ne(&a4, &a6));

    assert!(a::ne(&a5, &a6));

    let c = a::j_int(1984);
    let d = a::copy(&c);
    assert!(a::eq(&c, &d));
    assert_eq!(gi(&d), 1984);

    let e = s("1984");
    assert_eq!(gs(&e), b"1984");
}

#[test]
fn simple_json_nubmer_parsing_failure() {
    let mut ns1 = stream("0.124abc");
    throws_with(a::parse_number(&mut ns1), "Unexpected number");

    let mut ns2 = stream("124abc");
    throws_with(a::parse_number(&mut ns2), "Unexpected number");

    let mut ns3 = stream("124 000");
    throws_with(a::parse_number(&mut ns3), "Unexpected number");

    let mut ns4 = stream(".124 000");
    throws_with(a::parse_number(&mut ns4), "Unexpected number");

    let mut ns5 = stream("not a number 00.23");
    throws(a::parse_number(&mut ns5));
}

#[test]
fn simple_json_bool_parsing_failure() {
    let mut bs1 = stream("falt");
    throws_with(a::parse_bool(&mut bs1), "invalid boolean string");

    let mut bs2 = stream("Falsa");
    throws_with(a::parse_bool(&mut bs2), "invalid boolean string");
}

#[test]
fn simple_json_string_parsing_failure() {
    let mut ss1 = stream("\"hello world \\\"");
    throws_with(a::parse_string(&mut ss1), "expected char '\"'");

    let mut ss2 = stream("\"hello world \\u00A");
    throws_with(a::parse_string(&mut ss2), "not hex number");

    let mut ss3 = stream("\"hello \\a world ");
    throws_with(
        a::parse_string(&mut ss3),
        "backslash is followed by invalid character",
    );

    let mut ss4 = stream("\"hello \\uworld\"");
    throws_with(a::parse_string(&mut ss4), "not hex number");
}

#[test]
fn simple_json_array_parsing_failure() {
    // REQUIRE_THROWS(call, Contains(..)): the matcher is ignored by Catch, only the throw is required
    let mut as1 = stream("[a]");
    throws(a::parse_array(&mut as1));

    let mut as2 = stream("[,]");
    throws(a::parse_array(&mut as2)); // Contains("invalid array format")

    let mut as3 = stream("[1.5,]");
    throws(a::parse_array(&mut as3)); // Contains("parsing error: unexpected character")

    let mut as4 = stream("[\"abc\":]");
    throws(a::parse_array(&mut as4)); // Contains("parsing error: unexpected character")

    let mut as5 = stream("[3.14 , \"\", a]");
    throws(a::parse_array(&mut as5)); // Contains("parsing error: unexpected character")

    let mut as6 = stream("[[3.14 , \"\", a]");
    throws(a::parse_array(&mut as6)); // Contains("invalid array formatr")
}

#[test]
fn simple_json_object_parsing_failure() {
    let mut os1 = stream("{a}");
    throws(a::parse_object(&mut os1));

    let mut os2 = stream("{");
    throws(a::parse_object(&mut os2));

    let mut os3 = stream("{1}");
    throws(a::parse_object(&mut os3));

    let mut os4 = stream("{\"hello\"}");
    throws(a::parse_object(&mut os4));

    let mut os5 = stream("{\"hello\":}");
    throws(a::parse_object(&mut os5));

    let mut os6 = stream("{\"hello: 124 }");
    throws(a::parse_object(&mut os6));
}

#[test]
fn simple_json_member_parsing_failure() {
    let mut ms1 = stream("\"hello");
    throws(a::parse_member(&mut ms1));
}

#[test]
fn simple_json_json_parsing_failure() {
    throws(a::parse(b""));
    throws(a::parse(b"{"));
    throws(a::parse(b"}"));
    throws(a::parse(b"{}}"));
    throws(a::parse(b"]"));
    throws(a::parse(b"["));
    throws(a::parse(b"[]]"));
    throws(a::parse(b"hello"));
}

#[test]
fn simple_json_json_parsing_1() {
    let mut j = a::parse(
        br#"
        {
        "command": "launch",
        "arguments": {
            "name": "Debug",
            "type": "windbg",
            "request": "launch",
            "program": "C:\\VC\\helloworld/helloworld.exe",
            "args": [

            ],
            "cwd": "C:\\VC\\helloworld/",
            "debugServer": 8888,
            "__configurationTarget": 5,
            "__sessionId": "df7ac0b9-4d27-4aba-a73f-7fd8207bb590"
        },
        "type": "request",
        "seq": 2
        }
    "#,
    )
    .unwrap();

    assert_eq!(ty(&j), a::T_OBJECT);
    assert_eq!(gs(key(&mut j, "command")), b"launch");
    assert_eq!(gs(key(key(&mut j, "arguments"), "name")), b"Debug");
    assert_eq!(gs(key(key(&mut j, "arguments"), "type")), b"windbg");
    assert_eq!(gs(key(key(&mut j, "arguments"), "request")), b"launch");
    assert_eq!(
        gs(key(key(&mut j, "arguments"), "program")),
        b"C:\\VC\\helloworld/helloworld.exe"
    );
    assert_eq!(sz(key(key(&mut j, "arguments"), "args")), 0);
    assert_eq!(gs(key(key(&mut j, "arguments"), "cwd")), b"C:\\VC\\helloworld/");
    assert_eq!(gi(key(key(&mut j, "arguments"), "debugServer")), 8888);
    assert_eq!(gi(key(key(&mut j, "arguments"), "__configurationTarget")), 5);
    assert_eq!(
        gs(key(key(&mut j, "arguments"), "__sessionId")),
        b"df7ac0b9-4d27-4aba-a73f-7fd8207bb590"
    );
    assert_eq!(gs(key(&mut j, "type")), b"request");
    assert_eq!(gi(key(&mut j, "seq")), 2);
}

#[test]
fn simple_json_json_parsing_2() {
    let mut j = a::parse(
        br#"
        ["computer"]
    "#,
    )
    .unwrap();

    assert_eq!(gs(at(&mut j, 0)), b"computer");
}
