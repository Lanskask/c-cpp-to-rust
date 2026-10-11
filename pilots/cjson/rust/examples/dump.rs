//! Differential driver for cJSON + cJSON_Utils, Rust side (judge.md section 5.2).
//!
//! A line-for-line port of `migration/harness/dump.c`: same modes, same output bytes. It reaches the crate
//! only through the Judge adapter, so Stage 2 changes the adapter and not this file. Everything it prints is
//! compared byte for byte with the C driver on the whole corpus (`run.py --stage 2`).
//!
//! Input file = last argument. One JSON text "A", or "A" + "\n===\n" + "B". Modes: M1 parse, M2 print,
//! M3 lookups and mutations, M4 allocator hooks (RULEBOOK D1, revised: every adapter has them),
//! M5 cJSON_Utils, M6 locale (LC_NUMERIC for D3, LC_CTYPE for D19), M7 fixed constructors, writable views
//! (D21) and field writes (Q3).
//!
//! The only `unsafe` here is the C library's formatting and locale functions (`snprintf` for `%.17g`,
//! `setlocale`, `localeconv`, `tolower`), which are part of the observable output, not of the crate.
#![allow(clippy::needless_range_loop)]

#[path = "../tests/adapter/mod.rs"]
mod adapter;

use adapter::{ErrPos, Item, Parsed, NULL};
use std::cell::RefCell;
use std::ffi::{c_char, c_int, CStr};
use std::io::Write;

const MAXFAIL: i64 = 48;
/// `cJSON_Compare` on objects is exponential in the depth (RULEBOOK Q25): only called below this depth.
const COMPARE_MAX_DEPTH: usize = 14;

// ---- C library pieces that are part of the observable output ------------------------------------------------

unsafe extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    fn setlocale(category: c_int, locale: *const c_char) -> *const c_char;
    fn localeconv() -> *const Lconv;
    fn tolower(c: c_int) -> c_int;
}

#[repr(C)]
struct Lconv {
    decimal_point: *const c_char,
}

#[cfg(target_os = "macos")]
const LC_CTYPE: c_int = 2;
#[cfg(target_os = "macos")]
const LC_NUMERIC: c_int = 4;
#[cfg(not(target_os = "macos"))]
const LC_CTYPE: c_int = 0;
#[cfg(not(target_os = "macos"))]
const LC_NUMERIC: c_int = 1;

/// `printf("%.17g", v)`.
fn g17(v: f64) -> String {
    let mut buf = [0 as c_char; 64];
    // SAFETY: the buffer is large enough for any `%.17g` result (at most 24 bytes); format and argument match.
    let n = unsafe { snprintf(buf.as_mut_ptr(), buf.len(), c"%.17g".as_ptr(), v) };
    assert!(n > 0 && (n as usize) < buf.len());
    // SAFETY: snprintf NUL-terminated the buffer.
    unsafe { CStr::from_ptr(buf.as_ptr()) }.to_string_lossy().into_owned()
}

/// `setlocale(category, name)`; true when the OS accepted it.
fn set_locale(category: c_int, name: &CStr) -> bool {
    // SAFETY: `name` is a valid C string; setlocale copies it.
    !unsafe { setlocale(category, name.as_ptr()) }.is_null()
}

/// Set by the Stage 2 Judge (`run.py`, adapter `C_LAYOUT=false`, RULEBOOK D3/D19): the crate always uses the C
/// locale, so the locale probes run in it; the locale lines are still printed from the real locale first.
fn probe_in_c_locale() -> bool {
    std::env::var_os("DUMP_PROBE_IN_C_LOCALE").is_some()
}

fn decimal_point() -> String {
    // SAFETY: localeconv returns a pointer to static storage whose first member is a C string.
    unsafe { CStr::from_ptr((*localeconv()).decimal_point) }.to_string_lossy().into_owned()
}

fn c_tolower(c: i32) -> i32 {
    // SAFETY: plain libc call.
    unsafe { tolower(c) }
}

// ---- buffered output -----------------------------------------------------------------------------------------

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn out_bytes(b: &[u8]) {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        o.extend_from_slice(b);
        if o.len() > 1 << 16 {
            let _ = std::io::stdout().write_all(&o);
            o.clear();
        }
    });
}

fn out_flush() {
    OUT.with(|o| {
        let mut o = o.borrow_mut();
        let mut so = std::io::stdout().lock();
        let _ = so.write_all(&o);
        let _ = so.flush();
        o.clear();
    });
}

macro_rules! p {
    ($($arg:tt)*) => { out_bytes(format!($($arg)*).as_bytes()) };
}

// ---- printing helpers ----------------------------------------------------------------------------------------

/// `put_str`: NULL, or a quoted string with `"` and `\` escaped and bytes outside 0x20..0x7e as `\xNN`.
/// `s` is the string content (no NUL).
fn put_str(s: Option<&[u8]>) {
    let Some(s) = s else {
        out_bytes(b"NULL");
        return;
    };
    out_bytes(b"\"");
    for &c in s.iter().take_while(|&&c| c != 0) {
        if c == b'"' || c == b'\\' {
            out_bytes(&[b'\\', c]);
        } else if c < 0x20 || c >= 0x7f {
            p!("\\x{c:02x}");
        } else {
            out_bytes(&[c]);
        }
    }
    out_bytes(b"\"");
}

/// `put_str` of a C buffer: the bytes up to the first NUL.
fn put_cbuf(b: &[u8]) {
    let n = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    put_str(Some(&b[..n]));
}

fn put_line(label: &str, s: Option<&[u8]>) {
    p!("{label}=");
    put_str(s);
    out_bytes(b"\n");
}

fn put_err(err: ErrPos) {
    match err {
        ErrPos::Unset => p!("err=-1\n"),
        ErrPos::At(o) => p!("err={o}\n"),
        ErrPos::Outside => p!("err=outside\n"),
    }
}

