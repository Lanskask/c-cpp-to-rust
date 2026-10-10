"""Idiom rows L*: libc string, ctype, stdio and math calls (L1-L12).

Platform facts used here: macOS arm64, `char` signed, "C" locale unless a test calls `setlocale`.
"""

from __future__ import annotations

import struct
from pathlib import Path

from conftest import Lab

# Doubles as raw bit patterns so both programs see exactly the same inputs (including -0.0, NaN, subnormals).
FLOAT_VALUES = [
    0.0, -0.0, 1.0, -1.5, 0.1, 1.0 / 3.0, 100000.0, 1e6, 123456789.0, 0.0001, 0.00001, 1e21, -1e21,
    5e-324, 2.2250738585072014e-308, 1.7976931348623157e308, 0.5, 2.5, 1e-5, 99999.95,
]  # fmt: skip
FLOAT_BITS = [struct.unpack("<Q", struct.pack("<d", v))[0] for v in FLOAT_VALUES] + [
    0x7FF8000000000000,  # NaN
    0xFFF8000000000000,  # -NaN
    0x7FF0000000000000,  # +inf
    0xFFF0000000000000,  # -inf
]
C_BITS = "{" + ", ".join(f"0x{b:016X}ULL" for b in FLOAT_BITS) + "}"
RUST_BITS = "[" + ", ".join(f"0x{b:016X}u64" for b in FLOAT_BITS) + "]"

# What a Stage 2 port has to write by hand: Rust has no `%g`, and `{:e}` / `{}` differ from C for NaN, inf and
# the exponent (`1e21` vs `1e+21`).
RUST_FMT = r"""
fn strip(s: &str) -> String {
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s.to_string() }
}
fn special(v: f64) -> Option<String> {
    let sign = if v.is_sign_negative() { "-" } else { "" };
    // macOS libc prints NaN without a sign even for -NaN (glibc prints "-nan"); this port matches the macOS run.
    if v.is_nan() { Some("nan".to_string()) }
    else if v.is_infinite() { Some(format!("{}inf", sign)) }
    else { None }
}
fn fmt_f(v: f64, prec: usize) -> String {
    special(v).unwrap_or_else(|| format!("{:.*}", prec, v))
}
fn exp_style(a: f64, prec: usize) -> (String, i32) {
    let e = format!("{:.*e}", prec, a);
    let (m, x) = e.split_once('e').unwrap();
    (m.to_string(), x.parse().unwrap())
}
fn fmt_e(v: f64, prec: usize) -> String {
    if let Some(s) = special(v) { return s; }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    let (m, x) = exp_style(v.abs(), prec);
    format!("{}{}e{}{:02}", sign, m, if x < 0 { '-' } else { '+' }, x.abs())
}
fn fmt_g(v: f64, prec: usize) -> String {
    if let Some(s) = special(v) { return s; }
    let sign = if v.is_sign_negative() { "-" } else { "" };
    let p = prec.max(1);
    let a = v.abs();
    if a == 0.0 { return format!("{}0", sign); }
    let (m, x) = exp_style(a, p - 1);
    let body = if x < -4 || x >= p as i32 {
        format!("{}e{}{:02}", strip(&m), if x < 0 { '-' } else { '+' }, x.abs())
    } else {
        strip(&format!("{:.*}", (p as i32 - 1 - x) as usize, a))
    };
    format!("{}{}", sign, body)
}
"""

C_LINE = 'printf("%g|%.17g|%.3f|%e\\n", d, d, d, d);'


def test_l1_strlen_stops_at_first_nul(lab: Lab) -> None:
    """L1: `strlen` maps to `libc::strlen` / position of the first 0 byte; bytes after an embedded NUL are ignored."""
    c = r"""
#include <stdio.h>
#include <string.h>
int main(void) {
    char a[] = "hello\0world", e[] = "";
    printf("%zu %zu %zu\n", strlen(a), strlen(e), strlen(a + 6));
    return 0;
}
"""
    stage1 = r"""
extern "C" { fn strlen(s: *const std::os::raw::c_char) -> usize; }
fn main() { unsafe {
    let a = *b"hello\0world\0";
    let e = *b"\0";
    println!("{} {} {}", strlen(a.as_ptr() as *const _), strlen(e.as_ptr() as *const _),
             strlen(a.as_ptr().add(6) as *const _));
} }
"""
    stage2 = r"""
fn c_strlen(s: &[u8]) -> usize { s.iter().position(|&b| b == 0).unwrap_or(s.len()) }
fn main() {
    let a = b"hello\0world\0";
    println!("{} {} {}", c_strlen(a), c_strlen(b"\0"), c_strlen(&a[6..]));
}
"""
    assert lab.same(c, stage1) == "5 0 5\n"
    lab.same(c, stage2)


