"""Idiom rows E*: errors and sentinels."""

from __future__ import annotations

from conftest import Lab


def test_e1_error_codes_one_to_one(lab: Lab) -> None:
    """E1: error `enum` becomes `Result<T, Error>` with `code()`; trap: `as i32` on a plain enum prints 0,1,2."""
    c = r"""
#include <stdio.h>
enum { E_NOMEM = -1, E_INVAL = -4, E_FULL = -9 };
static int op(int v) {
    if (v < 0) return E_INVAL;
    if (v > 100) return E_FULL;
    return 0;
}
int main(void) {
    int in[] = {5, -3, 500};
    for (int i = 0; i < 3; i++) printf("%d\n", op(in[i]));
    return 0;
}
"""
    ok = r"""
#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Error { Nomem, Inval, Full }
impl Error {
    fn code(self) -> i32 { match self { Error::Nomem => -1, Error::Inval => -4, Error::Full => -9 } }
}
fn op(v: i32) -> Result<(), Error> {
    if v < 0 { return Err(Error::Inval); }
    if v > 100 { return Err(Error::Full); }
    Ok(())
}
fn main() {
    for v in [5, -3, 500] {
        println!("{}", match op(v) { Ok(()) => 0, Err(e) => e.code() });
    }
}
"""
    naive = ok.replace("Err(e) => e.code()", "Err(e) => e as i32")
    assert lab.same(c, ok) == "0\n-4\n-9\n"
    lab.differs(c, naive)


def test_e2_null_kinds_stay_distinct(lab: Lab) -> None:
    """E2: NULL from bad input and NULL from allocation failure are different kinds; one `Option` loses the kind."""
    c = r"""
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
static char *make(const char *s, size_t n) {
    char *p;
    if (!s) return NULL;          /* precondition-guard */
    p = malloc(n);
    if (!p) return NULL;          /* allocation-guard */
    memset(p, 'x', n < 4 ? n : 4);
    return p;
}
static void show(const char *s, size_t n) {
    char *p = make(s, n);
    if (p) printf("ok\n");
    else printf("%s\n", s == NULL ? "precondition" : "allocation");
    free(p);
}
int main(void) {
    show(NULL, 8);
    show("abc", SIZE_MAX / 2);
    show("abc", 8);
    return 0;
}
"""
    ok = r"""
#[derive(Debug)]
enum Error { Precondition, Allocation }
fn make(s: Option<&str>, n: usize) -> Result<Vec<u8>, Error> {
    if s.is_none() { return Err(Error::Precondition); }
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve_exact(n).map_err(|_| Error::Allocation)?;
    Ok(v)
}
fn show(s: Option<&str>, n: usize) {
    match make(s, n) {
        Ok(_) => println!("ok"),
        Err(Error::Precondition) => println!("precondition"),
        Err(Error::Allocation) => println!("allocation"),
    }
}
fn main() {
    show(None, 8);
    show(Some("abc"), usize::MAX / 2);
    show(Some("abc"), 8);
}
"""
    naive = r"""
fn make(s: Option<&str>, n: usize) -> Option<Vec<u8>> {
    s?;
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve_exact(n).ok()?;
    Some(v)
}
fn show(s: Option<&str>, n: usize) {
    match make(s, n) { Some(_) => println!("ok"), None => println!("null") }
}
fn main() {
    show(None, 8);
    show(Some("abc"), usize::MAX / 2);
    show(Some("abc"), 8);
}
"""
    assert lab.same(c, ok) == "precondition\nallocation\nok\n"
    lab.differs(c, naive)