fn dump_tree(mut n: Item, depth: usize) {
    while !n.is_null() {
        let c = adapter::child(n);
        let mut lastok = true;
        if !c.is_null() {
            let mut last = c;
            while !adapter::next(last).is_null() {
                last = adapter::next(last);
            }
            lastok = adapter::prev(c) == last;
        }
        p!(
            "{:w$}type={} vi={} vd={} key=",
            "",
            adapter::type_flags(n),
            adapter::value_int(n),
            g17(adapter::value_double(n)),
            w = depth * 2 + 2
        );
        put_str(adapter::key(n).as_deref());
        p!(" vs=");
        put_str(adapter::value_string(n).as_deref());
        p!(" first_prev_is_last={} prev_null={}\n", lastok as i32, adapter::prev(n).is_null() as i32);
        if !c.is_null() {
            dump_tree(c, depth + 1);
        }
        n = adapter::next(n);
    }
}

fn show(label: &str, item: Item) {
    let s = adapter::print_unformatted(item);
    put_line(label, s.as_ref().map(|s| s.bytes()));
}

fn show_fmt(label: &str, item: Item) {
    let s = adapter::print(item);
    put_line(label, s.as_ref().map(|s| s.bytes()));
}

fn tree_depth(mut n: Item) -> usize {
    let mut d = 0;
    while !n.is_null() {
        let c = tree_depth(adapter::child(n)) + 1;
        if c > d {
            d = c;
        }
        n = adapter::next(n);
    }
    d
}

fn yes_no(v: bool, yes: &str, no: &str) -> String {
    (if v { yes } else { no }).to_string()
}

// ---- M1/M2: parse and print ----------------------------------------------------------------------------------

fn print_modes(tag: &str, root: Item) {
    p!("-- print {tag}\n");
    show_fmt("formatted", root);
    show("unformatted", root);
    for fmt in 0..=1 {
        for pre in [0, 1, 256] {
            let s = adapter::print_buffered(root, pre, fmt != 0);
            put_line(&format!("buffered[pre={pre},fmt={fmt}]"), s.as_ref().map(|s| s.bytes()));
        }
    }
    let s = adapter::print_unformatted(root);
    if let Some(s) = s {
        let need = s.bytes().len() as i32 + 1;
        for extra in [0, 5, 64] {
            let len = need + extra;
            let mut b = vec![0u8; len as usize + 1];
            let ok = adapter::print_preallocated(root, Some(&mut b), len, false);
            p!("prealloc[len=need+{extra},fmt=0] ok={} ", ok as i32);
            put_cbuf(&b);
            out_bytes(b"\n");
        }
        for i in 0..2 {
            let mut len = need - 1 - i;
            if len < 0 {
                len = 0;
            }
            let mut b = vec![0u8; len as usize + 1];
            let ok = adapter::print_preallocated(root, Some(&mut b), len, true);
            p!("prealloc[len=need-{},fmt=1] ok={} ", 1 + i, ok as i32);
            put_cbuf(&b);
            out_bytes(b"\n");
        }
    }
}

fn end_off(e: Option<usize>) -> i64 {
    e.map_or(-1, |e| e as i64)
}

fn parse_modes(label: &str, text: &[u8], len: usize) {
    p!("== parse {label} len={len}\n");
    let r = adapter::parse(Some(text));
    p!("Parse root={} ", yes_no(!r.item.is_null(), "ok", "NULL"));
    put_err(r.err);
    if !r.item.is_null() {
        dump_tree(r.item, 0);
    }
    adapter::delete(r.item);

    for req in 0..=1 {
        let r = adapter::parse_with_opts(Some(text), req != 0, true);
        p!(
            "ParseWithOpts[req={req}] root={} end={} ",
            yes_no(!r.item.is_null(), "ok", "NULL"),
            end_off(r.end)
        );
        put_err(r.err);
        if !r.item.is_null() {
            show("  unformatted", r.item);
        }
        adapter::delete(r.item);
    }
    let lens = [len, len / 2, if len > 0 { len - 1 } else { 0 }, 0];
    for i in 0..4 {
        let r = adapter::parse_with_length_opts(Some(text), lens[i], i & 1 != 0, true);
        p!(
            "ParseWithLengthOpts[len={},req={}] root={} end={} ",
            lens[i],
            i & 1,
            yes_no(!r.item.is_null(), "ok", "NULL"),
            end_off(r.end)
        );
        put_err(r.err);
        if !r.item.is_null() {
            show("  unformatted", r.item);
        }
        adapter::delete(r.item);
    }
    let r = adapter::parse_with_length(Some(text), len);
    p!("ParseWithLength root={} ", yes_no(!r.item.is_null(), "ok", "NULL"));
    put_err(r.err);
    adapter::delete(r.item);
    let r = adapter::parse(None);
    p!("Parse(NULL) root={} ", yes_no(!r.item.is_null(), "ok", "NULL"));
    put_err(r.err);
    adapter::delete(r.item);
}

// ---- M3: lookups, mutations, minify --------------------------------------------------------------------------

/// Flip ASCII case and Latin-1 case (0xC0-0xDE <-> 0xE0-0xFE, not 0xD7/0xF7).
fn flip_case(s: &[u8]) -> Vec<u8> {
    s.iter()
        .map(|&c| match c {
            b'a'..=b'z' => c - 32,
            b'A'..=b'Z' => c + 32,
            0xE0..=0xFE if c != 0xF7 => c - 32,
            0xC0..=0xDE if c != 0xD7 => c + 32,
            _ => c,
        })
        .collect()
}