def test_l2_strcmp_sign_and_magnitude(lab: Lab) -> None:
    """L2: `strcmp`/`strncmp`/`memcmp` map to slice `.cmp()`; a caller that prints the magnitude needs the C way.

    Trap: `.cmp()` yields -1/0/1 while the C library (macOS) returns the byte difference.
    """
    c = r"""
#include <stdio.h>
#include <string.h>
static int sgn(int x) { return (x > 0) - (x < 0); }
int main(void) {
    char a1[] = "a", a2[] = "z", b1[] = "abc", b2[] = "abc";
    char c1[] = "ab\xff", c2[] = "ab\x01", d1[] = "abcd", d2[] = "abxx";
    printf("%d %d %d %d\n", sgn(strcmp(a1, a2)), sgn(strcmp(b1, b2)), sgn(strcmp(c1, c2)), sgn(strcmp(a2, a1)));
    printf("%d %d\n", sgn(strncmp(d1, d2, 2)), sgn(strncmp(d1, d2, 3)));
    printf("%d %d\n", sgn(memcmp(c1, c2, 3)), sgn(memcmp("\0a", "\0b", 2)));
    printf("%d %d %d\n", strcmp(a1, a2), strncmp(d1, d2, 4), memcmp(c1, c2, 3));
    return 0;
}
"""
    ok = r"""
use std::cmp::Ordering;
fn sgn(o: Ordering) -> i32 { o as i32 }
fn c_cmp(a: &[u8], b: &[u8]) -> i32 {
    for (x, y) in a.iter().zip(b) { if x != y { return *x as i32 - *y as i32; } }
    0
}
fn strlen(s: &[u8]) -> usize { s.iter().position(|&b| b == 0).unwrap() }
fn main() {
    let (a1, a2, b1, b2) = (b"a\0", b"z\0", b"abc\0", b"abc\0");
    let (c1, c2) = (b"ab\xff\0", b"ab\x01\0");
    let (d1, d2) = (b"abcd\0", b"abxx\0");
    let s = |x: &[u8], y: &[u8]| sgn(x[..strlen(x)].cmp(&y[..strlen(y)]));
    println!("{} {} {} {}", s(a1, a2), s(b1, b2), s(c1, c2), s(a2, a1));
    println!("{} {}", sgn(d1[..2].cmp(&d2[..2])), sgn(d1[..3].cmp(&d2[..3])));
    println!("{} {}", sgn(c1[..3].cmp(&c2[..3])), sgn(b"\0a"[..].cmp(&b"\0b"[..])));
    println!("{} {} {}", c_cmp(a1, a2), c_cmp(&d1[..4], &d2[..4]), c_cmp(&c1[..3], &c2[..3]));
}
"""
    out = lab.same(c, ok)
    assert out.splitlines()[0] == "-1 0 1 1"
    naive = ok.replace("c_cmp(a1, a2)", "sgn(a1.cmp(a2))")
    lab.differs(c, naive)


def test_l3_strchr_finds_terminator(lab: Lab) -> None:
    """L3: `strchr`/`strstr`/`memchr` map to `iter().position` / `windows().position`.

    Trap: `strchr(s, '\\0')` finds the terminator; searching a slice without it reports "not found".
    """
    c = r"""
#include <stdio.h>
#include <string.h>
static void show(const char *base, const char *p) { if (p) printf("%ld ", (long)(p - base)); else printf("NULL "); }
int main(void) {
    char s[] = "hello world", z[] = "ab\0cd";
    show(s, strchr(s, 'o'));
    show(s, strchr(s, 'x'));
    show(s, strchr(s, '\0'));
    show(s, strstr(s, "o w"));
    show(s, strstr(s, "xyz"));
    show(s, strstr(s, ""));
    show(z, memchr(z, 'd', 5));
    show(z, memchr(z, 'c', 2));
    printf("\n");
    return 0;
}
"""
    ok = r"""
fn show(p: Option<usize>) { match p { Some(i) => print!("{} ", i), None => print!("NULL ") } }
fn strlen(s: &[u8]) -> usize { s.iter().position(|&b| b == 0).unwrap() }
fn strchr(s: &[u8], c: u8) -> Option<usize> { s[..=strlen(s)].iter().position(|&b| b == c) }
fn strstr(s: &[u8], n: &[u8]) -> Option<usize> {
    let h = &s[..strlen(s)];
    if n.is_empty() { return Some(0); }
    h.windows(n.len()).position(|w| w == n)
}
fn main() {
    let s = b"hello world\0";
    let z = b"ab\0cd";
    show(strchr(s, b'o'));
    show(strchr(s, b'x'));
    show(strchr(s, 0));
    show(strstr(s, b"o w"));
    show(strstr(s, b"xyz"));
    show(strstr(s, b""));
    show(z[..5].iter().position(|&b| b == b'd'));
    show(z[..2].iter().position(|&b| b == b'c'));
    println!();
}
"""
    assert lab.same(c, ok) == "4 NULL 11 4 NULL 0 4 NULL \n"
    naive = ok.replace("s[..=strlen(s)].iter()", "s[..strlen(s)].iter()")
    lab.differs(c, naive)


