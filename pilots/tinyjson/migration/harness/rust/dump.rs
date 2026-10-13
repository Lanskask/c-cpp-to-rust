// Differential driver for tinyjson, Rust side (ticket 02, judge.md section 5 item 2).
//
// A line-for-line port of migration/harness/dump.cpp: same sections in the same order, same labels, same helpers,
// reaching the crate only through the adapter. Output grammar (one record per line):
//   <label> = <value>                    a call that returned
//   <label> ! <class> <quoted what()>    a call that threw (class: out_of_range, invalid_argument, runtime_error)
// Strings print as "..." with \\ \" escaped and every byte outside 0x20..0x7e as \xHH. Integers print in decimal.
// A double prints as its 16-digit IEEE bit pattern (0x%016x), a NaN as "nan"; to_string text is printed as is.
//
// Not ported (driver-only on the C++ side, nothing to port): `pin_d3_value`, the re-exec that pins the
// uninitialised value of D3. The Rust crate gives U+0001 for every \u escape (DEVIATION(port): D3).
// `.after` records: the C++ driver calls `clear()` before `tellg()` and `peek()`; the Rust stream has no state bits.
//
// Installed by run.py as <crate>/examples/dump.rs next to <crate>/tests/adapter/mod.rs. Exit 0 after a normal run,
// 2 for a driver error. A panic inside the crate exits 101 and the Judge records it as a difference.

#[path = "../tests/adapter/mod.rs"]
mod adapter;

use adapter as a;
use adapter::{AErr, Arr, Obj, J, R, S};
use std::io::Write;
use std::sync::Mutex;

// ---------------------------------------------------------------------------------------------------------------
// output (buffered; flushed at the end and by the panic hook, so a crash still leaves the records so far)
// ---------------------------------------------------------------------------------------------------------------

static OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

fn flush_out() {
    let mut g = match OUT.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let so = std::io::stdout();
    let mut lock = so.lock();
    let _ = lock.write_all(&g);
    let _ = lock.flush();
    g.clear();
}

// ---------------------------------------------------------------------------------------------------------------
// printing helpers (one per kind of value)
// ---------------------------------------------------------------------------------------------------------------

fn num(v: i64) -> String {
    format!("{}", v)
}

fn q(s: &[u8]) -> String {
    let mut r = String::from("\"");
    for &c in s {
        if c == b'\\' {
            r.push_str("\\\\");
        } else if c == b'"' {
            r.push_str("\\\"");
        } else if (0x20..=0x7e).contains(&c) {
            r.push(c as char);
        } else {
            r.push_str(&format!("\\x{:02x}", c));
        }
    }
    r.push('"');
    r
}

fn dbl(d: f64) -> String {
    if d.is_nan() {
        return "nan".to_string();
    }
    format!("0x{:016x}", d.to_bits())
}

fn code(c: u32) -> String {
    if c == 0xFFFF_FFFF {
        return "EOF".to_string();
    }
    format!("U+{:04X}", c)
}

fn line(label: &str, value: &str) {
    let mut g = match OUT.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    g.extend_from_slice(label.as_bytes());
    g.extend_from_slice(b" = ");
    g.extend_from_slice(value.as_bytes());
    g.push(b'\n');
    let full = g.len() > 1 << 16;
    drop(g);
    if full {
        flush_out();
    }
}

fn fail(label: &str, cls: &str, what: &[u8]) {
    let mut g = match OUT.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    g.extend_from_slice(label.as_bytes());
    g.extend_from_slice(b" ! ");
    g.extend_from_slice(cls.as_bytes());
    g.push(b' ');
    g.extend_from_slice(q(what).as_bytes());
    g.push(b'\n');
}

/// label + suffix (C++: `p + ".get_string"`)
fn cat(p: &str, suffix: &str) -> String {
    let mut s = String::from(p);
    s.push_str(suffix);
    s
}