fn lookups(root: Item) {
    let n = adapter::array_size(root);
    p!(
        "-- lookups size={} isinvalid={} isfalse={} istrue={} isbool={} isnull={} isnumber={} isstring={} isarray={} isobject={} israw={}\n",
        n,
        adapter::is_invalid(root) as i32,
        adapter::is_false(root) as i32,
        adapter::is_true(root) as i32,
        adapter::is_bool(root) as i32,
        adapter::is_null(root) as i32,
        adapter::is_number(root) as i32,
        adapter::is_string(root) as i32,
        adapter::is_array(root) as i32,
        adapter::is_object(root) as i32,
        adapter::is_raw(root) as i32
    );
    put_line("stringvalue", adapter::get_string_value(root).as_deref());
    p!("numbervalue={}\n", g17(adapter::get_number_value(root)));
    let mut i = -1;
    while i <= n && i < 6 {
        let it = adapter::array_item(root, i);
        p!("item[{i}]={}", yes_no(!it.is_null(), "ok", "NULL"));
        if !it.is_null() {
            show(" ", it);
        } else {
            out_bytes(b"\n");
        }
        i += 1;
    }
    let mut c = adapter::child(root);
    while !c.is_null() && adapter::is_object(root) {
        let Some(k) = adapter::key(c) else {
            c = adapter::next(c);
            continue;
        };
        let flip = flip_case(&k);
        p!("key ");
        put_str(Some(&k));
        p!(
            ": ci={} cs={} has={} | flip ",
            yes_no(adapter::object_item(root, Some(&k)) == c, "self", "other"),
            yes_no(adapter::object_item_cs(root, Some(&k)) == c, "self", "other"),
            adapter::has_object_item(root, Some(&k)) as i32
        );
        put_str(Some(&flip));
        p!(
            ": ci={} cs={} has={}\n",
            yes_no(!adapter::object_item(root, Some(&flip)).is_null(), "found", "NULL"),
            yes_no(!adapter::object_item_cs(root, Some(&flip)).is_null(), "found", "NULL"),
            adapter::has_object_item(root, Some(&flip)) as i32
        );
        c = adapter::next(c);
    }
    p!(
        "get missing={} null_object={}\n",
        yes_no(!adapter::object_item(root, Some(b"no-such-key")).is_null(), "found", "NULL"),
        yes_no(!adapter::object_item(NULL, Some(b"a")).is_null(), "found", "NULL")
    );
}

fn mutations(src: Item) {
    let d = adapter::duplicate(src, true);
    p!("-- mutations\n");
    if d.is_null() {
        p!("duplicate NULL\n");
        return;
    }
    if adapter::is_array(d) {
        let x = adapter::detach_item_from_array(d, 0);
        p!("detach[0]={}\n", yes_no(!x.is_null(), "ok", "NULL"));
        if !x.is_null() {
            show("  detached", x);
        }
        show("  after-detach", d);
        let ins = if !x.is_null() { x } else { adapter::create_null() };
        p!("insert[0]={}\n", adapter::insert_item_in_array(d, 0, ins) as i32);
        p!("insert[99]={}\n", adapter::insert_item_in_array(d, 99, adapter::create_true()) as i32);
        p!("replace[1]={}\n", adapter::replace_item_in_array(d, 1, adapter::create_number(7.0)) as i32);
        p!("replace[-1]={}\n", adapter::replace_item_in_array(d, -1, adapter::create_null()) as i32);
        p!(
            "addref={}\n",
            adapter::add_item_reference_to_array(d, adapter::create_string_reference(Some(c"ref"))) as i32
        );
        show("  after-edits", d);
        adapter::delete_item_from_array(d, adapter::array_size(d) - 1);
        adapter::delete_item_from_array(d, 1000);
        show("  after-delete", d);
        let x = adapter::detach_item_via_pointer(d, adapter::child(d));
        p!("detach-via-pointer={}\n", yes_no(!x.is_null(), "ok", "NULL"));
        adapter::delete(x);
        show("  final", d);
    } else if adapter::is_object(d) {
        let first = adapter::child(d);
        let k = if first.is_null() { None } else { adapter::key(first) };
        if let Some(kc) = k {
            let fl = flip_case(&kc);
            let x = adapter::detach_item_from_object(d, Some(&fl));
            p!("detach-ci[flip]={}\n", yes_no(!x.is_null(), "ok", "NULL"));
            if !x.is_null() {
                p!("re-add={}\n", adapter::add_item_to_object(d, Some(&kc), x) as i32);
            }
            p!(
                "replace-ci={}\n",
                adapter::replace_item_in_object(d, Some(&fl), adapter::create_string(Some(b"rep"))) as i32
            );
            p!(
                "replace-cs={}\n",
                adapter::replace_item_in_object_cs(d, Some(b"no-such-key"), adapter::create_string(Some(b"rep2"))) as i32
            );
            show("  after-replace", d);
            p!(
                "addCS={}\n",
                adapter::add_item_to_object_cs(d, Some(c"const-key"), adapter::create_number(1.25)) as i32
            );
            p!(
                "addref={}\n",
                adapter::add_item_reference_to_object(d, Some(b"ref-key"), adapter::child(d)) as i32
            );
            adapter::delete_item_from_object_cs(d, Some(b"const-key"));
            adapter::delete_item_from_object(d, Some(b"ref-key")); // the reference first: it aliases d->child's data
            adapter::delete_item_from_object(d, Some(&fl));
            show("  after-delete", d);
        } else {
            p!("object without keys\n");
        }
        let ok = |i: Item| yes_no(!i.is_null(), "ok", "NULL");
        p!("AddStringToObject={}\n", ok(adapter::add_string_to_object(d, Some(b"added"), Some(b"v"))));
        p!("AddNumberToObject={}\n", ok(adapter::add_number_to_object(d, Some(b"n"), 2.5)));
        p!("AddNullToObject={}\n", ok(adapter::add_null_to_object(d, Some(b"z"))));
        p!("AddRawToObject={}\n", ok(adapter::add_raw_to_object(d, Some(b"raw"), Some(b"[1,2]"))));
        p!("AddObjectToObject={}\n", ok(adapter::add_object_to_object(d, Some(b"o"))));
        p!("AddArrayToObject={}\n", ok(adapter::add_array_to_object(d, Some(b"a"))));
        show("  final", d);
    } else {
        p!("setters ");
        p!("SetNumberHelper={} ", g17(adapter::set_number_helper(d, 3.75)));
        put_line("SetValuestring", adapter::set_valuestring(d, Some(b"replaced")).as_deref());
        show("  final", d);
    }
    adapter::delete(d);
}