CTYPE_C = r"""
#include <stdio.h>
#include <ctype.h>
#define TABLE(name, expr) printf(name ": "); for (int c = 0; c < 256; c++) putchar((expr) ? '1' : '0'); putchar('\n');
"""


def test_l4_isdigit_isxdigit_table(lab: Lab) -> None:
    """L4: `isdigit`/`isxdigit` over every `unsigned char` equal `u8::is_ascii_digit` / `is_ascii_hexdigit`.

    The trap (negative `char` is UB) is avoided by the `(unsigned char)` cast, which is `c as u8`.
    """
    c = (
        CTYPE_C
        + r"""
int main(void) {
    TABLE("digit", isdigit(c));
    TABLE("xdigit", isxdigit(c));
    char neg = (char)0xC3;
    printf("%d %d\n", isdigit((unsigned char)neg) != 0, isxdigit((unsigned char)neg) != 0);
    return 0;
}
"""
    )
    rs = r"""
fn table(name: &str, f: impl Fn(u8) -> bool) {
    let s: String = (0..=255u8).map(|c| if f(c) { '1' } else { '0' }).collect();
    println!("{}: {}", name, s);
}
fn main() {
    table("digit", |c| c.is_ascii_digit());
    table("xdigit", |c| c.is_ascii_hexdigit());
    let neg = 0xC3u8 as i8;
    println!("{} {}", (neg as u8).is_ascii_digit() as i32, (neg as u8).is_ascii_hexdigit() as i32);
}
"""
    out = lab.same(c, rs)
    assert out.splitlines()[0].count("1") == 10
    assert out.splitlines()[1].count("1") == 22


def test_l5_isspace_includes_vertical_tab(lab: Lab) -> None:
    """L5: `isspace` maps to a `matches!` over six bytes; `u8::is_ascii_whitespace` excludes `\\v` (0x0B)."""
    c = (
        CTYPE_C
        + r"""
int main(void) {
    TABLE("space", isspace(c));
    return 0;
}
"""
    )
    ok = r"""
fn main() {
    let s: String = (0..=255u8)
        .map(|b| if matches!(b, b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r') { '1' } else { '0' })
        .collect();
    println!("space: {}", s);
}
"""
    naive = ok.replace("matches!(b, b' ' | b'\\t' | b'\\n' | b'\\x0b' | b'\\x0c' | b'\\r')", "b.is_ascii_whitespace()")
    assert naive != ok
    out = lab.same(c, ok)
    assert out.count("1") == 6
    lab.differs(c, naive)


def test_l6_alpha_alnum_print_case_tables(lab: Lab) -> None:
    """L6: `isalpha`/`isalnum`/`isprint`/`toupper`/`tolower` in the "C" locale equal the ASCII helpers.

    Trap: after `setlocale` to a Latin-1 locale C classifies 0xE9 as alphabetic; the Rust ASCII helpers do not.
    """
    c = (
        CTYPE_C
        + r"""
int main(void) {
    TABLE("alpha", isalpha(c));
    TABLE("alnum", isalnum(c));
    TABLE("print", isprint(c));
    printf("upper:"); for (int c = 0; c < 256; c++) printf(" %d", toupper(c)); printf("\n");
    printf("lower:"); for (int c = 0; c < 256; c++) printf(" %d", tolower(c)); printf("\n");
    printf("underscore: %d\n", isalpha('_') != 0);
    return 0;
}
"""
    )
    rs = r"""
fn table(name: &str, f: impl Fn(u8) -> bool) {
    let s: String = (0..=255u8).map(|c| if f(c) { '1' } else { '0' }).collect();
    println!("{}: {}", name, s);
}
fn main() {
    table("alpha", |c| c.is_ascii_alphabetic());
    table("alnum", |c| c.is_ascii_alphanumeric());
    table("print", |c| (0x20..0x7f).contains(&c));
    print!("upper:"); for c in 0..=255u8 { print!(" {}", c.to_ascii_uppercase()); } println!();
    print!("lower:"); for c in 0..=255u8 { print!(" {}", c.to_ascii_lowercase()); } println!();
    println!("underscore: {}", b'_'.is_ascii_alphabetic() as i32);
}
"""
    out = lab.same(c, rs)
    assert out.endswith("underscore: 0\n")
    c_loc = r"""
#include <stdio.h>
#include <ctype.h>
#include <locale.h>
int main(void) {
    if (!setlocale(LC_ALL, "en_US.ISO8859-1")) { printf("no locale\n"); return 0; }
    printf("%d\n", isalpha(0xE9) != 0);
    return 0;
}
"""
    naive = 'fn main() { println!("{}", 0xE9u8.is_ascii_alphabetic() as i32); }\n'
    lab.differs(c_loc, naive)


