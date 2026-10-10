"""Idiom rows I*: integer and float semantics."""

from __future__ import annotations

from conftest import Lab

WRAPV = ("-std=c11", "-fwrapv")


def _same_flags(lab: Lab, c_source: str, rust_source: str, flags: tuple[str, ...]) -> str:
    c, r = lab.c(c_source, flags), lab.rust(rust_source)
    assert c.code == 0, f"C exited {c.code}: {c.err}"
    assert r.code == 0, f"Rust exited {r.code}: {r.err}"
    assert c.out == r.out, f"C printed {c.out!r}, Rust printed {r.out!r}"
    return c.out


def test_i1_unsigned_wraps_with_wrapping_ops(lab: Lab) -> None:
    """I1: unsigned arithmetic wraps (defined); trap: plain `-`/`*` panic in a Rust debug build."""
    c = r"""
#include <stdio.h>
#include <stdint.h>
int main(void){
  volatile unsigned a = 3, b = 5;
  volatile uint32_t m = 0xFFFFFFFFu;
  volatile uint64_t w = 0xFFFFFFFFFFFFFFFFull;
  printf("%u %u %llu\n", a - b, m * m, (unsigned long long)(w * 3));
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let (a, b): (u32, u32) = (black_box(3), black_box(5));
    let m: u32 = black_box(u32::MAX);
    let w: u64 = black_box(u64::MAX);
    println!("{} {} {}", a.wrapping_sub(b), m.wrapping_mul(m), w.wrapping_mul(3));
}
"""
    naive = ok.replace("a.wrapping_sub(b)", "a - b")
    assert lab.same(c, ok) == "4294967294 1 18446744073709551613\n"
    lab.differs(c, naive)


def test_i2_signed_overflow_wrapping_and_checked(lab: Lab) -> None:
    """I2: signed overflow is UB; Stage 1 `wrapping_*` (compared under -fwrapv), Stage 2 `checked_*`."""
    c = r"""
#include <stdio.h>
#include <limits.h>
int main(void){
  volatile int m = INT_MAX, n = INT_MIN;
  int r;
  int ovf = __builtin_add_overflow(m, 1, &r);
  printf("%d %d %d %d\n", m + 1, n - 1, m * 2, ovf);
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let m: i32 = black_box(i32::MAX);
    let n: i32 = black_box(i32::MIN);
    let ovf = m.checked_add(1).is_none() as i32;
    println!("{} {} {} {}", m.wrapping_add(1), n.wrapping_sub(1), m.wrapping_mul(2), ovf);
}
"""
    naive = ok.replace("m.wrapping_add(1)", "m + 1")
    assert _same_flags(lab, c, ok, WRAPV) == "-2147483648 2147483647 -2 1\n"
    lab.differs(c, naive)


def test_i3_shift_by_width_masks_count(lab: Lab) -> None:
    """I3: shift by >= width is UB; `wrapping_shl` masks the count like arm64 hardware; plain `<<` panics."""
    c = r"""
#include <stdio.h>
int main(void){
  volatile unsigned x = 1, s = 33;
  volatile int neg = -1;
  printf("%u %u\n", x << s, x << neg);
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let x: u32 = black_box(1);
    let s: u32 = black_box(33);
    let neg: i32 = black_box(-1);
    println!("{} {}", x.wrapping_shl(s), x.wrapping_shl(neg as u32));
    assert_eq!(x.checked_shl(s), None);
}
"""
    naive = ok.replace("x.wrapping_shl(s)", "x << s")
    assert lab.same(c, ok) == "2 2147483648\n"
    lab.differs(c, naive)