fn minify_pass(text: &[u8]) {
    let m = adapter::minify(Some(text)).expect("Some in, Some out");
    put_line("minified", Some(&m));
    let r = adapter::parse(Some(&m)).item;
    p!("minified-parses={}\n", yes_no(!r.is_null(), "ok", "NULL"));
    if !r.is_null() {
        show("minified-reprint", r);
    }
    adapter::delete(r);
}

// ---- M4: hooks -----------------------------------------------------------------------------------------------

/// `live_now`: allocations still alive after a fail-the-Nth run (the failed allocation is not one); 0 = no leak.
fn live_now() -> i64 {
    let st = adapter::hook_stats();
    (st.allocs - if st.fail_at != 0 && st.fail_at <= st.allocs { 1 } else { 0 }) - st.frees
}

fn hooks_pass(text: &[u8], len: usize) {
    p!("== hooks\n");
    adapter::hooks_on(0);
    let root = adapter::parse_with_length(Some(text), len).item;
    let st = adapter::hook_stats();
    p!(
        "count parse allocs={} frees={} bytes={} root={}\n",
        st.allocs,
        st.frees,
        st.bytes,
        yes_no(!root.is_null(), "ok", "NULL")
    );
    let mut total = st.allocs;
    if !root.is_null() {
        let (a0, f0, b0) = (st.allocs, st.frees, st.bytes);
        let s = adapter::print(root);
        let st = adapter::hook_stats();
        p!("count print allocs={} frees={} bytes={}\n", st.allocs - a0, st.frees - f0, st.bytes - b0);
        drop(s);
        let st = adapter::hook_stats();
        let (a0, f0, b0) = (st.allocs, st.frees, st.bytes);
        let s = adapter::print_unformatted(root);
        let st = adapter::hook_stats();
        p!("count print-unformatted allocs={} frees={} bytes={}\n", st.allocs - a0, st.frees - f0, st.bytes - b0);
        drop(s);
        let (a0, b0) = (adapter::hook_stats().allocs, adapter::hook_stats().bytes);
        {
            let d = adapter::duplicate(root, true);
            let st = adapter::hook_stats();
            p!("count duplicate allocs={} bytes={}\n", st.allocs - a0, st.bytes - b0);
            adapter::delete(d);
        }
    }
    adapter::delete(root);
    let st = adapter::hook_stats();
    p!("count after delete: allocs={} frees={}\n", st.allocs, st.frees);
    adapter::hooks_off();

    let limit = total.min(MAXFAIL);
    for i in 1..=limit {
        adapter::hooks_on(i);
        let root = adapter::parse_with_length(Some(text), len).item;
        adapter::delete(root);
        let st = adapter::hook_stats();
        p!(
            "fail-parse[{i}] root={} live={}\n",
            yes_no(!root.is_null(), "ok", "NULL"),
            (st.allocs - if st.fail_at <= st.allocs { 1 } else { 0 }) - st.frees
        );
        adapter::hooks_off();
    }
    let root = adapter::parse_with_length(Some(text), len).item;
    if root.is_null() {
        return;
    }
    adapter::hooks_on(0);
    let s = adapter::print(root);
    total = adapter::hook_stats().allocs;
    drop(s);
    adapter::hooks_off();
    let limit = total.min(MAXFAIL);
    for i in 1..=limit {
        adapter::hooks_on(i);
        let s = adapter::print(root);
        let ok = s.is_some();
        drop(s);
        p!("fail-print[{i}] out={} live={}\n", yes_no(ok, "ok", "NULL"), live_now());
        adapter::hooks_off();
        adapter::hooks_on(i);
        let s = adapter::print_buffered(root, 8, false);
        let ok = s.is_some();
        drop(s);
        p!("fail-buffered[{i}] out={} live={}\n", yes_no(ok, "ok", "NULL"), live_now());
        adapter::hooks_off();
    }
    adapter::hooks_on(0);
    {
        let d = adapter::duplicate(root, true);
        total = adapter::hook_stats().allocs;
        adapter::delete(d);
    }
    adapter::hooks_off();
    let limit = total.min(MAXFAIL);
    for i in 1..=limit {
        adapter::hooks_on(i);
        let d = adapter::duplicate(root, true);
        adapter::delete(d);
        p!("fail-duplicate[{i}] out={} live={}\n", yes_no(!d.is_null(), "ok", "NULL"), live_now());
        adapter::hooks_off();
    }
    adapter::delete(root);
}

// ---- M5: cJSON_Utils -----------------------------------------------------------------------------------------

fn rel(found: Item, c: Item) -> &'static str {
    if found == c {
        "same"
    } else if !found.is_null() {
        "other"
    } else {
        "NULL"
    }
}