def test_l7_printf_float_formats(lab: Lab) -> None:
    """L7: `printf` maps to `libc::printf` (Stage 1) or a `Write` target with hand-written `%g/%f/%e` (Stage 2).

    Floats tested: 0.0, -0.0, 1e21, subnormals, NaN, -NaN, inf, -inf and ordinary values.
    Trap: Rust `{}` is not `%g` (`1e+21`, `nan`, exponent width).
    """
    c = (
        r"""
#include <stdio.h>
#include <stdint.h>
#include <string.h>
int main(void) {
    uint64_t bits[] = """
        + C_BITS
        + r""";
    for (size_t i = 0; i < sizeof bits / sizeof bits[0]; i++) {
        double d; memcpy(&d, &bits[i], 8);
        """
        + C_LINE
        + r"""
    }
    return 0;
}
"""
    )
    stage1 = (
        r"""
extern "C" { fn printf(fmt: *const std::os::raw::c_char, ...) -> i32; }
fn main() {
    let bits: [u64; """
        + str(len(FLOAT_BITS))
        + r"""] = """
        + RUST_BITS
        + r""";
    for b in bits {
        let d = f64::from_bits(b);
        unsafe { printf(b"%g|%.17g|%.3f|%e\n\0".as_ptr() as *const _, d, d, d, d); }
    }
}
"""
    )
    stage2 = (
        RUST_FMT
        + r"""
use std::io::Write;
fn emit(out: &mut impl Write, d: f64) {
    writeln!(out, "{}|{}|{}|{}", fmt_g(d, 6), fmt_g(d, 17), fmt_f(d, 3), fmt_e(d, 6)).unwrap();
}
fn main() {
    let bits: [u64; """
        + str(len(FLOAT_BITS))
        + r"""] = """
        + RUST_BITS
        + r""";
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for b in bits { emit(&mut out, f64::from_bits(b)); }
}
"""
    )
    out = lab.same(c, stage1)
    assert "1e+21" in out and "nan|nan|nan|nan" in out and "-nan" not in out and "4.9406564584124654e-324" in out
    assert lab.same(c, stage2) == out
    naive = (
        r"""
fn main() {
    let bits: [u64; """
        + str(len(FLOAT_BITS))
        + r"""] = """
        + RUST_BITS
        + r""";
    for b in bits { let d = f64::from_bits(b); println!("{}|{:.17}|{:.3}|{:e}", d, d, d, d); }
}
"""
    )
    lab.differs(c, naive)


def test_l7_printf_stdio_buffering_interleave(lab: Lab) -> None:
    """L7: trap "buffering and stdout/stderr interleaving": libc `printf` and Rust `println!` do not share a buffer."""
    c = r"""
#include <stdio.h>
int main(void) {
    printf("a\n");
    puts("b");
    fprintf(stdout, "c\n");
    return 0;
}
"""
    stage1 = r"""
extern "C" { fn printf(fmt: *const std::os::raw::c_char, ...) -> i32; fn puts(s: *const std::os::raw::c_char) -> i32; }
fn main() { unsafe {
    printf(b"a\n\0".as_ptr() as *const _);
    puts(b"b\0".as_ptr() as *const _);
    printf(b"c\n\0".as_ptr() as *const _);
} }
"""
    stage2 = r"""
use std::io::Write;
fn emit(out: &mut impl Write) { writeln!(out, "a").unwrap(); writeln!(out, "b").unwrap(); writeln!(out, "c").unwrap(); }
fn main() { emit(&mut std::io::stdout()); }
"""
    assert lab.same(c, stage1) == "a\nb\nc\n"
    lab.same(c, stage2)
    mixed = r"""
extern "C" { fn printf(fmt: *const std::os::raw::c_char, ...) -> i32; }
fn main() {
    unsafe { printf(b"a\n\0".as_ptr() as *const _); }
    println!("b");
    unsafe { printf(b"c\n\0".as_ptr() as *const _); }
}
"""
    lab.differs(c, mixed)