// Runs f (returns the printed value); prints "= value" or "! class what".
fn attempt<F: FnOnce() -> R<String>>(label: &str, f: F) {
    match f() {
        Ok(v) => line(label, &v),
        Err(AErr { class, what }) => fail(label, class, &what),
    }
}

fn kind(j: &J) -> String {
    format!("{} {}", num(a::type_code(j) as i64), a::type_name(j))
}

/// the bytes of a C string: up to the first NUL
fn c_str(s: &[u8]) -> &[u8] {
    match s.iter().position(|&b| b == 0) {
        Some(i) => &s[..i],
        None => s,
    }
}

// ---------------------------------------------------------------------------------------------------------------
// equality guard: operator== is UB for the pairs of D1 (the Rust port returns false there), so the driver only
// calls it on pairs where the C++ result is defined (or D2, which the port reproduces).
// ---------------------------------------------------------------------------------------------------------------

fn rank_of(t: i32) -> i32 {
    match t {
        a::T_ARRAY => 0,
        a::T_OBJECT => 1,
        a::T_NULL => 2,
        a::T_STRING => 3,
        a::T_BOOLEAN => 4,
        a::T_NUMBER_INTEGER => 5,
        _ => 6,
    }
}

fn eq_safe(x: &J, y: &J) -> bool {
    let ta = a::type_code(x);
    let tb = a::type_code(y);
    if ta == tb {
        if ta == a::T_ARRAY {
            let ax = a::get_array(x).expect("type checked");
            let ay = a::get_array(y).expect("type checked");
            if ax.len() != ay.len() {
                return true;
            }
            for i in 0..ax.len() {
                if !eq_safe(&ax[i], &ay[i]) {
                    return false;
                }
            }
            return true;
        }
        if ta == a::T_OBJECT {
            let ox = a::get_object(x).expect("type checked");
            let oy = a::get_object(y).expect("type checked");
            if ox.len() != oy.len() {
                return true;
            }
            for ((kx, vx), (ky, vy)) in ox.iter().zip(oy.iter()) {
                if kx != ky {
                    return true;
                }
                if !eq_safe(vx, vy) {
                    return false;
                }
            }
            return true;
        }
        return true;
    }
    if rank_of(ta) > rank_of(tb) {
        return true;
    }
    if (ta == a::T_ARRAY || ta == a::T_OBJECT) && tb == a::T_NULL {
        return true;
    }
    if ta == a::T_NUMBER_INTEGER && tb == a::T_NUMBER_DOUBLE {
        return true;
    }
    false
}

// "eq" record for (x, y): == both ways and !=, or "skip" for the pairs of D1.
fn eq_line(label: &str, x: &J, y: &J) {
    if !eq_safe(x, y) || !eq_safe(y, x) {
        line(label, "skip");
        return;
    }
    let ab = a::eq(x, y);
    let ba = a::eq(y, x);
    let ne = a::ne(x, y);
    line(
        label,
        &format!(
            "ab={} ba={} ne={}",
            num(ab as i64),
            num(ba as i64),
            num(ne as i64)
        ),
    );
}

// ---------------------------------------------------------------------------------------------------------------
// section node: everything observable on one value, then its children
// ---------------------------------------------------------------------------------------------------------------

fn path_key(path: &str, key: &[u8]) -> String {
    format!("{}[{}]", path, q(key))
}

fn path_index(path: &str, i: usize) -> String {
    format!("{}[{}]", path, num(i as i64))
}