fn pointers(root: Item) {
    p!("-- pointers\n");
    let mut c = adapter::child(root);
    let mut n = 0;
    while !c.is_null() && n < 12 {
        let ptr = adapter::find_pointer_from_object_to(root, c);
        let back1 = ptr.as_deref().map_or(NULL, |q| adapter::get_pointer(root, Some(q)));
        let back2 = ptr.as_deref().map_or(NULL, |q| adapter::get_pointer_cs(root, Some(q)));
        put_line("  pointer-to-child", ptr.as_deref());
        p!("  roundtrip ci={} cs={}\n", rel(back1, c), rel(back2, c));
        c = adapter::next(c);
        n += 1;
    }
    let fixed: [&[u8]; 26] = [
        b"", b"/", b"/0", b"/1", b"/-", b"/00", b"/a", b"/A", b"/a/b", b"/a~1b", b"/a~0b", b"/~0", b"/~1", b"/~01",
        b"/x~1", b"/ab~1cd", b"/\xC3\xA9", b"/\xC9", b"/\xE9", b"a", b"/key", b"/0/0", b"/1/a", b"//",
        b"/18446744073709551616", b"/0/18446744073709551617",
    ];
    for f in fixed {
        let g = adapter::get_pointer(root, Some(f));
        let gc = adapter::get_pointer_cs(root, Some(f));
        p!("  GetPointer ");
        put_str(Some(f));
        p!(" ci={} cs={}", yes_no(!g.is_null(), "ok", "NULL"), yes_no(!gc.is_null(), "ok", "NULL"));
        if !g.is_null() {
            show(" ", g);
        } else {
            out_bytes(b"\n");
        }
    }
    // object keys as encoded pointers
    let mut c = adapter::child(root);
    while !c.is_null() && adapter::is_object(root) {
        let Some(k) = adapter::key(c) else { break };
        let mut enc: Vec<u8> = vec![b'/'];
        for &ch in k.iter().take_while(|&&ch| ch != 0) {
            if enc.len() >= 512 - 3 {
                break;
            }
            match ch {
                b'~' => enc.extend_from_slice(b"~0"),
                b'/' => enc.extend_from_slice(b"~1"),
                _ => enc.push(ch),
            }
        }
        let tgt = adapter::get_pointer_cs(root, Some(&enc));
        p!("  key-pointer ");
        put_str(Some(&enc));
        p!(" -> {}\n", rel(tgt, c));
        c = adapter::next(c);
    }
}

fn pair_ops(ta: &[u8], tb: &[u8]) {
    let a = adapter::parse(Some(ta)).item;
    let b = adapter::parse(Some(tb)).item;
    p!("== pair a={} b={}\n", yes_no(!a.is_null(), "ok", "NULL"), yes_no(!b.is_null(), "ok", "NULL"));
    if !a.is_null() && !b.is_null() {
        if tree_depth(a) <= COMPARE_MAX_DEPTH && tree_depth(b) <= COMPARE_MAX_DEPTH {
            p!(
                "Compare ci={} cs={} self={}\n",
                adapter::compare(a, b, false) as i32,
                adapter::compare(a, b, true) as i32,
                adapter::compare(a, a, true) as i32
            );
        } else {
            p!("Compare skipped (depth > {COMPARE_MAX_DEPTH})\n");
        }
        for cs in 0..=1 {
            let cs = cs != 0;
            let patches = if cs { adapter::generate_patches_cs(a, b) } else { adapter::generate_patches(a, b) };
            p!("-- GeneratePatches[cs={}]\n", cs as i32);
            show("  a-after", a); // Q18: sorting in place
            show("  b-after", b);
            if !patches.is_null() {
                show("  patches", patches);
                let target = adapter::duplicate(a, true);
                let rc = if cs { adapter::apply_patches_cs(target, patches) } else { adapter::apply_patches(target, patches) };
                p!("  apply-own rc={rc}\n");
                show("  apply-own-result", target);
                adapter::delete(target);
            } else {
                p!("  patches NULL\n");
            }
            adapter::delete(patches);

            let target = adapter::duplicate(a, true);
            let rc = if cs { adapter::apply_patches_cs(target, b) } else { adapter::apply_patches(target, b) };
            p!("-- ApplyPatches(a, b as patches)[cs={}] rc={rc}\n", cs as i32);
            show("  result", target);
            adapter::delete(target);

            let target = adapter::duplicate(a, true);
            let target = if cs { adapter::merge_patch_cs(target, b) } else { adapter::merge_patch(target, b) };
            p!("-- MergePatch(a, b)[cs={}]\n", cs as i32);
            show("  result", target);
            adapter::delete(target);

            {
                let mp = if cs { adapter::generate_merge_patch_cs(a, b) } else { adapter::generate_merge_patch(a, b) };
                p!("-- GenerateMergePatch[cs={}]\n", cs as i32);
                if !mp.is_null() {
                    let t2 = adapter::duplicate(a, true);
                    show("  patch", mp);
                    let t2 = if cs { adapter::merge_patch_cs(t2, mp) } else { adapter::merge_patch(t2, mp) };
                    show("  applied", t2);
                    adapter::delete(t2);
                } else {
                    p!("  NULL\n");
                }
                adapter::delete(mp);
            }
        }
        {
            let s = adapter::duplicate(a, true);
            adapter::sort_object(s);
            show("sorted-ci", s);
            if UTILS_SUBSET.load(std::sync::atomic::Ordering::SeqCst) {
                dump_tree(s, 0);
            }
            adapter::delete(s);
            let s = adapter::duplicate(a, true);
            adapter::sort_object_cs(s);
            show("sorted-cs", s);
            if UTILS_SUBSET.load(std::sync::atomic::Ordering::SeqCst) {
                dump_tree(s, 0);
            }
            adapter::delete(s);
        }
        {
            let arr = adapter::create_array();
            adapter::add_patch_to_array(arr, Some(b"add"), Some(b"/x"), b);
            adapter::add_patch_to_array(arr, Some(b"remove"), Some(b"/y"), NULL);
            show("AddPatchToArray", arr);
            adapter::delete(arr);
        }
        pointers(a);
    }
    adapter::delete(a);
    adapter::delete(b);
}

// ---- M6: locales ---------------------------------------------------------------------------------------------