def test_i4_integer_promotion(lab: Lab) -> None:
    """I4: `uint8 + int` and `uint16 * uint16` promote to int; trap: `u16 * u16` panics, compute in `u32`."""
    c = r"""
#include <stdio.h>
#include <stdint.h>
int main(void){
  volatile uint8_t a = 200;
  volatile int b = 100;
  volatile uint16_t p = 65535, q = 65535;
  uint16_t r = (uint16_t)(p * q);
  printf("%d %u %u\n", a + b, (unsigned)r, (uint32_t)(p * q));
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let a: u8 = black_box(200);
    let b: i32 = black_box(100);
    let (p, q): (u16, u16) = (black_box(65535), black_box(65535));
    let prod = p as u32 * q as u32;
    println!("{} {} {}", a as i32 + b, (prod as u16) as u32, prod);
}
"""
    naive_u16 = ok.replace("let prod = p as u32 * q as u32;", "let prod = (p * q) as u32;")
    naive_i32 = ok.replace("p as u32 * q as u32", "p as i32 * q as i32").replace("prod as u16", "prod as u16")
    naive_i32 = naive_i32.replace("let prod =", "let prod: u32 =").replace(
        "p as i32 * q as i32;", "(p as i32 * q as i32) as u32;"
    )
    assert _same_flags(lab, c, ok, WRAPV) == "300 1 4294836225\n"
    lab.differs(c, naive_u16)
    lab.differs(c, naive_i32)


def test_i5_division_truncates_and_int_min_traps(lab: Lab) -> None:
    """I5: `/` and `%` truncate toward zero (match); trap: `INT_MIN / -1` is UB in C, panics in Rust."""
    c = r"""
#include <stdio.h>
int main(void){
  volatile int a = -7, b = 2, c = 7, d = -2;
  printf("%d %d %d %d\n", a / b, a % b, c / d, c % d);
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let (a, b, c, d): (i32, i32, i32, i32) = (black_box(-7), black_box(2), black_box(7), black_box(-2));
    println!("{} {} {} {}", a / b, a % b, c / d, c % d);
}
"""
    c_ub = r"""
#include <stdio.h>
#include <limits.h>
int main(void){ volatile int m = INT_MIN, d = -1; printf("%d\n", m / d); return 0; }
"""
    rust_trap = r"""
fn main() {
    let m: i32 = std::hint::black_box(i32::MIN);
    let d: i32 = std::hint::black_box(-1);
    println!("{}", m / d);
}
"""
    assert lab.same(c, ok) == "-3 -1 -3 1\n"
    lab.differs(c_ub, rust_trap)


def test_i6_float_to_int_cast(lab: Lab) -> None:
    """I6: float -> int is `as`; in range both truncate toward zero; out-of-range saturates (matches arm64 C)."""
    c = r"""
#include <stdio.h>
#include <math.h>
int main(void){
  volatile double v[5] = {3.99, -3.99, 1e10, -1e10, NAN};
  for (int i = 0; i < 5; i++) printf("%d ", (int)v[i]);
  printf("\n");
  return 0;
}
"""
    ok = r"""
use std::hint::black_box;
fn main() {
    let v: [f64; 5] = [3.99, -3.99, 1e10, -1e10, f64::NAN];
    for x in v { print!("{} ", black_box(x) as i32); }
    println!();
}
"""
    assert lab.same(c, ok) == "3 -3 2147483647 -2147483648 0 \n"


_FLOAT_VALUES_C = (
    "{0.0, -0.0, 1.0, 0.1, 1.0/3.0, 1e21, 1e-5, 123456789.0, 5e-324, 1.7976931348623157e308, NAN, INFINITY, -INFINITY}"
)
_FLOAT_VALUES_RS = (
    "[0.0, -0.0, 1.0, 0.1, 1.0 / 3.0, 1e21, 1e-5, 123456789.0, 5e-324, 1.7976931348623157e308,"
    " f64::NAN, f64::INFINITY, f64::NEG_INFINITY]"
)