def test_l8_snprintf_returns_would_be_length(lab: Lab) -> None:
    """L8: `snprintf` returns the length it would have written and truncates; `%g` floats go through it.

    Trap: reporting the truncated length (what `write!` into a bounded buffer naturally gives) changes the return.
    """
    c = (
        r"""
#include <stdio.h>
#include <stdint.h>
#include <string.h>
int main(void) {
    char small[5], zero[1] = {'Z'}, big[64];
    int n1 = snprintf(small, sizeof small, "%d-%s", 12345, "abcdef");
    printf("%d [%s]\n", n1, small);
    int n2 = snprintf(zero, 0, "%s", "xyz");
    printf("%d [%c]\n", n2, zero[0]);
    int n3 = snprintf(NULL, 0, "%05d|%-4s|%x", 42, "ab", 255);
    printf("%d\n", n3);
    uint64_t bits[] = """
        + C_BITS
        + r""";
    for (size_t i = 0; i < sizeof bits / sizeof bits[0]; i++) {
        double d; memcpy(&d, &bits[i], 8);
        int n = snprintf(big, sizeof big, "%g", d);
        printf("%d %s\n", n, big);
        n = snprintf(big, 6, "%.17g", d);
        printf("%d %s\n", n, big);
    }
    return 0;
}
"""
    )
    stage1 = (
        r"""
extern "C" { fn snprintf(buf: *mut std::os::raw::c_char, n: usize, fmt: *const std::os::raw::c_char, ...) -> i32; }
fn cs(b: &[u8]) -> String { String::from_utf8_lossy(&b[..b.iter().position(|&x| x == 0).unwrap()]).into_owned() }
fn main() { unsafe {
    let mut small = [0u8; 5];
    let n1 = snprintf(small.as_mut_ptr() as *mut _, 5, b"%d-%s\0".as_ptr() as *const _, 12345i32, b"abcdef\0".as_ptr());
    println!("{} [{}]", n1, cs(&small));
    let mut zero = [b'Z'; 1];
    let n2 = snprintf(zero.as_mut_ptr() as *mut _, 0, b"%s\0".as_ptr() as *const _, b"xyz\0".as_ptr());
    println!("{} [{}]", n2, zero[0] as char);
    let n3 = snprintf(std::ptr::null_mut(), 0, b"%05d|%-4s|%x\0".as_ptr() as *const _, 42i32, b"ab\0".as_ptr(), 255u32);
    println!("{}", n3);
    let bits: [u64; """
        + str(len(FLOAT_BITS))
        + r"""] = """
        + RUST_BITS
        + r""";
    let mut big = [0u8; 64];
    for b in bits {
        let d = f64::from_bits(b);
        let n = snprintf(big.as_mut_ptr() as *mut _, 64, b"%g\0".as_ptr() as *const _, d);
        println!("{} {}", n, cs(&big));
        let n = snprintf(big.as_mut_ptr() as *mut _, 6, b"%.17g\0".as_ptr() as *const _, d);
        println!("{} {}", n, cs(&big));
    }
} }
"""
    )
    stage2 = (
        RUST_FMT
        + r"""
use std::fmt::Write;
// snprintf over a String: returns the full length, keeps at most cap-1 bytes.
fn snp(cap: usize, full: &str) -> (i32, String) {
    let keep = if cap == 0 { 0 } else { full.len().min(cap - 1) };
    (full.len() as i32, full[..keep].to_string())
}
fn main() {
    let mut s = String::new();
    write!(s, "{}-{}", 12345, "abcdef").unwrap();
    let (n1, small) = snp(5, &s);
    println!("{} [{}]", n1, small);
    let (n2, _) = snp(0, "xyz");
    println!("{} [Z]", n2);
    let (n3, _) = snp(0, &format!("{:05}|{:<4}|{:x}", 42, "ab", 255));
    println!("{}", n3);
    let bits: [u64; """
        + str(len(FLOAT_BITS))
        + r"""] = """
        + RUST_BITS
        + r""";
    for b in bits {
        let d = f64::from_bits(b);
        let (n, t) = snp(64, &fmt_g(d, 6));
        println!("{} {}", n, t);
        let (n, t) = snp(6, &fmt_g(d, 17));
        println!("{} {}", n, t);
    }
}
"""
    )
    out = lab.same(c, stage1)
    assert out.startswith("12 [1234]\n3 [Z]\n13\n")
    assert lab.same(c, stage2) == out
    naive = stage2.replace("(full.len() as i32, full[..keep].to_string())", "(keep as i32, full[..keep].to_string())")
    assert naive != stage2
    lab.differs(c, naive)