fn dump_node(n: &mut J, p: &str) {
    line(&cat(p, ".type"), &kind(n));

    attempt(&cat(p, ".get_string"), || Ok(q(&a::get_string(n)?)));
    attempt(&cat(p, ".get_integer"), || Ok(num(a::get_integer(n)?)));
    attempt(&cat(p, ".get_double"), || Ok(dbl(a::get_double(n)?)));
    attempt(&cat(p, ".get_bool"), || Ok(num(a::get_bool(n)? as i64)));
    attempt(&cat(p, ".get_null"), || {
        a::get_null(n)?;
        Ok("nullptr".to_string())
    });
    attempt(&cat(p, ".get_object"), || {
        Ok(format!("size={}", num(a::get_object(n)?.len() as i64)))
    });
    attempt(&cat(p, ".get_array"), || {
        Ok(format!("size={}", num(a::get_array(n)?.len() as i64)))
    });
    attempt(&cat(p, ".size"), || Ok(num(a::size(n)? as i64)));

    attempt(&cat(p, ".conv_string"), || Ok(q(&a::conv_string(n)?)));
    attempt(&cat(p, ".conv_double"), || Ok(dbl(a::conv_double(n)?)));
    attempt(&cat(p, ".conv_long_long"), || {
        Ok(num(a::conv_long_long(n)?))
    });
    attempt(&cat(p, ".conv_bool"), || Ok(num(a::conv_bool(n)? as i64)));

    attempt(&cat(p, ".to_string"), || Ok(q(&a::to_string(n))));

    // lookups by key and index (object, array and every other type)
    attempt(&cat(p, ".has_member(absent)"), || {
        Ok(num(a::has_member(n, b"\x01 absent")? as i64))
    });
    attempt(&cat(p, ".has_member(empty)"), || {
        Ok(num(a::has_member(n, b"")? as i64))
    });
    attempt(&cat(p, ".op[absent]"), || {
        Ok(kind(a::index_key(n, b"\x01 absent")?))
    });
    attempt(&cat(p, ".op[-1]"), || Ok(kind(a::index(n, -1)?)));
    attempt(&cat(p, ".op[0]"), || Ok(kind(a::index(n, 0)?)));
    attempt(&cat(p, ".op[1]"), || Ok(kind(a::index(n, 1)?)));
    if a::type_code(n) == a::T_ARRAY {
        let sz = a::size(n).expect("type checked") as i64;
        attempt(&cat(p, ".op[size-1]"), || {
            Ok(kind(a::index(n, (sz - 1) as i32)?))
        });
        attempt(&cat(p, ".op[size]"), || Ok(kind(a::index(n, sz as i32)?)));
        attempt(&cat(p, ".op[size+1]"), || {
            Ok(kind(a::index(n, (sz + 1) as i32)?))
        });
    }
    if a::type_code(n) == a::T_OBJECT {
        let m: Obj = a::get_object(n).expect("type checked");
        for (key, _) in m.iter() {
            let kp = path_key(p, key);
            attempt(&cat(&kp, ".has_member"), || {
                Ok(num(a::has_member(n, key)? as i64))
            });
            attempt(&cat(&kp, ".op[c_str]"), || {
                Ok(kind(a::index_key(n, c_str(key))?))
            });
        }
    }

    // copy, equality with the copy, then the mutators on the copy (the original stays untouched)
    let mut c = a::copy(n);
    eq_line(&cat(p, ".copy_eq"), n, &c);
    attempt(&cat(p, ".copy.add_member"), || {
        a::add_member(&mut c, b"k", a::j_int(1))?;
        Ok(q(&a::to_string(&c)))
    });
    let mut c2 = a::copy(n);
    attempt(&cat(p, ".copy.add_element"), || {
        a::add_element(&mut c2, a::j_str(b"e"))?;
        Ok(q(&a::to_string(&c2)))
    });
    let mut c3 = a::copy(n);
    attempt(&cat(p, ".copy.add_member_overwrite"), || {
        a::add_member(&mut c3, b"", a::j_bool(true))?;
        a::add_member(&mut c3, b"", a::j_bool(false))?;
        Ok(q(&a::to_string(&c3)))
    });

    // children
    if a::type_code(n) == a::T_ARRAY {
        let mut arr: Arr = a::get_array(n).expect("type checked");
        for (i, e) in arr.iter_mut().enumerate() {
            dump_node(e, &path_index(p, i));
        }
    }
    if a::type_code(n) == a::T_OBJECT {
        let mut m: Obj = a::get_object(n).expect("type checked");
        for (key, v) in m.iter_mut() {
            dump_node(v, &path_key(p, key));
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// section streams
// ---------------------------------------------------------------------------------------------------------------

fn u32_text(s: &[u32]) -> String {
    let mut r = String::from("[");
    for (i, &c) in s.iter().enumerate() {
        if i != 0 {
            r.push(' ');
        }
        r.push_str(&code(c));
    }
    r.push(']');
    r
}

fn after(label: &str, ss: &mut S) {
    let pos = ss.pos();
    let next = ss.peek();
    line(
        &cat(label, ".after"),
        &format!("pos={} next={}", num(pos), code(next)),
    );
}

fn probe_json(label: &str, src: &[u32], f: fn(&mut S) -> R<J>) {
    let mut ss = S::new(src);
    attempt(label, || {
        let r = f(&mut ss)?;
        Ok(format!("{} {}", kind(&r), q(&a::to_string(&r))))
    });
    after(label, &mut ss);
}

fn probe_string(label: &str, src: &[u32], f: fn(&mut S) -> R<Vec<u8>>) {
    let mut ss = S::new(src);
    attempt(label, || Ok(q(&f(&mut ss)?)));
    after(label, &mut ss);
}

fn probe_char(label: &str, src: &[u32], f: fn(&mut S) -> R<u32>) {
    let mut ss = S::new(src);
    attempt(label, || Ok(code(f(&mut ss)?)));
    after(label, &mut ss);
}

fn probe_char_plain(label: &str, src: &[u32], f: fn(&mut S) -> u32) {
    let mut ss = S::new(src);
    attempt(label, || Ok(code(f(&mut ss))));
    after(label, &mut ss);
}

fn probe_int(label: &str, src: &[u32], f: fn(&mut S) -> i32) {
    let mut ss = S::new(src);
    attempt(label, || Ok(code(f(&mut ss) as u32)));
    after(label, &mut ss);
}

fn probe_void(label: &str, src: &[u32], f: fn(&mut S) -> R<()>) {
    let mut ss = S::new(src);
    attempt(label, || {
        f(&mut ss)?;
        Ok("void".to_string())
    });
    after(label, &mut ss);
}

fn skip_space_void(s: &mut S) -> R<()> {
    a::skip_space(s);
    Ok(())
}
fn skip_quote(s: &mut S) -> R<()> {
    a::skip_char(s, b'"' as u32)
}
fn skip_colon(s: &mut S) -> R<()> {
    a::skip_char(s, b':' as u32)
}

fn stream_probes(tag: &str, src: &[u32]) {
    line(&cat(tag, ".src"), &u32_text(src));
    probe_json(&cat(tag, ".parse_value"), src, a::parse_value);
    probe_json(&cat(tag, ".parse_object"), src, a::parse_object);
    probe_json(&cat(tag, ".parse_array"), src, a::parse_array);
    probe_json(&cat(tag, ".parse_string"), src, a::parse_string);
    probe_json(&cat(tag, ".parse_number"), src, a::parse_number);
    probe_json(&cat(tag, ".parse_bool"), src, a::parse_bool);
    probe_json(&cat(tag, ".parse_null"), src, a::parse_null);
    probe_string(&cat(tag, ".parse_member"), src, a::parse_member);
    probe_char(&cat(tag, ".escape_char"), src, a::escape_char);
    probe_char(&cat(tag, ".parse_hex"), src, a::parse_hex);
    probe_char_plain(&cat(tag, ".get_next_non_space"), src, a::get_next_non_space);
    probe_int(
        &cat(tag, ".peek_next_non_space"),
        src,
        a::peek_next_non_space,
    );
    probe_void(&cat(tag, ".skip_space"), src, skip_space_void);
    probe_void(&cat(tag, ".skip_char(quote)"), src, skip_quote);
    probe_void(&cat(tag, ".skip_char(colon)"), src, skip_colon);
}

fn section_streams(data: &[u8]) {
    let mut utf8: Vec<u32> = Vec::new();
    let mut utf8_ok = false;
    attempt("streams.U8ToU32", || {
        utf8 = a::u8_to_u32(data)?;
        utf8_ok = true;
        Ok(u32_text(&utf8))
    });
    if utf8_ok {
        attempt("streams.U32ToU8(utf8)", || Ok(q(&a::u32_to_u8(&utf8)?)));
        stream_probes("streams.utf8", &utf8);
    }

    // every byte as one code point (reaches 0x80..0xFF in the u32 parsers without the UTF-8 decoder)
    let latin1: Vec<u32> = data.iter().map(|&b| b as u32).collect();
    attempt("streams.U32ToU8(latin1)", || Ok(q(&a::u32_to_u8(&latin1)?)));
    stream_probes("streams.latin1", &latin1);

    // 4 bytes little endian per code point (reaches surrogates and values above 0x10FFFF)
    let mut wide: Vec<u32> = Vec::new();
    let mut i = 0;
    while i + 4 <= data.len() {
        let mut v: u32 = 0;
        for k in 0..4 {
            v |= (data[i + k] as u32) << (8 * k);
        }
        wide.push(v);
        i += 4;
    }
    line("streams.wide", &u32_text(&wide));
    attempt("streams.U32ToU8(wide)", || Ok(q(&a::u32_to_u8(&wide)?)));
    // ticket 04 review: the parser functions on a stream that holds surrogates and values above 0x10FFFF (D4)
    stream_probes("streams.wide", &wide);

    // scalar helpers on the whole input (they take std::string by value)
    attempt("streams.to_bool", || Ok(num(a::to_bool(data)? as i64)));
    attempt("streams.to_integer", || Ok(num(a::to_integer(data)?)));
    attempt("streams.to_double", || Ok(dbl(a::to_double(data)?)));
    attempt("streams.trim", || Ok(q(&a::trim(data))));
}

// ---------------------------------------------------------------------------------------------------------------
// section build: construction API driven from the input bytes
// ---------------------------------------------------------------------------------------------------------------

fn make_elem(b: u8) -> J {
    match b % 7 {
        0 => a::j_str(&[b]),
        1 => a::j_int(b as i32),
        2 => a::j_dbl((b as f64) / 8.0 - 10.0),
        3 => a::j_bool((b & 1) != 0),
        4 => a::j_null(),
        5 => a::j_arr(Vec::new()),
        _ => a::j_obj(Obj::new()),
    }
}

fn le_bits(d: &[u8], n: usize) -> i64 {
    let mut v: u64 = 0;
    let mut i = 0;
    while i < n && i < d.len() {
        v |= (d[i] as u64) << (8 * i);
        i += 1;
    }
    v as i64
}

fn section_build(data: &[u8]) {
    // scalars
    let mut js = a::j_str(data);
    dump_node(&mut js, "build.string");
    let rev: Vec<u8> = data.iter().rev().cloned().collect();
    let mut jr = a::j_str(&rev);
    dump_node(&mut jr, "build.string_rvalue");
    let mut jc = a::j_str(c_str(data));
    dump_node(&mut jc, "build.char_array");

    let mut ji = a::j_int(le_bits(data, 4) as i32);
    dump_node(&mut ji, "build.int");
    let mut jl = a::j_ll(le_bits(data, 8));
    dump_node(&mut jl, "build.long_long");
    let raw = le_bits(data, 8);
    let dv = f64::from_bits(raw as u64);
    let mut jd = a::j_dbl(dv);
    dump_node(&mut jd, "build.double");
    let mut jb = a::j_bool(!data.is_empty() && (data[0] & 1) != 0);
    dump_node(&mut jb, "build.bool");
    let mut jn = a::j_null();
    dump_node(&mut jn, "build.null");

    // array of one element per byte (first 12 bytes), by constructor and by add_element
    let n = if data.len() < 12 { data.len() } else { 12 };
    let mut arr: Arr = Vec::new();
    for i in 0..n {
        arr.push(make_elem(data[i]));
    }
    let mut ja = a::j_arr(arr.clone());
    dump_node(&mut ja, "build.array_lvalue");
    let arr_copy: Arr = arr.clone();
    let ja2 = a::j_arr(arr_copy);
    line("build.array_rvalue.to_string", &q(&a::to_string(&ja2)));
    let mut jb2 = a::j_arr(Vec::new());
    for i in 0..n {
        a::add_element(&mut jb2, make_elem(data[i])).expect("array");
    }
    dump_node(&mut jb2, "build.array_add_element");
    eq_line("build.array_eq", &ja, &jb2);

    // object: key = one byte (so NUL and high bytes occur), value = element of the next byte; duplicates overwrite
    let mut obj: Obj = Obj::new();
    let mut i = 0;
    while i + 1 < n {
        obj.insert(vec![data[i]], make_elem(data[i + 1]));
        i += 1;
    }
    let mut jo = a::j_obj(obj.clone());
    dump_node(&mut jo, "build.object_lvalue");
    let mut jo2 = a::j_obj(Obj::new());
    let mut i = 0;
    while i + 1 < n {
        a::add_member(&mut jo2, &[data[i]], make_elem(data[i + 1])).expect("object");
        i += 1;
    }
    dump_node(&mut jo2, "build.object_add_member");
    eq_line("build.object_eq", &jo, &jo2);

    // nesting through references: operator[] then a mutator on the result
    let doc_members: Obj = Obj::new();
    let mut doc = a::j_obj(doc_members);
    a::add_member(&mut doc, b"arr", a::copy(&ja)).expect("object");
    a::add_member(&mut doc, b"obj", a::copy(&jo2)).expect("object");
    attempt("build.nested.arr_add", || {
        a::add_element(a::index_key(&mut doc, b"arr")?, a::j_str(data))?;
        Ok(q(&a::to_string(&doc)))
    });
    attempt("build.nested.obj_add", || {
        a::add_member(
            a::index_key(&mut doc, b"obj")?,
            data,
            a::j_int(data.len() as i32),
        )?;
        Ok(q(&a::to_string(&doc)))
    });
    attempt("build.nested.index", || {
        let slot = a::index(a::index_key(&mut doc, b"arr")?, 0)?;
        a::assign(slot, &a::j_str(b"replaced"));
        Ok(q(&a::to_string(&doc)))
    });
    dump_node(&mut doc, "build.nested");

    // get_object / get_array return copies (Q19)
    attempt("build.copy_object", || {
        let mut m: Obj = a::get_object(&jo)?;
        m.insert(b"x".to_vec(), a::j_int(1));
        Ok(format!(
            "copy_size={} orig_size={}",
            num(m.len() as i64),
            num(a::size(&jo)? as i64)
        ))
    });
    attempt("build.copy_array", || {
        let mut v: Arr = a::get_array(&ja)?;
        v.push(a::j_int(1));
        Ok(format!(
            "copy_size={} orig_size={}",
            num(v.len() as i64),
            num(a::size(&ja)? as i64)
        ))
    });

    // type-error paths on the built values
    attempt("build.add_member_on_array", || {
        a::add_member(&mut ja, b"k", a::j_int(1))?;
        Ok("ok".to_string())
    });
    attempt("build.add_element_on_object", || {
        a::add_element(&mut jo, a::j_int(1))?;
        Ok("ok".to_string())
    });
    attempt("build.size_on_string", || Ok(num(a::size(&js)? as i64)));
}

// ---------------------------------------------------------------------------------------------------------------
// section matrix: fixed values, every ordered pair (independent of the input)
// ---------------------------------------------------------------------------------------------------------------

fn section_matrix() {
    let mut v: Vec<J> = Vec::new();
    let mut names: Vec<&str> = Vec::new();
    v.push(a::j_null());
    names.push("null");
    v.push(a::j_bool(true));
    names.push("true");
    v.push(a::j_bool(false));
    names.push("false");
    v.push(a::j_int(0));
    names.push("int0");
    v.push(a::j_int(1));
    names.push("int1");
    v.push(a::j_int(-1));
    names.push("int-1");
    v.push(a::j_dbl(0.0));
    names.push("dbl0");
    v.push(a::j_dbl(-0.0));
    names.push("dbl-0");
    v.push(a::j_dbl(1.5));
    names.push("dbl1.5");
    v.push(a::j_dbl(f64::NAN));
    names.push("nan");
    v.push(a::j_str(b""));
    names.push("str-empty");
    v.push(a::j_str(b"a"));
    names.push("str-a");
    v.push(a::j_str(b"b"));
    names.push("str-b");
    v.push(a::j_arr(Vec::new()));
    names.push("arr-empty");
    v.push(a::j_arr(vec![a::j_int(1)]));
    names.push("arr-1");
    v.push(a::j_arr(vec![a::j_dbl(1.0)]));
    names.push("arr-1.0");
    v.push(a::j_arr(vec![a::j_str(b"a"), a::j_null()]));
    names.push("arr-a-null");
    let mut o1: Obj = Obj::new();
    o1.insert(b"a".to_vec(), a::j_int(1));
    v.push(a::j_obj(Obj::new()));
    names.push("obj-empty");
    v.push(a::j_obj(o1));
    names.push("obj-a1");
    for i in 0..v.len() {
        for j in 0..v.len() {
            let mut label = String::from("matrix.");
            label.push_str(names[i]);
            label.push('.');
            label.push_str(names[j]);
            eq_line(&label, &v[i], &v[j]);
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: dump <input-file>");
        std::process::exit(2);
    }
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        flush_out();
        default_hook(info);
    }));
    let path = &args[args.len() - 1];
    let data: Vec<u8> = match std::fs::read(path) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("cannot open {}", path);
            std::process::exit(2);
        }
    };

    line("input.size", &num(data.len() as i64));

    // parse (the crate reads the buffer up to the first NUL, Q10)
    let mut parsed = a::j_null();
    let mut ok = false;
    attempt("parse", || {
        parsed = a::parse(&data)?;
        ok = true;
        Ok(kind(&parsed))
    });

    if ok {
        dump_node(&mut parsed, "$");

        // roundtrip
        let ser = a::to_string(&parsed);
        line("roundtrip.ser", &q(&ser));
        let mut again = a::j_null();
        let mut ok2 = false;
        attempt("roundtrip.parse", || {
            again = a::parse(&ser)?;
            ok2 = true;
            Ok(kind(&again))
        });
        if ok2 {
            line("roundtrip.ser2", &q(&a::to_string(&again)));
            eq_line("roundtrip.eq", &parsed, &again);
        }

        // copy constructor and assignment
        let copy = a::copy(&parsed);
        eq_line("copy.eq", &parsed, &copy);
        line("copy.ser", &q(&a::to_string(&copy)));
        let mut assigned = a::j_int(5);
        a::assign(&mut assigned, &parsed);
        eq_line("assign.eq", &parsed, &assigned);
        line("assign.ser", &q(&a::to_string(&assigned)));
        let mut chain = a::j_null();
        a::assign(&mut assigned, &copy);
        a::assign(&mut chain, &assigned);
        eq_line("chain.eq", &parsed, &chain);
        let mut nulled = a::copy(&parsed);
        a::assign(&mut nulled, &a::j_null());
        line(
            "assign_null",
            &format!("{} {}", kind(&nulled), q(&a::to_string(&nulled))),
        );
        eq_line("assign_null.eq", &parsed, &nulled);
    }

    section_streams(&data);
    section_build(&data);
    section_matrix();
    flush_out();
}