def test_i7_double_printing_snprintf_and_handwritten(lab: Lab) -> None:
    """I7: `%g`/`%.17g`/`%f`: Stage 1 `snprintf`, Stage 2 hand-written formatter; trap: Rust `{}` / `{:e}` differ."""
    c = (
        r"""
#include <stdio.h>
#include <math.h>
int main(void){
  double v[] = """
        + _FLOAT_VALUES_C
        + r""";
  for (int i = 0; i < 13; i++) printf("%g|%.17g|%f\n", v[i], v[i], v[i]);
  return 0;
}
"""
    )
    stage1 = (
        r"""
use std::ffi::{c_char, c_int};
extern "C" {
    fn snprintf(buf: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
}
fn f(fmt: &[u8], v: f64) -> String {
    let mut buf = [0 as c_char; 512];
    let n = unsafe { snprintf(buf.as_mut_ptr(), buf.len(), fmt.as_ptr() as *const c_char, v) } as usize;
    buf[..n].iter().map(|&c| c as u8 as char).collect()
}
fn main() {
    let v: [f64; 13] = """
        + _FLOAT_VALUES_RS
        + r""";
    for x in v {
        println!("{}|{}|{}", f(b"%g\0", x), f(b"%.17g\0", x), f(b"%f\0", x));
    }
}
"""
    )
    stage2 = (
        r"""
fn strip(s: &str) -> String {
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s.to_string() }
}
fn special(v: f64) -> Option<String> {
    if v.is_nan() { Some("nan".into()) }
    else if v.is_infinite() { Some(if v < 0.0 { "-inf" } else { "inf" }.into()) }
    else { None }
}
fn fmt_g(v: f64, p: usize) -> String {
    if let Some(s) = special(v) { return s; }
    let e = format!("{:.*e}", p - 1, v);
    let (mant, exp) = e.split_once('e').unwrap();
    let x: i32 = exp.parse().unwrap();
    if x < -4 || x >= p as i32 {
        format!("{}e{}{:02}", strip(mant), if x < 0 { '-' } else { '+' }, x.abs())
    } else {
        strip(&format!("{:.*}", (p as i32 - 1 - x) as usize, v))
    }
}
fn fmt_f(v: f64) -> String { special(v).unwrap_or_else(|| format!("{:.6}", v)) }
fn main() {
    let v: [f64; 13] = """
        + _FLOAT_VALUES_RS
        + r""";
    for x in v {
        println!("{}|{}|{}", fmt_g(x, 6), fmt_g(x, 17), fmt_f(x));
    }
}
"""
    )
    naive = stage2.replace(
        'println!("{}|{}|{}", fmt_g(x, 6), fmt_g(x, 17), fmt_f(x));', 'println!("{}|{:e}|{}", x, x, x);'
    )
    out = lab.same(c, stage1)
    assert out.splitlines()[5] == "1e+21|1e+21|1000000000000000000000.000000"
    assert lab.same(c, stage2) == out
    lab.differs(c, naive)


_STRTOD_C = r"""
#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
int main(void){
  const char *cases[] = {"3.5", "  3.5abc", "1e5", ".5", "-inf", "infinity", "abc", "1e", "1e+", "-0",
                         "  +7.25e-1x", "", "5.", "1.5e3e4"};
  for (int i = 0; i < 14; i++) {
    char *end;
    double v = strtod(cases[i], &end);
    printf("[%s] %.6f %td\n", cases[i], v, end - cases[i]);
  }
  return 0;
}
"""
_STRTOD_CASES_RS = (
    '["3.5", "  3.5abc", "1e5", ".5", "-inf", "infinity", "abc", "1e", "1e+", "-0", "  +7.25e-1x", "", "5.", "1.5e3e4"]'
)