fn locale_pass(ta: &[u8]) {
    let comma: [&CStr; 5] = [c"de_DE.UTF-8", c"fr_FR.UTF-8", c"ru_RU.UTF-8", c"nl_NL.UTF-8", c"de_DE.ISO8859-1"];
    let latin1: [&CStr; 3] = [c"de_DE.ISO8859-1", c"fr_FR.ISO8859-1", c"en_US.ISO8859-15"];
    p!("== locale\n");
    let mut used: Option<&CStr> = None;
    for l in comma {
        if used.is_some() {
            break;
        }
        if set_locale(LC_NUMERIC, l) && decimal_point().as_bytes().first() == Some(&b',') {
            used = Some(l);
        }
    }
    match used {
        None => {
            set_locale(LC_NUMERIC, c"C");
            p!("numeric-locale none (no comma locale available)\n");
        }
        Some(u) => {
            let texts: [&[u8]; 5] = [b"1.5", b"[1.5,2.25,-0.5e2]", b"{\"a\":1.5}", b"1,5", ta];
            p!("numeric-locale {} decimal_point={}\n", u.to_string_lossy(), decimal_point());
            if probe_in_c_locale() {
                set_locale(LC_NUMERIC, c"C");
            }
            for (i, t) in texts.iter().enumerate() {
                let r: Parsed = adapter::parse(Some(t));
                p!("parse[{i}] root={} ", yes_no(!r.item.is_null(), "ok", "NULL"));
                put_err(r.err);
                if !r.item.is_null() {
                    show("  print", r.item);
                    dump_tree(r.item, 1);
                }
                adapter::delete(r.item);
            }
            for (label, v) in [("create(1.5)", 1.5), ("create(0.1)", 0.1), ("create(123456.789012345678)", 123456.789012345678)] {
                let n = adapter::create_number(v);
                show(label, n);
                adapter::delete(n);
            }
            set_locale(LC_NUMERIC, c"C");
        }
    }

    let mut used: Option<&CStr> = None;
    for l in latin1 {
        if used.is_some() {
            break;
        }
        if set_locale(LC_CTYPE, l) {
            used = Some(l);
        }
    }
    match used {
        None => {
            set_locale(LC_CTYPE, c"C");
            p!("ctype-locale none (no Latin-1 locale available)\n");
        }
        Some(u) => {
            let objs: [&[u8]; 3] = [b"{\"\xC9t\xE9\":1,\"b\":2,\"\xE9t\xC9\":3,\"B\":4}", b"{\"\xC9\":1}", b"{\"\xE9\":1}"];
            p!(
                "ctype-locale {} tolower(0xC9)={} tolower(0xDF)={} tolower(0xE9)={}\n",
                u.to_string_lossy(),
                c_tolower(0xC9),
                c_tolower(0xDF),
                c_tolower(0xE9)
            );
            if probe_in_c_locale() {
                set_locale(LC_CTYPE, c"C");
            }
            let o = adapter::parse(Some(objs[0])).item;
            let keys: [&[u8]; 7] = [b"\xC9t\xE9", b"\xE9t\xE9", b"\xE9t\xC9", b"\xC9T\xC9", b"b", b"B", b"\xE9"];
            for k in keys {
                let f = adapter::object_item(o, Some(k));
                let g = adapter::object_item_cs(o, Some(k));
                p!("lookup ");
                put_str(Some(k));
                let fs = if f.is_null() {
                    "NULL"
                } else if adapter::value_int(f) == 0 {
                    "found0"
                } else {
                    "found"
                };
                p!(" ci={} cs={}\n", fs, yes_no(!g.is_null(), "found", "NULL"));
                if !f.is_null() {
                    p!("  ci-value={}\n", adapter::value_int(f));
                }
            }
            for k in keys {
                let mut ptr = vec![b'/'];
                ptr.extend_from_slice(k);
                let q = adapter::get_pointer(o, Some(&ptr));
                p!("pointer ");
                put_str(Some(&ptr));
                p!(" -> {}\n", yes_no(!q.is_null(), "found", "NULL"));
            }
            let a = adapter::parse(Some(objs[1])).item;
            let b = adapter::parse(Some(objs[2])).item;
            p!("compare ci={} cs={}\n", adapter::compare(a, b, false) as i32, adapter::compare(a, b, true) as i32);
            {
                let s = adapter::parse(Some(b"{\"\xE9\":1,\"\xC9\":2,\"z\":3,\"A\":4,\"\xC0\":5,\"\xE0\":6}")).item;
                adapter::sort_object(s);
                show("sorted-ci", s);
                adapter::delete(s);
            }
            {
                let d = adapter::parse(Some(ta)).item;
                if !d.is_null() && adapter::is_object(d) && !adapter::child(d).is_null() {
                    if let Some(k) = adapter::key(adapter::child(d)) {
                        let fl = flip_case(&k);
                        p!("input-key lookup ");
                        put_str(Some(&fl));
                        p!(" ci={}\n", yes_no(!adapter::object_item(d, Some(&fl)).is_null(), "found", "NULL"));
                    }
                }
                adapter::delete(d);
            }
            adapter::delete(o);
            adapter::delete(a);
            adapter::delete(b);
            set_locale(LC_CTYPE, c"C");
        }
    }
}

// ---- M7: fixed constructors ----------------------------------------------------------------------------------