def test_l9_file_roundtrip_binary_and_text_mode(lab: Lab, tmp_path: Path) -> None:
    """L9: `fopen`/`fread`/`fwrite`/`fclose` map to `std::fs::File`; mode `"w"` equals `"wb"` on POSIX."""
    data = r"{0x00, 0x0D, 0x0A, 0xFF, 0x41, 0x0A, 0x1A, 0x00}"
    c = (
        r"""
#include <stdio.h>
#include <string.h>
int main(void) {
    unsigned char data[8] = """
        + data
        + r""", back[16], back2[16];
    FILE *f = fopen("@DIR@/c_bin.dat", "wb");
    if (!f) return 1;
    size_t w = fwrite(data, 1, 8, f);
    fclose(f);
    f = fopen("@DIR@/c_txt.dat", "w");
    fwrite(data, 1, 8, f);
    fclose(f);
    f = fopen("@DIR@/c_bin.dat", "rb");
    size_t r = fread(back, 1, 16, f);
    fclose(f);
    f = fopen("@DIR@/c_txt.dat", "r");
    size_t r2 = fread(back2, 1, 16, f);
    fclose(f);
    printf("%zu %zu %zu same=%d\n", w, r, r2, r == r2 && memcmp(back, back2, r) == 0);
    for (size_t i = 0; i < r; i++) printf("%02x", back[i]);
    printf("\n");
    f = fopen("@DIR@/missing.dat", "rb");
    printf("missing=%d\n", f == NULL);
    return 0;
}
"""
    ).replace("@DIR@", str(tmp_path))
    stage1 = (
        r"""
use std::os::raw::{c_char, c_void};
extern "C" {
    fn fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
    fn fwrite(p: *const c_void, s: usize, n: usize, f: *mut c_void) -> usize;
    fn fread(p: *mut c_void, s: usize, n: usize, f: *mut c_void) -> usize;
    fn fclose(f: *mut c_void) -> i32;
}
fn p(name: &str) -> Vec<u8> { format!("@DIR@/{}\0", name).into_bytes() }
fn main() { unsafe {
    let data: [u8; 8] = [0x00, 0x0D, 0x0A, 0xFF, 0x41, 0x0A, 0x1A, 0x00];
    let mut back = [0u8; 16]; let mut back2 = [0u8; 16];
    let f = fopen(p("r_bin.dat").as_ptr() as *const _, b"wb\0".as_ptr() as *const _);
    if f.is_null() { std::process::exit(1); }
    let w = fwrite(data.as_ptr() as *const _, 1, 8, f);
    fclose(f);
    let f = fopen(p("r_txt.dat").as_ptr() as *const _, b"w\0".as_ptr() as *const _);
    fwrite(data.as_ptr() as *const _, 1, 8, f);
    fclose(f);
    let f = fopen(p("r_bin.dat").as_ptr() as *const _, b"rb\0".as_ptr() as *const _);
    let r = fread(back.as_mut_ptr() as *mut _, 1, 16, f);
    fclose(f);
    let f = fopen(p("r_txt.dat").as_ptr() as *const _, b"r\0".as_ptr() as *const _);
    let r2 = fread(back2.as_mut_ptr() as *mut _, 1, 16, f);
    fclose(f);
    println!("{} {} {} same={}", w, r, r2, (r == r2 && back[..r] == back2[..r]) as i32);
    let hex: String = back[..r].iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex);
    let f = fopen(p("missing.dat").as_ptr() as *const _, b"rb\0".as_ptr() as *const _);
    println!("missing={}", f.is_null() as i32);
} }
"""
    ).replace("@DIR@", str(tmp_path))
    stage2 = (
        r"""
use std::io::{Read, Write};
use std::fs::File;
fn main() {
    let data: [u8; 8] = [0x00, 0x0D, 0x0A, 0xFF, 0x41, 0x0A, 0x1A, 0x00];
    let d = "@DIR@";
    let mut f = File::create(format!("{}/s_bin.dat", d)).unwrap();
    f.write_all(&data).unwrap();
    drop(f);
    let mut f = File::create(format!("{}/s_txt.dat", d)).unwrap();
    f.write_all(&data).unwrap();
    drop(f);
    let mut back = Vec::new();
    File::open(format!("{}/s_bin.dat", d)).unwrap().read_to_end(&mut back).unwrap();
    let mut back2 = Vec::new();
    File::open(format!("{}/s_txt.dat", d)).unwrap().read_to_end(&mut back2).unwrap();
    println!("{} {} {} same={}", data.len(), back.len(), back2.len(), (back == back2) as i32);
    let hex: String = back.iter().map(|b| format!("{:02x}", b)).collect();
    println!("{}", hex);
    println!("missing={}", File::open(format!("{}/missing.dat", d)).is_err() as i32);
}
"""
    ).replace("@DIR@", str(tmp_path))
    out = lab.same(c, stage1)
    assert out == "8 8 8 same=1\n000d0aff410a1a00\nmissing=1\n"
    lab.same(c, stage2)


