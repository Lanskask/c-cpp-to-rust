"""Idiom rows V*: variadic functions (V1-V2).

Observable behaviour is tested through what a C-variadic callee sees. Defining a C-variadic function in Rust needs
nightly `c_variadic`; V1 also claims that, which is checked by asking stable rustc to compile one.
"""

from __future__ import annotations

import subprocess

from conftest import RUSTC, Lab

C_SUM = r"""
#include <stdio.h>
#include <stdarg.h>
/* sum n ints; the callee trusts `n`, the caller must pass exactly that many */
static int sum(int n, ...) {
    va_list ap; va_start(ap, n);
    int s = 0;
    for (int i = 0; i < n; i++) s += va_arg(ap, int);
    va_end(ap);
    return s;
}
/* format-driven variadic: 'i' int, 'd' double, 's' string */
static void show(const char *fmt, ...) {
    va_list ap; va_start(ap, fmt);
    for (; *fmt; fmt++) {
        if (*fmt == 'i') printf("i=%d\n", va_arg(ap, int));
        else if (*fmt == 'd') printf("d=%.2f\n", va_arg(ap, double));
        else if (*fmt == 's') printf("s=%s\n", va_arg(ap, const char *));
    }
    va_end(ap);
}
int main(void) {
    printf("%d\n", sum(3, 10, 20, 30));
    printf("%d\n", sum(0));
    show("isd", 7, "hi", 2.5);
    return 0;
}
"""

RUST_ARG = r"""
#[allow(dead_code)]
enum Arg<'a> { I(i32), D(f64), S(&'a str) }
fn sum(args: &[i32]) -> i32 { args.iter().sum() }
fn show(fmt: &str, args: &[Arg]) {
    for (c, a) in fmt.chars().zip(args) {
        match (c, a) {
            ('i', Arg::I(v)) => println!("i={}", v),
            ('d', Arg::D(v)) => println!("d={:.2}", v),
            ('s', Arg::S(v)) => println!("s={}", v),
            _ => panic!("format/argument mismatch"),
        }
    }
}
fn main() {
    println!("{}", sum(&[10, 20, 30]));
    println!("{}", sum(&[]));
    show("isd", &[Arg::I(7), Arg::S("hi"), Arg::D(2.5)]);
}
"""


def test_v1_variadic_fn_as_slice_of_args(lab: Lab) -> None:
    """V1: a variadic C function maps to `&[Arg]`; the slice carries the count and the types the callee trusted.

    Trap: defining a C-variadic fn in Rust needs nightly `c_variadic`; stable rustc must refuse it.
    """
    out = lab.same(C_SUM, RUST_ARG)
    assert out == "60\n0\ni=7\ns=hi\nd=2.50\n"
    # Stage 1 keep-via-libc route: calling a variadic libc function works on stable (declaration only).
    stage1 = r"""
extern "C" { fn snprintf(b: *mut std::os::raw::c_char, n: usize, f: *const std::os::raw::c_char, ...) -> i32; }
fn main() {
    let mut buf = [0u8; 32];
    let n = unsafe { snprintf(buf.as_mut_ptr() as *mut _, 32, b"%d,%s,%.2f\0".as_ptr() as *const _,
                              7i32, b"hi\0".as_ptr(), 2.5f64) };
    println!("{} {}", n, std::str::from_utf8(&buf[..n as usize]).unwrap());
}
"""
    c = r"""
#include <stdio.h>
int main(void) {
    char buf[32];
    int n = snprintf(buf, 32, "%d,%s,%.2f", 7, "hi", 2.5);
    printf("%d %s\n", n, buf);
    return 0;
}
"""
    lab.same(c, stage1)
    lab.n += 1
    src = lab.root / f"cvariadic{lab.n}.rs"
    src.write_text('unsafe extern "C" fn f(n: i32, mut _ap: ...) -> i32 { n }\nfn main() {}\n')
    r = subprocess.run(
        [RUSTC or "rustc", "--edition", "2021", "-o", str(src.with_suffix("")), str(src)],
        capture_output=True,
        text=True,
    )
    assert r.returncode != 0 and "C-variadic functions are unstable" in r.stderr, r.stderr


def test_v2_variadic_test_helper_as_expected_slice(lab: Lab) -> None:
    """V2: a variadic test helper (jsmn `tokeq`) becomes a slice of expected values in the translated Rust test.

    The variadic helper checks `n` tokens against `(type, start, end)` triples; the slice version must report the
    same pass/fail pattern, including the first mismatch.
    """
    c = r"""
#include <stdio.h>
#include <stdarg.h>
typedef struct { int type, start, end; } Tok;
/* like jsmn tokeq: n expected (type, start, end) triples as varargs */
static int tokeq(const Tok *t, int n, ...) {
    va_list ap; va_start(ap, n);
    int ok = 1;
    for (int i = 0; i < n; i++) {
        int ty = va_arg(ap, int), s = va_arg(ap, int), e = va_arg(ap, int);
        if (t[i].type != ty || t[i].start != s || t[i].end != e) { printf("mismatch at %d\n", i); ok = 0; break; }
    }
    va_end(ap);
    return ok;
}
int main(void) {
    Tok t[3] = {{1, 0, 9}, {3, 1, 4}, {4, 6, 8}};
    printf("%d\n", tokeq(t, 3, 1, 0, 9, 3, 1, 4, 4, 6, 8));
    printf("%d\n", tokeq(t, 3, 1, 0, 9, 3, 1, 5, 4, 6, 8));
    printf("%d\n", tokeq(t, 0));
    return 0;
}
"""
    rust = r"""
struct Tok { ty: i32, start: i32, end: i32 }
fn tokeq(t: &[Tok], expected: &[(i32, i32, i32)]) -> bool {
    for (i, &(ty, s, e)) in expected.iter().enumerate() {
        if t[i].ty != ty || t[i].start != s || t[i].end != e { println!("mismatch at {}", i); return false; }
    }
    true
}
fn main() {
    let t = [Tok { ty: 1, start: 0, end: 9 }, Tok { ty: 3, start: 1, end: 4 }, Tok { ty: 4, start: 6, end: 8 }];
    println!("{}", tokeq(&t, &[(1, 0, 9), (3, 1, 4), (4, 6, 8)]) as i32);
    println!("{}", tokeq(&t, &[(1, 0, 9), (3, 1, 5), (4, 6, 8)]) as i32);
    println!("{}", tokeq(&t, &[]) as i32);
}
"""
    assert lab.same(c, rust) == "1\nmismatch at 1\n0\n1\n"