def test_i8_strtod_libc_and_prefix_scanner(lab: Lab) -> None:
    """I8: `strtod`: Stage 1 `libc::strtod`, Stage 2 prefix scanner + `parse`; trap: whole-string `parse` fails."""
    stage1 = (
        r"""
use std::ffi::{c_char, c_double};
extern "C" {
    fn strtod(s: *const c_char, end: *mut *mut c_char) -> c_double;
}
fn main() {
    let cases = """
        + _STRTOD_CASES_RS
        + r""";
    for s in cases {
        let mut z = s.as_bytes().to_vec();
        z.push(0);
        let mut end: *mut c_char = std::ptr::null_mut();
        let v = unsafe { strtod(z.as_ptr() as *const c_char, &mut end) };
        let n = end as usize - z.as_ptr() as usize;
        println!("[{}] {:.6} {}", s, v, n);
    }
}
"""
    )
    stage2 = (
        r"""
fn scan(s: &[u8]) -> (f64, usize) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') { i += 1; }
    let start = i;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') { i += 1; }
    let low: Vec<u8> = s[i..].iter().take(8).map(|b| b.to_ascii_lowercase()).collect();
    if low.starts_with(b"infinity") { i += 8; }
    else if low.starts_with(b"inf") { i += 3; }
    else {
        let d0 = i;
        while i < s.len() && s[i].is_ascii_digit() { i += 1; }
        let mut nd = i - d0;
        if i < s.len() && s[i] == b'.' {
            i += 1;
            let f0 = i;
            while i < s.len() && s[i].is_ascii_digit() { i += 1; }
            nd += i - f0;
        }
        if nd == 0 { return (0.0, 0); }
        if i < s.len() && (s[i] == b'e' || s[i] == b'E') {
            let mut j = i + 1;
            if j < s.len() && (s[j] == b'+' || s[j] == b'-') { j += 1; }
            let e0 = j;
            while j < s.len() && s[j].is_ascii_digit() { j += 1; }
            if j > e0 { i = j; }
        }
    }
    let t = std::str::from_utf8(&s[start..i]).unwrap();
    (t.parse::<f64>().unwrap(), i)
}
fn main() {
    let cases = """
        + _STRTOD_CASES_RS
        + r""";
    for s in cases {
        let (v, n) = scan(s.as_bytes());
        println!("[{}] {:.6} {}", s, v, n);
    }
}
"""
    )
    naive = stage2.replace(
        "let (v, n) = scan(s.as_bytes());",
        "let v = s.parse::<f64>().unwrap_or(0.0); let n = if s.parse::<f64>().is_ok() { s.len() } else { 0 };",
    )
    out = lab.same(_STRTOD_C, stage1)
    assert "[  3.5abc] 3.500000 5" in out
    assert lab.same(_STRTOD_C, stage2) == out
    lab.differs(_STRTOD_C, naive)


def test_i8_strtod_hex_float_rust_parse_rejects(lab: Lab) -> None:
    """I8: trap: C `strtod` accepts hex floats; Rust `parse::<f64>` rejects them."""
    c = r"""
#include <stdio.h>
#include <stdlib.h>
int main(void){
  char *end;
  const char *s = "0x1p3";
  double v = strtod(s, &end);
  printf("%.3f %td\n", v, end - s);
  return 0;
}
"""
    naive = r"""
fn main() {
    let s = "0x1p3";
    match s.parse::<f64>() { Ok(v) => println!("{:.3} {}", v, s.len()), Err(_) => println!("0.000 0") }
}
"""
    lab.differs(c, naive)


_STRTOL_C = r"""
#include <stdio.h>
#include <stdlib.h>
#include <errno.h>
#include <stddef.h>
struct Case { const char *s; int base; };
int main(void){
  struct Case cs[] = {{"  -42xyz", 10}, {"99999999999999999999", 10}, {"-99999999999999999999", 10},
                      {"0x1F", 16}, {"0x1F", 0}, {"010", 0}, {"abc", 10}, {"0xg", 16}, {"+7", 10}, {"", 10}};
  for (int i = 0; i < 10; i++) {
    char *end;
    errno = 0;
    long v = strtol(cs[i].s, &end, cs[i].base);
    printf("%ld %td %d\n", v, end - cs[i].s, errno == ERANGE);
  }
  return 0;
}
"""
_STRTOL_CASES_RS = (
    '[("  -42xyz", 10), ("99999999999999999999", 10), ("-99999999999999999999", 10), ("0x1F", 16),'
    ' ("0x1F", 0), ("010", 0), ("abc", 10), ("0xg", 16), ("+7", 10), ("", 10)]'
)