def test_l10_qsort_equal_elements_order(lab: Lab) -> None:
    """L10: `qsort` maps to `libc::qsort` / `sort_unstable_by`; order of equal elements is unspecified.

    Distinct keys agree; with equal keys the Rust sort leaves a different order than macOS `qsort` (Deviation).
    """
    c_distinct = r"""
#include <stdio.h>
#include <stdlib.h>
static int cmp(const void *a, const void *b) { int x = *(const int *)a, y = *(const int *)b; return (x > y) - (x < y); }
int main(void) {
    int d[] = {5, 3, 9, -1, 7, 0, 2, 8, 4};
    qsort(d, 9, sizeof(int), cmp);
    for (int i = 0; i < 9; i++) printf("%d ", d[i]);
    printf("\n");
    return 0;
}
"""
    stage2 = r"""
fn main() {
    let mut d = [5, 3, 9, -1, 7, 0, 2, 8, 4];
    d.sort_unstable_by(|a, b| a.cmp(b));
    for x in d { print!("{} ", x); }
    println!();
}
"""
    assert lab.same(c_distinct, stage2) == "-1 0 2 3 4 5 7 8 9 \n"

    c_equal = r"""
#include <stdio.h>
#include <stdlib.h>
typedef struct { int k; int t; } P;
static int cmp(const void *a, const void *b) { return ((const P *)a)->k - ((const P *)b)->k; }
int main(void) {
    P a[] = {{1,0},{0,1},{1,2},{0,3},{1,4},{0,5},{1,6},{0,7},{1,8},{0,9},{1,10},{0,11}};
    qsort(a, 12, sizeof(P), cmp);
    for (int i = 0; i < 12; i++) printf("%d:%d ", a[i].k, a[i].t);
    printf("\n");
    return 0;
}
"""
    stage1 = r"""
use std::os::raw::{c_int, c_void};
extern "C" {
    fn qsort(base: *mut c_void, n: usize, size: usize, cmp: extern "C" fn(*const c_void, *const c_void) -> c_int);
}
#[repr(C)] #[derive(Clone, Copy)] struct P { k: i32, t: i32 }
extern "C" fn cmp(a: *const c_void, b: *const c_void) -> c_int {
    unsafe { (*(a as *const P)).k - (*(b as *const P)).k }
}
fn main() {
    let mut a: Vec<P> = [(1,0),(0,1),(1,2),(0,3),(1,4),(0,5),(1,6),(0,7),(1,8),(0,9),(1,10),(0,11)]
        .iter().map(|&(k, t)| P { k, t }).collect();
    unsafe { qsort(a.as_mut_ptr() as *mut c_void, a.len(), 8, cmp); }
    for p in a { print!("{}:{} ", p.k, p.t); }
    println!();
}
"""
    lab.same(c_equal, stage1)
    naive = r"""
fn main() {
    let mut a = [(1,0),(0,1),(1,2),(0,3),(1,4),(0,5),(1,6),(0,7),(1,8),(0,9),(1,10),(0,11)];
    a.sort_unstable_by(|x, y| x.0.cmp(&y.0));
    for p in a { print!("{}:{} ", p.0, p.1); }
    println!();
}
"""
    lab.differs(c_equal, naive)