fn constructors() {
    let nums: [f64; 25] = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        1.5,
        0.1,
        1e10,
        1e15,
        1e16,
        1e300,
        -1e300,
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967296.0,
        3.141592653589793,
        1.0 / 3.0,
        1e-5,
        1e-7,
        123456789012345678.0,
        9007199254740993.0,
        5e-324,
        1.7976931348623157e308,
    ];
    let nan_v = f64::NAN;
    let inf_v = f64::INFINITY;
    p!("== constructors\n");
    for (i, v) in nums.iter().enumerate() {
        let n = adapter::create_number(*v);
        p!("number[{i}] vi={} vd={} ", adapter::value_int(n), g17(adapter::value_double(n)));
        show("print", n);
        adapter::delete(n);
    }
    // D6: (int)NaN is UB in C; both sides are expected to print 0 on this platform (measured, see RUN-NOTES)
    let n = adapter::create_number(nan_v);
    p!("NaN vi={} ", adapter::value_int(n));
    show("print", n);
    p!("NaN getnumber isnan={}\n", adapter::get_number_value(n).is_nan() as i32);
    let r = adapter::set_number_helper(n, nan_v);
    p!("SetNumberHelper(NaN)={} vi={}\n", g17(r), adapter::value_int(n));
    adapter::delete(n);
    let n = adapter::create_number(inf_v);
    p!("Inf vi={} ", adapter::value_int(n));
    show("print", n);
    adapter::delete(n);
    let n = adapter::create_number(-inf_v);
    p!("-Inf vi={} ", adapter::value_int(n));
    show("print", n);
    adapter::delete(n);
    p!("GetNumberValue(NULL) isnan={}\n", adapter::get_number_value(NULL).is_nan() as i32);

    let arr = adapter::create_array();
    adapter::add_item_to_array(arr, adapter::create_string(Some(b"a\"b\\c\n\t\x01\x1f/")));
    adapter::add_item_to_array(arr, adapter::create_string(Some(b"\xc3\xa9\xff")));
    adapter::add_item_to_array(arr, adapter::create_string(Some(b"")));
    adapter::add_item_to_array(arr, adapter::create_true());
    adapter::add_item_to_array(arr, adapter::create_false());
    adapter::add_item_to_array(arr, adapter::create_bool(true));
    adapter::add_item_to_array(arr, adapter::create_null());
    adapter::add_item_to_array(arr, adapter::create_raw(Some(b"{raw:1}")));
    adapter::add_item_to_array(arr, adapter::create_string_reference(Some(c"ref")));
    {
        let ints = [1, -2, 2147483647];
        let floats = [0.5f32, 1.25f32];
        let doubles = [0.1, 1e100];
        let strs: [&[u8]; 2] = [b"x", b"y\"z"];
        adapter::add_item_to_array(arr, adapter::create_int_array(Some(&ints), 3));
        adapter::add_item_to_array(arr, adapter::create_float_array(Some(&floats), 2));
        adapter::add_item_to_array(arr, adapter::create_double_array(Some(&doubles), 2));
        adapter::add_item_to_array(arr, adapter::create_string_array(Some(&strs), 2));
    }
    show("built", arr);
    show_fmt("built-fmt", arr);
    p!("version=");
    put_str(Some(&adapter::version()));
    out_bytes(b"\n");
    adapter::delete(arr);
    let n = adapter::create_null();
    p!("PrintBuffered(NULL item)={}\n", yes_no(adapter::print_buffered(NULL, 0, false).is_some(), "ok", "NULL"));
    p!("Print(NULL)={}\n", yes_no(adapter::print(NULL).is_some(), "ok", "NULL"));
    p!("AddItemToArray(NULL,NULL)={}\n", adapter::add_item_to_array(NULL, NULL) as i32);
    p!("InsertItemInArray(self)={}\n", adapter::insert_item_in_array(n, 0, n) as i32);
    adapter::delete(n);

    // D21: writes through the char * that GetStringValue, SetValuestring and the print functions return
    let n = adapter::create_string(Some(b"abcdef"));
    let first = adapter::value_string_addr(n);
    adapter::with_string_value_mut(n, |v| {
        v[0] = b'X';
        v[3] = 0;
    });
    put_line("view GetStringValue", adapter::get_string_value(n).as_deref());
    show("view print", n);
    adapter::set_valuestring(n, Some(b"xyz")); // 3 <= strlen 3: copied in place
    p!("view SetValuestring(xyz) same-buffer={}\n", (adapter::value_string_addr(n) == first) as i32);
    adapter::with_string_value_mut(n, |v| v[1] = b'Y');
    put_line("view after write", adapter::get_string_value(n).as_deref());
    adapter::set_valuestring(n, Some(b"long1")); // longer: new buffer, allocated before the old one is freed
    p!("view SetValuestring(long1) same-buffer={}\n", (adapter::value_string_addr(n) == first) as i32);
    if let Some(mut out) = adapter::print_unformatted(n) {
        out.bytes_mut()[1] = b'Q';
        out.bytes_mut()[4] = 0;
        put_line("view printed", Some(out.bytes()));
    }
    adapter::delete(n);
    // Q3: valueint and valuedouble are separate fields; print uses %d only when they agree
    let n = adapter::create_number(7.0);
    adapter::set_value_double(n, 7.5);
    show("field vd=7.5", n);
    adapter::set_value_int(n, 3);
    show("field vi=3", n);
    adapter::set_value_double(n, 3.0);
    show("field vd=3", n);
    adapter::set_value_int(n, 100);
    show("field vi=100", n);
    p!("field getnumber={} vi={}\n", g17(adapter::get_number_value(n)), adapter::value_int(n));
    adapter::delete(n);
}

// ---- main ----------------------------------------------------------------------------------------------------