def test_i9_strtol_libc_and_prefix_scanner(lab: Lab) -> None:
    """I9: `strtol`: Stage 1 `libc::strtol`, Stage 2 prefix scanner + `from_str_radix` + clamp; trap: `parse` errors."""
    stage1 = (
        r"""
use std::ffi::{c_char, c_int, c_long};
extern "C" {
    fn strtol(s: *const c_char, end: *mut *mut c_char, base: c_int) -> c_long;
    fn __error() -> *mut c_int;
}
fn main() {
    let cases: [(&str, i32); 10] = """
        + _STRTOL_CASES_RS
        + r""";
    for (s, base) in cases {
        let mut z = s.as_bytes().to_vec();
        z.push(0);
        let mut end: *mut c_char = std::ptr::null_mut();
        let (v, erange) = unsafe {
            *__error() = 0;
            let v = strtol(z.as_ptr() as *const c_char, &mut end, base);
            (v, *__error() == 34)
        };
        let n = end as usize - z.as_ptr() as usize;
        println!("{} {} {}", v, n, erange as i32);
    }
}
"""
    )
    stage2 = (
        r"""
fn strtol(s: &[u8], base: u32) -> (i64, usize, bool) {
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') { i += 1; }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') { neg = s[i] == b'-'; i += 1; }
    let mut base = base;
    let hex_prefix = i + 2 < s.len() && s[i] == b'0' && (s[i + 1] | 0x20) == b'x' && s[i + 2].is_ascii_hexdigit();
    if (base == 0 || base == 16) && hex_prefix { i += 2; base = 16; }
    else if base == 0 { base = if i < s.len() && s[i] == b'0' { 8 } else { 10 }; }
    let ds = i;
    while i < s.len() && (s[i] as char).to_digit(base).is_some() { i += 1; }
    if i == ds { return (0, 0, false); }
    let mag = i128::from_str_radix(std::str::from_utf8(&s[ds..i]).unwrap(), base).unwrap();
    let v = if neg { -mag } else { mag };
    if v > i64::MAX as i128 { (i64::MAX, i, true) }
    else if v < i64::MIN as i128 { (i64::MIN, i, true) }
    else { (v as i64, i, false) }
}
fn main() {
    let cases: [(&str, u32); 10] = """
        + _STRTOL_CASES_RS
        + r""";
    for (s, base) in cases {
        let (v, n, erange) = strtol(s.as_bytes(), base);
        println!("{} {} {}", v, n, erange as i32);
    }
}
"""
    )
    naive = stage2.replace(
        "let (v, n, erange) = strtol(s.as_bytes(), base);",
        "let r = i64::from_str_radix(s, base); let (v, n, erange) = (r.clone().unwrap_or(0), 0, r.is_err());",
    )
    out = lab.same(_STRTOL_C, stage1)
    assert out.splitlines()[1] == "9223372036854775807 20 1"
    assert lab.same(_STRTOL_C, stage2) == out
    lab.differs(_STRTOL_C, naive)


def test_i10_unsigned_minus_one_into_signed_sentinel(lab: Lab) -> None:
    """I10: unsigned `x - 1` into a signed `-1` sentinel; trap: plain `x - 1` panics at 0, `usize::MAX` is not -1."""
    c = r"""
#include <stdio.h>
int main(void){
  volatile unsigned vals[3] = {0, 1, 5};
  for (int i = 0; i < 3; i++) {
    unsigned toknext = vals[i];
    int toksuper = toknext - 1;
    printf("%d ", toksuper);
  }
  printf("\n");
  return 0;
}
"""
    stage1 = r"""
use std::ffi::{c_int, c_uint};
use std::hint::black_box;
fn main() {
    for v in [0u32, 1, 5] {
        let toknext: c_uint = black_box(v);
        let toksuper: c_int = toknext.wrapping_sub(1) as c_int;
        print!("{} ", toksuper);
    }
    println!();
}
"""
    stage2 = r"""
use std::hint::black_box;
fn main() {
    for v in [0usize, 1, 5] {
        let toknext = black_box(v);
        let toksuper: Option<usize> = toknext.checked_sub(1);
        print!("{} ", toksuper.map_or(-1, |t| t as i32));
    }
    println!();
}
"""
    naive_plain = stage2.replace("toknext.checked_sub(1)", "Some(toknext - 1)")
    naive_wrap = stage2.replace("toksuper.map_or(-1, |t| t as i32)", "toknext.wrapping_sub(1)")
    assert lab.same(c, stage1) == "-1 0 4 \n"
    assert lab.same(c, stage2) == "-1 0 4 \n"
    lab.differs(c, naive_plain)
    lab.differs(c, naive_wrap)