def test_l11_abs_floor_pow_fmod(lab: Lab) -> None:
    """L11: `abs`/`labs`/`fabs`/`floor`/`pow`/`fmod` map to `i32::abs`, `f64::abs`, `floor`, `powf`, `%`.

    Compared bit for bit (NaN as the word `nan`). Trap: `rem_euclid` is not `fmod`; `i32::MIN.abs()` panics in debug.
    """
    c = r"""
#include <stdio.h>
#include <stdlib.h>
#include <math.h>
#include <stdint.h>
#include <string.h>
static void pr(double d) {
    uint64_t b; memcpy(&b, &d, 8);
    if (d != d) printf("nan "); else printf("%016llx ", (unsigned long long)b);
}
int main(void) {
    volatile int i = -5; volatile long l = -7L; volatile double z = -0.0, f = -2.5, e = 1.0 / 3.0;
    printf("%d %ld\n", abs(i), labs(l));
    pr(fabs(z)); pr(floor(f)); pr(floor(z)); pr(floor(-0.5));
    pr(pow(2.0, 10.0)); pr(pow(2.5, 3.5)); pr(pow(-8.0, e)); pr(pow(0.0, -1.0)); pr(pow(-2.0, 3.0));
    volatile double zero = 0.0;
    pr(fmod(-7.5, 2.0)); pr(fmod(7.5, -2.0)); pr(fmod(5.0, zero)); pr(fmod(-6.0, 3.0));
    printf("\n");
    return 0;
}
"""
    stage2 = r"""
fn pr(d: f64) { if d.is_nan() { print!("nan "); } else { print!("{:016x} ", d.to_bits()); } }
fn main() {
    let i: i32 = std::hint::black_box(-5);
    let l: i64 = std::hint::black_box(-7);
    let z: f64 = std::hint::black_box(-0.0);
    let f: f64 = std::hint::black_box(-2.5);
    let e: f64 = std::hint::black_box(1.0 / 3.0);
    println!("{} {}", i.abs(), l.abs());
    pr(z.abs()); pr(f.floor()); pr(z.floor()); pr((-0.5f64).floor());
    pr(2.0f64.powf(10.0)); pr(2.5f64.powf(3.5)); pr((-8.0f64).powf(e)); pr(0.0f64.powf(-1.0)); pr((-2.0f64).powf(3.0));
    let zero: f64 = std::hint::black_box(0.0);
    pr(-7.5 % 2.0); pr(7.5 % -2.0); pr(5.0 % zero); pr(-6.0 % 3.0);
    println!();
}
"""
    out = lab.same(c, stage2)
    assert out.splitlines()[0] == "5 7"
    naive = stage2.replace("pr(-7.5 % 2.0);", "pr((-7.5f64).rem_euclid(2.0));")
    assert naive != stage2
    lab.differs(c, naive)
    r = lab.rust('fn main() { let m: i32 = std::hint::black_box(i32::MIN); println!("{}", m.abs()); }\n')
    assert r.code != 0 and "overflow" in r.err


def test_l12_rand_time_are_deterministic_under_injection(lab: Lab) -> None:
    """L12: `rand`/`time`/`clock` are non-deterministic; with a fixed `srand` seed or an injected source they repeat.

    Only the determinism property is checked, never the generated values.
    """
    c = r"""
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
int main(void) {
    int a[5], b[5], same = 1;
    srand(7);
    for (int i = 0; i < 5; i++) a[i] = rand();
    srand(7);
    for (int i = 0; i < 5; i++) b[i] = rand();
    for (int i = 0; i < 5; i++) same &= (a[i] == b[i]);
    time_t t1 = time(NULL), t2 = time(NULL);
    clock_t c1 = clock(), c2 = clock();
    printf("seeded-repeatable=%d time-monotonic=%d clock-monotonic=%d\n", same, t2 >= t1, c2 >= c1);
    return 0;
}
"""
    stage1 = r"""
extern "C" { fn srand(seed: u32); fn rand() -> i32; fn time(t: *mut i64) -> i64; fn clock() -> u64; }
fn main() { unsafe {
    let mut a = [0i32; 5]; let mut b = [0i32; 5];
    srand(7);
    for x in a.iter_mut() { *x = rand(); }
    srand(7);
    for x in b.iter_mut() { *x = rand(); }
    let t1 = time(std::ptr::null_mut()); let t2 = time(std::ptr::null_mut());
    let c1 = clock(); let c2 = clock();
    println!("seeded-repeatable={} time-monotonic={} clock-monotonic={}",
             (a == b) as i32, (t2 >= t1) as i32, (c2 >= c1) as i32);
} }
"""
    stage2 = r"""
trait Source { fn rand(&mut self) -> u32; fn now(&mut self) -> u64; }
struct Fixed { state: u32, tick: u64 }
impl Source for Fixed {
    fn rand(&mut self) -> u32 { self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345); self.state >> 1 }
    fn now(&mut self) -> u64 { self.tick += 1; self.tick }
}
fn draw(s: &mut impl Source) -> [u32; 5] { let mut a = [0; 5]; for x in a.iter_mut() { *x = s.rand(); } a }
fn main() {
    let a = draw(&mut Fixed { state: 7, tick: 0 });
    let b = draw(&mut Fixed { state: 7, tick: 0 });
    let mut s = Fixed { state: 7, tick: 0 };
    let (t1, t2) = (s.now(), s.now());
    println!("seeded-repeatable={} time-monotonic={} clock-monotonic={}",
             (a == b) as i32, (t2 >= t1) as i32, (t2 >= t1) as i32);
}
"""
    want = "seeded-repeatable=1 time-monotonic=1 clock-monotonic=1\n"
    assert lab.same(c, stage1) == want
    assert lab.same(c, stage2) == want
    assert lab.c(c).out == lab.c(c).out == want