def test_e3_errno_read_right_after_call(lab: Lab) -> None:
    """E3: `errno` via `last_os_error`; trap: only meaningful right after the failing call, a later call overwrites."""
    c = r"""
#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    errno = 0;
    FILE *f = fopen("/nonexistent-dir-xyz/file", "r");
    int e1 = errno;
    strtol("99999999999999999999", NULL, 10);
    int e2 = errno;
    printf("%d %d %d\n", f == NULL, e1, e2);
    return 0;
}
"""
    head = r"""
use std::os::raw::{c_char, c_long, c_void};
extern "C" {
    fn fopen(p: *const c_char, m: *const c_char) -> *mut c_void;
    fn strtol(s: *const c_char, e: *mut *mut c_char, b: i32) -> c_long;
}
fn errno() -> i32 { std::io::Error::last_os_error().raw_os_error().unwrap_or(0) }
fn main() {
    let path = b"/nonexistent-dir-xyz/file\0".as_ptr() as *const c_char;
    let f = unsafe { fopen(path, b"r\0".as_ptr() as *const c_char) };
"""
    ok = (
        head
        + r"""    let e1 = errno();
    unsafe { strtol(b"99999999999999999999\0".as_ptr() as *const c_char, std::ptr::null_mut(), 10) };
    let e2 = errno();
    println!("{} {} {}", f.is_null() as i32, e1, e2);
}
"""
    )
    naive = (
        head
        + r"""    unsafe { strtol(b"99999999999999999999\0".as_ptr() as *const c_char, std::ptr::null_mut(), 10) };
    let e2 = errno();
    let e1 = e2;
    println!("{} {} {}", f.is_null() as i32, e1, e2);
}
"""
    )
    assert lab.same(c, ok) == "1 2 34\n"
    lab.differs(c, naive)


E4_C = r"""
#include <stdio.h>
static int parse(const char *s, int *out, int cap, int *pos, int *n) {
    int i = 0;
    *n = 0;
    for (; s[i]; i++) {
        if (s[i] >= '0' && s[i] <= '9') {
            if (*n >= cap) { *pos = i; return -2; }
            out[(*n)++] = s[i] - '0';
        } else if (s[i] != ',') { *pos = i; return -1; }
    }
    *pos = i;
    return *n;
}
int main(void) {
    const char *in[] = {"1,2,3", "4,5x,6", "1,2,3,4,5"};
    for (int k = 0; k < 3; k++) {
        int out[4], pos, n;
        int rc = parse(in[k], out, 4, &pos, &n);
        printf("rc=%d pos=%d n=%d\n", rc, pos, n);
    }
    return 0;
}
"""


def test_e4_partial_result_with_error(lab: Lab) -> None:
    """E4: error with partial state (position, tokens written): `Err` plus accessors; a bare `Err` loses them."""
    ok = r"""
#[derive(Clone, Copy)]
enum Error { Invalid, NoMem }
impl Error { fn code(self) -> i32 { match self { Error::Invalid => -1, Error::NoMem => -2 } } }
struct Parser { cap: usize, pos: usize, tokens: Vec<i32> }
impl Parser {
    fn new(cap: usize) -> Parser { Parser { cap, pos: 0, tokens: Vec::new() } }
    fn parse(&mut self, s: &[u8]) -> Result<usize, Error> {
        self.tokens.clear();
        let mut i = 0;
        while i < s.len() {
            let b = s[i];
            if b.is_ascii_digit() {
                if self.tokens.len() >= self.cap { self.pos = i; return Err(Error::NoMem); }
                self.tokens.push((b - b'0') as i32);
            } else if b != b',' { self.pos = i; return Err(Error::Invalid); }
            i += 1;
        }
        self.pos = i;
        Ok(self.tokens.len())
    }
}
fn main() {
    for s in ["1,2,3", "4,5x,6", "1,2,3,4,5"] {
        let mut p = Parser::new(4);
        let rc = match p.parse(s.as_bytes()) { Ok(n) => n as i32, Err(e) => e.code() };
        println!("rc={} pos={} n={}", rc, p.pos, p.tokens.len());
    }
}
"""
    naive = r"""
fn parse(s: &[u8], cap: usize) -> Result<Vec<i32>, i32> {
    let mut t = Vec::new();
    for &b in s {
        if b.is_ascii_digit() {
            if t.len() >= cap { return Err(-2); }
            t.push((b - b'0') as i32);
        } else if b != b',' { return Err(-1); }
    }
    Ok(t)
}
fn main() {
    for s in ["1,2,3", "4,5x,6", "1,2,3,4,5"] {
        match parse(s.as_bytes(), 4) {
            Ok(t) => println!("rc={} pos={} n={}", t.len(), s.len(), t.len()),
            Err(e) => println!("rc={} pos=0 n=0", e),
        }
    }
}
"""
    assert lab.same(E4_C, ok) == "rc=3 pos=5 n=3\nrc=-1 pos=3 n=2\nrc=-2 pos=8 n=4\n"
    lab.differs(E4_C, naive)