fn run_document(label: &str, text: &[u8], len: usize) {
    parse_modes(label, text, len);
    let root = adapter::parse_with_length(Some(text), len).item;
    if !root.is_null() {
        p!("== document {label}\n");
        print_modes(label, root);
        lookups(root);
        mutations(root);
        {
            let d1 = adapter::duplicate(root, true);
            let d0 = adapter::duplicate(root, false);
            p!("-- duplicate\n");
            show("  deep", d1);
            show("  shallow", d0);
            if tree_depth(root) <= COMPARE_MAX_DEPTH {
                p!(
                    "  compare deep ci={} cs={} shallow ci={}\n",
                    adapter::compare(root, d1, false) as i32,
                    adapter::compare(root, d1, true) as i32,
                    adapter::compare(root, d0, false) as i32
                );
            } else {
                p!("  compare skipped (depth > {COMPARE_MAX_DEPTH})\n");
            }
            adapter::delete(d1);
            adapter::delete(d0);
        }
        adapter::delete(root);
    }
    minify_pass(text);
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---- Utils allocator section (patch-* and judge-* inputs) ----------------------------------------------------

static UTILS_SUBSET: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static UTILS_PROBE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn grandchild_or_child(x: Item) -> Item {
    let c = adapter::child(x);
    if c.is_null() {
        return x;
    }
    let g = adapter::child(c);
    if g.is_null() {
        c
    } else {
        g
    }
}

fn utils_once(op: usize, a: Item, b: Item, fail: i64, label: &str) -> i64 {
    let mut x = adapter::duplicate(a, true);
    let y = adapter::duplicate(b, true);
    let mut r = NULL;
    let mut ptr: Option<Vec<u8>> = None;
    let mut st = 0;
    adapter::hooks_on(fail);
    adapter::trace_start();
    match op {
        0 => st = adapter::apply_patches(x, y),
        1 => st = adapter::apply_patches_cs(x, y),
        2 => x = adapter::merge_patch(x, y),
        3 => r = adapter::generate_merge_patch(x, y),
        4 => r = adapter::generate_patches(x, y),
        _ => ptr = adapter::find_pointer_from_object_to(x, grandchild_or_child(x)),
    }
    let trace = adapter::trace_stop();
    let live = live_now();
    let total = adapter::hook_stats().allocs;
    p!("utils-{label}[{fail}] st={st} live={live} allocs={total} trace={trace}\n");
    adapter::hooks_off();
    if op == 5 {
        put_line("  pointer", ptr.as_deref());
    } else {
        show("  target", x);
        if op >= 3 {
            if r.is_null() {
                p!("  result=NULL\n");
            } else {
                show("  result", r);
            }
        }
    }
    adapter::delete(x);
    adapter::delete(y);
    adapter::delete(r);
    total
}

fn utils_hooks_pass(ta: &[u8], tb: &[u8]) {
    const NAMES: [&str; 6] = ["apply-ci", "apply-cs", "merge", "genmerge", "genpatches", "findptr"];
    let a = adapter::parse(Some(ta)).item;
    let b = adapter::parse(Some(tb)).item;
    p!("== utils-hooks\n");
    if !a.is_null() && !b.is_null() {
        for (op, name) in NAMES.iter().enumerate() {
            let total = utils_once(op, a, b, 0, name);
            if op >= 4 {
                continue;
            }
            for i in 1..=total.min(48) {
                utils_once(op, a, b, i, name);
            }
        }
        if UTILS_PROBE.load(std::sync::atomic::Ordering::SeqCst) {
            let e = adapter::parse(Some(b"[]")).item;
            let one = adapter::parse(Some(b"[1]")).item;
            let tgt = adapter::child(adapter::child(a));
            adapter::hooks_on(1);
            let pp = adapter::find_pointer_from_object_to(a, tgt);
            p!("probe findptr[fail 1] {} live={}\n", yes_no(pp.is_some(), "ok", "NULL"), live_now());
            adapter::hooks_off();
            adapter::hooks_on(0);
            adapter::trace_start();
            let g = adapter::generate_patches(e, one);
            let trace = adapter::trace_stop();
            p!("probe genpatches trace={trace}\n");
            adapter::hooks_off();
            adapter::delete(g);
            adapter::hooks_on(2);
            let g = adapter::generate_patches(e, one);
            p!("probe genpatches[fail 2] {} live={}\n", yes_no(!g.is_null(), "ok", "NULL"), live_now());
            adapter::hooks_off();
            if g.is_null() {
                p!("  result=NULL\n");
            } else {
                show("  result", g);
            }
            adapter::delete(g);
            adapter::delete(e);
            adapter::delete(one);
        }
    }
    adapter::delete(a);
    adapter::delete(b);
}

fn main() {
    // A Stage 1 debug build uses a few KiB of stack per nesting level; the C driver gets 8 MiB at -O0.
    std::thread::Builder::new().stack_size(512 << 20).spawn(real_main).unwrap().join().unwrap();
}

fn real_main() {
    let Some(path) = std::env::args_os().next_back().filter(|_| std::env::args_os().count() >= 2) else {
        std::process::exit(2);
    };
    let Ok(mut buf) = std::fs::read(&path) else {
        std::process::exit(2);
    };
    buf.truncate(1 << 20);
    let len = buf.len();

    let sep = if len >= 5 { find(&buf, b"\n===\n") } else { None };
    let (a, b): (Vec<u8>, Vec<u8>) = match sep {
        Some(s) => (buf[..s].to_vec(), buf[s + 5..].to_vec()),
        None => (buf.clone(), buf.clone()),
    };
    let (la, lb) = (a.len(), b.len());

    p!("input len={len} a={la} b={lb} separator={}\n", sep.is_some() as i32);
    run_document("A", &a, la);
    if sep.is_some() {
        run_document("B", &b, lb);
    }
    hooks_pass(&a, la);
    {
        let base: Vec<u8> = std::path::Path::new(&path)
            .file_name()
            .map(|n| n.as_encoded_bytes().to_vec())
            .unwrap_or_default();
        let subset = base.starts_with(b"patch-") || base.starts_with(b"judge-");
        UTILS_SUBSET.store(subset, std::sync::atomic::Ordering::SeqCst);
        UTILS_PROBE.store(base.starts_with(b"judge-utils-oom"), std::sync::atomic::Ordering::SeqCst);
        if subset {
            utils_hooks_pass(&a[..la], &b[..lb]);
        }
    }
    pair_ops(&a, &b);
    locale_pass(&a);
    constructors();
    out_flush();
}