def test_e5_count_or_error_in_one_int(lab: Lab) -> None:
    """E5: `>= 0` count / `< 0` error becomes `Result<usize, Error>`; zero is a success, not an error."""
    c = r"""
#include <stdio.h>
static int count_digits(const char *s) {
    int n = 0;
    for (; *s; s++) {
        if (*s < '0' || *s > '9') return -3;
        n++;
    }
    return n;
}
int main(void) {
    const char *in[] = {"", "12345", "12a45"};
    for (int i = 0; i < 3; i++) printf("%d\n", count_digits(in[i]));
    return 0;
}
"""
    ok = r"""
#[derive(Clone, Copy)]
enum Error { BadChar }
impl Error { fn code(self) -> i32 { match self { Error::BadChar => -3 } } }
fn count_digits(s: &str) -> Result<usize, Error> {
    let mut n = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() { return Err(Error::BadChar); }
        n += 1;
    }
    Ok(n)
}
fn main() {
    for s in ["", "12345", "12a45"] {
        println!("{}", match count_digits(s) { Ok(n) => n as i32, Err(e) => e.code() });
    }
}
"""
    assert lab.same(c, ok) == "0\n5\n-3\n"


def test_e6_index_bound_by_real_length(lab: Lab) -> None:
    """E6: bound `arr[i]` by the real slice length, not a limit-only count (jsmn `num_tokens`); a bare index panics."""
    c = r"""
#include <stdio.h>
int main(void) {
    int arr[8] = {10, 11, 12, 13, 14, 15, 16, 17};
    unsigned count = 3;          /* limit only: C may read earlier entries past it */
    unsigned held[2] = {5, 7};   /* caller-held indices, both below 8 */
    (void)count;
    for (int k = 0; k < 2; k++) printf("%d\n", arr[held[k]]);
    return 0;
}
"""
    ok = r"""
#[derive(Debug)]
enum Error { OutOfRange }
fn at(arr: &[i32], i: usize) -> Result<i32, Error> { arr.get(i).copied().ok_or(Error::OutOfRange) }
fn main() {
    let arr = [10, 11, 12, 13, 14, 15, 16, 17];
    for i in [5usize, 7] { println!("{}", at(&arr, i).unwrap()); }
}
"""
    naive = ok.replace("arr.get(i)", "arr[..3].get(i)")
    assert lab.same(c, ok) == "15\n17\n"
    lab.differs(c, naive)
    # a caller-reachable `arr[i]` with a bad index is a panic (UB in C), `get` turns it into a value
    bare = 'fn main(){ let a = [1, 2, 3]; let i = std::hint::black_box(10usize); println!("{}", a[i]); }\n'
    guarded = (
        "fn main(){ let a = [1, 2, 3]; let i = std::hint::black_box(10usize); "
        'println!("{}", a.get(i).map_or("err".to_string(), |v| v.to_string())); }\n'
    )
    assert lab.rust(bare).code != 0
    g = lab.rust(guarded)
    assert (g.code, g.out) == (0, "err\n")
