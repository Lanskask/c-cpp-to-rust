"""Idiom rows A*: allocation, buffers and raw memory (A1-A11).

A6 (allocator hooks) is a design rule and has no test: it is observable only through a custom allocator.
"""

from __future__ import annotations

import subprocess

from conftest import RUSTC, Lab

EXTERN = r"""
extern "C" {
    fn malloc(n: usize) -> *mut u8;
    fn calloc(n: usize, size: usize) -> *mut u8;
    fn realloc(p: *mut u8, n: usize) -> *mut u8;
    fn free(p: *mut u8);
    fn strdup(s: *const std::os::raw::c_char) -> *mut std::os::raw::c_char;
}
"""


def rust_rejects(lab: Lab, source: str) -> str:
    """Compile `source`; assert that rustc refuses it and return the diagnostics."""
    lab.n += 1
    src = lab.root / f"reject{lab.n}.rs"
    src.write_text(source)
    r = subprocess.run(
        [RUSTC or "rustc", "--edition", "2021", "-o", str(src.with_suffix("")), str(src)],
        capture_output=True,
        text=True,
    )
    assert r.returncode != 0, "rustc unexpectedly accepted the program"
    return r.stderr


def test_a1_malloc_free_one_object_is_box(lab: Lab) -> None:
    """A1: `malloc`+`free` of one object maps to `libc::malloc`/`free` (Stage 1) or `Box<T>` (Stage 2)."""
    c = r"""
#include <stdio.h>
#include <stdlib.h>
typedef struct { int id; double w; } Obj;
int main(void) {
    Obj *o = malloc(sizeof *o);
    if (!o) return 1;
    o->id = 7; o->w = 2.5;
    printf("%d %.1f\n", o->id, o->w);
    free(o);
    return 0;
}
"""
    stage1 = (
        EXTERN
        + r"""
#[repr(C)] struct Obj { id: i32, w: f64 }
fn main() {
    unsafe {
        let o = malloc(std::mem::size_of::<Obj>()) as *mut Obj;
        if o.is_null() { std::process::exit(1); }
        (*o).id = 7; (*o).w = 2.5;
        println!("{} {:.1}", (*o).id, (*o).w);
        free(o as *mut u8);
    }
}
"""
    )
    stage2 = r"""
struct Obj { id: i32, w: f64 }
fn main() {
    let mut o = Box::new(Obj { id: 0, w: 0.0 });
    o.id = 7; o.w = 2.5;
    println!("{} {:.1}", o.id, o.w);
}
"""
    assert lab.same(c, stage1) == "7 2.5\n"
    assert lab.same(c, stage2) == "7 2.5\n"


def test_a2_calloc_zero_fills_vec_default(lab: Lab) -> None:
    """A2: `calloc` zero-fills, so it maps to `vec![T::default(); n]`; `malloc` + full write maps the same way.

    Trap: the naive `Vec::with_capacity(n)` has length 0, not n zeroed slots.
    """
    c = r"""
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    size_t n = 8;
    int *a = calloc(n, sizeof(int));
    double *d = calloc(n, sizeof(double));
    long s = 0; double t = 1.0;
    for (size_t i = 0; i < n; i++) { s += a[i]; t += d[i]; }
    printf("%ld %.1f\n", s, t);
    int *m = malloc(n * sizeof(int));
    for (size_t i = 0; i < n; i++) m[i] = (int)(i * i);
    s = 0;
    for (size_t i = 0; i < n; i++) s += m[i];
    printf("%ld %zu\n", s, n);
    free(a); free(d); free(m);
    return 0;
}
"""
    stage1 = (
        EXTERN
        + r"""
fn main() { unsafe {
    let n = 8usize;
    let a = calloc(n, 4) as *mut i32;
    let d = calloc(n, 8) as *mut f64;
    let (mut s, mut t) = (0i64, 1.0f64);
    for i in 0..n { s += *a.add(i) as i64; t += *d.add(i); }
    println!("{} {:.1}", s, t);
    let m = malloc(n * 4) as *mut i32;
    for i in 0..n { *m.add(i) = (i * i) as i32; }
    s = 0;
    for i in 0..n { s += *m.add(i) as i64; }
    println!("{} {}", s, n);
    free(a as *mut u8); free(d as *mut u8); free(m as *mut u8);
} }
"""
    )
    stage2 = r"""
fn main() {
    let n = 8usize;
    let a = vec![0i32; n];
    let d = vec![0f64; n];
    let mut s: i64 = a.iter().map(|&x| x as i64).sum();
    let t = 1.0 + d.iter().sum::<f64>();
    println!("{} {:.1}", s, t);
    let mut m = vec![i32::default(); n];
    for i in 0..n { m[i] = (i * i) as i32; }
    s = m.iter().map(|&x| x as i64).sum();
    println!("{} {}", s, n);
}
"""
    lab.same(c, stage1)
    lab.same(c, stage2)
    c_len = r"""
#include <stdio.h>
int main(void) { size_t n = 8; printf("%zu\n", n); return 0; }
"""
    naive = 'fn main() { let v: Vec<i32> = Vec::with_capacity(8); println!("{}", v.len()); }\n'
    lab.differs(c_len, naive)


def test_a3_realloc_growth_is_push(lab: Lab) -> None:
    """A3: `realloc` growth maps to `libc::realloc` / `Vec::push`; a pointer into the old block must not survive.

    Trap: a reference held across `push` is rejected by the borrow checker; use an index.
    """
    c = r"""
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    size_t cap = 2, len = 0;
    int *p = malloc(cap * sizeof *p);
    for (int i = 0; i < 20; i++) {
        if (len == cap) { cap *= 2; p = realloc(p, cap * sizeof *p); }
        p[len++] = i * 3;
    }
    long s = 0;
    for (size_t i = 0; i < len; i++) s += p[i];
    printf("%zu %zu %ld %d\n", len, cap, s, p[0]);
    free(p);
    return 0;
}
"""
    stage1 = (
        EXTERN
        + r"""
fn main() { unsafe {
    let (mut cap, mut len) = (2usize, 0usize);
    let mut p = malloc(cap * 4) as *mut i32;
    for i in 0..20 {
        if len == cap { cap *= 2; p = realloc(p as *mut u8, cap * 4) as *mut i32; }
        *p.add(len) = i * 3; len += 1;
    }
    let mut s = 0i64;
    for i in 0..len { s += *p.add(i) as i64; }
    println!("{} {} {} {}", len, cap, s, *p);
    free(p as *mut u8);
} }
"""
    )
    stage2 = r"""
fn main() {
    let mut v: Vec<i32> = Vec::with_capacity(2);
    for i in 0..20 { v.push(i * 3); }
    let first = 0usize;
    let s: i64 = v.iter().map(|&x| x as i64).sum();
    println!("{} {} {} {}", v.len(), v.capacity().max(32), s, v[first]);
}
"""
    out = lab.same(c, stage1)
    assert out == "20 32 570 0\n"
    lab.same(c, stage2)
    err = rust_rejects(
        lab,
        'fn main() { let mut v = vec![1, 2, 3]; let first = &v[0]; v.push(4); println!("{}", first); }\n',
    )
    assert "E0502" in err


def test_a4_malloc_failure_branch(lab: Lab) -> None:
    """A4: the OOM check is unreachable in Stage 2 (Rust aborts); a precondition guard that looks the same must stay.

    Trap: deleting `if (!s) return -1` for a NULL *argument* changes behaviour.
    """
    c_oom = r"""
#include <stdio.h>
#include <stdlib.h>
int main(void) {
    volatile size_t n = (size_t)-1 / 2;
    char *p = malloc(n);
    if (!p) { printf("null\n"); return 0; }
    printf("got\n");
    free(p);
    return 0;
}
"""
    stage1 = (
        EXTERN
        + r"""
fn main() { unsafe {
    let p = malloc(usize::MAX / 2);
    if p.is_null() { println!("null"); return; }
    println!("got");
    free(p);
} }
"""
    )
    assert lab.same(c_oom, stage1) == "null\n"
    # Stage 2 deletes the branch: Rust aborts on OOM, which is a Deviation (not a NULL return).
    stage2 = (
        'fn main() { let n = std::hint::black_box(usize::MAX / 2); let v = vec![0u8; n]; println!("{}", v.len()); }\n'
    )
    lab.differs(c_oom, stage2)

    c_guard = r"""
#include <stdio.h>
#include <string.h>
static int length(const char *s) { if (!s) return -1; return (int)strlen(s); }
int main(void) {
    volatile const char *n = NULL;
    printf("%d %d\n", length("abc"), length((const char *)n));
    return 0;
}
"""
    ok = r"""
fn length(s: Option<&[u8]>) -> i32 { match s { None => -1, Some(b) => b.len() as i32 } }
fn main() { println!("{} {}", length(Some(b"abc")), length(None)); }
"""
    naive = r"""
fn length(s: Option<&[u8]>) -> i32 { s.unwrap().len() as i32 }
fn main() { println!("{} {}", length(Some(b"abc")), length(None)); }
"""
    assert lab.same(c_guard, ok) == "3 -1\n"
    lab.differs(c_guard, naive)


def test_a5_strdup_is_independent_copy(lab: Lab) -> None:
    """A5: `strdup` maps to `libc::strdup` / `Vec<u8>`; the copy is independent of the original."""
    c = r"""
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
int main(void) {
    char src[] = "hello";
    char *d = strdup(src);
    d[0] = 'J';
    src[4] = '!';
    printf("%s %s %zu\n", src, d, strlen(d));
    free(d);
    return 0;
}
"""
    stage1 = (
        EXTERN
        + r"""
extern "C" { fn strlen(s: *const std::os::raw::c_char) -> usize; }
fn main() { unsafe {
    let mut src = *b"hello\0";
    let d = strdup(src.as_ptr() as *const _) as *mut u8;
    *d = b'J';
    src[4] = b'!';
    let s = std::str::from_utf8(&src[..5]).unwrap();
    let ds = std::ffi::CStr::from_ptr(d as *const _).to_str().unwrap();
    println!("{} {} {}", s, ds, strlen(d as *const _));
    free(d);
} }
"""
    )
    stage2 = r"""
fn main() {
    let mut src = b"hello".to_vec();
    let mut d = src.clone();
    d[0] = b'J';
    src[4] = b'!';
    println!("{} {} {}", String::from_utf8(src).unwrap(), String::from_utf8(d.clone()).unwrap(), d.len());
}
"""
    assert lab.same(c, stage1) == "hell! Jello 5\n"
    lab.same(c, stage2)


def test_a7_linked_list_as_arena_indices(lab: Lab) -> None:
    """A7: next/prev pointer list becomes `Vec<Node>` + indices; detach then re-attach order must survive.

    Trap: modelling the list as a plain `Vec` with `swap_remove` changes iteration order.
    """
    c = r"""
#include <stdio.h>
typedef struct N { int v; struct N *next, *prev; } N;
static N *head, *tail;
static void push(N *n) { n->next = 0; n->prev = tail; if (tail) tail->next = n; else head = n; tail = n; }
static void detach(N *n) {
    if (n->prev) n->prev->next = n->next; else head = n->next;
    if (n->next) n->next->prev = n->prev; else tail = n->prev;
    n->next = n->prev = 0;
}
static void insert_head(N *n) { n->prev = 0; n->next = head; if (head) head->prev = n; else tail = n; head = n; }
int main(void) {
    static N nodes[4] = {{10,0,0},{20,0,0},{30,0,0},{40,0,0}};
    for (int i = 0; i < 4; i++) push(&nodes[i]);
    detach(&nodes[1]);
    insert_head(&nodes[1]);
    detach(&nodes[3]);
    for (N *n = head; n; n = n->next) printf("%d ", n->v);
    printf("|");
    for (N *n = tail; n; n = n->prev) printf(" %d", n->v);
    printf("\n");
    return 0;
}
"""
    stage2 = r"""
struct Node { v: i32, next: Option<usize>, prev: Option<usize> }
struct List { nodes: Vec<Node>, head: Option<usize>, tail: Option<usize> }
impl List {
    fn push(&mut self, i: usize) {
        self.nodes[i].next = None; self.nodes[i].prev = self.tail;
        match self.tail { Some(t) => self.nodes[t].next = Some(i), None => self.head = Some(i) }
        self.tail = Some(i);
    }
    fn detach(&mut self, i: usize) {
        let (p, n) = (self.nodes[i].prev, self.nodes[i].next);
        match p { Some(p) => self.nodes[p].next = n, None => self.head = n }
        match n { Some(n) => self.nodes[n].prev = p, None => self.tail = p }
        self.nodes[i].next = None; self.nodes[i].prev = None;
    }
    fn insert_head(&mut self, i: usize) {
        self.nodes[i].prev = None; self.nodes[i].next = self.head;
        match self.head { Some(h) => self.nodes[h].prev = Some(i), None => self.tail = Some(i) }
        self.head = Some(i);
    }
}
fn main() {
    let nodes = [10, 20, 30, 40].iter().map(|&v| Node { v, next: None, prev: None }).collect();
    let mut l = List { nodes, head: None, tail: None };
    for i in 0..4 { l.push(i); }
    l.detach(1);
    l.insert_head(1);
    l.detach(3);
    let mut out = String::new();
    let mut c = l.head;
    while let Some(i) = c { out += &format!("{} ", l.nodes[i].v); c = l.nodes[i].next; }
    out += "|";
    let mut c = l.tail;
    while let Some(i) = c { out += &format!(" {}", l.nodes[i].v); c = l.nodes[i].prev; }
    println!("{}", out);
}
"""
    assert lab.same(c, stage2) == "20 10 30 | 30 10 20\n"
    naive = r"""
fn main() {
    let mut v = vec![10, 20, 30, 40];
    let x = v.swap_remove(1); v.insert(0, x);
    v.swap_remove(3);
    let f: Vec<String> = v.iter().map(|x| x.to_string()).collect();
    let b: Vec<String> = v.iter().rev().map(|x| x.to_string()).collect();
    println!("{} | {}", f.join(" ") + " ", b.join(" "));
}
"""
    lab.differs(c, naive)


def test_a8_caller_owned_buffer_nomem_is_a_result(lab: Lab) -> None:
    """A8: caller-owned buffer maps to `&mut [T]`; "too small" is a returned error, not an allocation.

    Trap: growing a `Vec` instead of returning NOMEM changes the result.
    """
    c = r"""
#include <stdio.h>
#define NOMEM (-1)
static int fill(int *buf, unsigned cap, unsigned n) {
    if (n > cap) return NOMEM;
    for (unsigned i = 0; i < n; i++) buf[i] = (int)(i * 2);
    return (int)n;
}
int main(void) {
    int buf[4];
    int r1 = fill(buf, 4, 3), r2 = fill(buf, 4, 5);
    printf("%d %d %d %d\n", r1, r2, buf[0], buf[2]);
    return 0;
}
"""
    ok = r"""
const NOMEM: i32 = -1;
fn fill(buf: &mut [i32], n: usize) -> i32 {
    if n > buf.len() { return NOMEM; }
    for i in 0..n { buf[i] = (i * 2) as i32; }
    n as i32
}
fn main() {
    let mut buf = [0i32; 4];
    let r1 = fill(&mut buf, 3);
    let r2 = fill(&mut buf, 5);
    println!("{} {} {} {}", r1, r2, buf[0], buf[2]);
}
"""
    naive = r"""
fn fill(buf: &mut Vec<i32>, n: usize) -> i32 {
    buf.clear();
    for i in 0..n { buf.push((i * 2) as i32); }
    n as i32
}
fn main() {
    let mut buf: Vec<i32> = Vec::with_capacity(4);
    let r1 = fill(&mut buf, 3);
    let r2 = fill(&mut buf, 5);
    println!("{} {} {} {}", r1, r2, buf[0], buf[2]);
}
"""
    assert lab.same(c, ok) == "3 -1 0 4\n"
    lab.differs(c, naive)


def test_a9_memcpy_copy_from_slice(lab: Lab) -> None:
    """A9: `memcpy` maps to `copy_nonoverlapping` / `copy_from_slice`.

    Trap: overlap is UB in C; in Rust the borrow checker refuses it, and `copy_from_slice` panics on a length
    mismatch where `memcpy(dst, src, n)` copies a prefix.
    """
    c = r"""
#include <stdio.h>
#include <string.h>
int main(void) {
    char src[] = "hello world", dst[16];
    memset(dst, '.', sizeof dst); dst[15] = 0;
    memcpy(dst + 2, src + 6, 5);
    printf("%s\n", dst);
    return 0;
}
"""
    stage1 = r"""
fn main() {
    let src = *b"hello world\0";
    let mut dst = [b'.'; 16];
    dst[15] = 0;
    unsafe { core::ptr::copy_nonoverlapping(src.as_ptr().add(6), dst.as_mut_ptr().add(2), 5); }
    println!("{}", std::str::from_utf8(&dst[..15]).unwrap());
}
"""
    stage2 = r"""
fn main() {
    let src = *b"hello world\0";
    let mut dst = [b'.'; 16];
    dst[15] = 0;
    dst[2..7].copy_from_slice(&src[6..11]);
    println!("{}", std::str::from_utf8(&dst[..15]).unwrap());
}
"""
    assert lab.same(c, stage1) == "..world........\n"
    lab.same(c, stage2)
    naive = r"""
fn main() {
    let src = *b"hello world\0";
    let mut dst = [b'.'; 16];
    dst[2..].copy_from_slice(&src[6..11]);
    println!("{}", std::str::from_utf8(&dst[..15]).unwrap());
}
"""
    lab.differs(c, naive)
    err = rust_rejects(
        lab,
        "fn main() { let mut v = [1u8, 2, 3, 4, 5, 6];"
        " let (a, b) = (&mut v[0..4], &v[2..6]); a.copy_from_slice(b); }\n",
    )
    assert "E0502" in err


def test_a10_memmove_copy_within(lab: Lab) -> None:
    """A10: `memmove` (overlap allowed, both directions) maps to `ptr::copy` / `copy_within`."""
    c = r"""
#include <stdio.h>
#include <string.h>
int main(void) {
    char a[] = "0123456789", b[] = "0123456789";
    memmove(a + 2, a, 6);
    memmove(b, b + 3, 6);
    printf("%s %s\n", a, b);
    return 0;
}
"""
    stage1 = r"""
fn main() {
    let mut a = *b"0123456789";
    let mut b = *b"0123456789";
    unsafe {
        core::ptr::copy(a.as_ptr(), a.as_mut_ptr().add(2), 6);
        core::ptr::copy(b.as_ptr().add(3), b.as_mut_ptr(), 6);
    }
    println!("{} {}", std::str::from_utf8(&a).unwrap(), std::str::from_utf8(&b).unwrap());
}
"""
    stage2 = r"""
fn main() {
    let mut a = *b"0123456789";
    let mut b = *b"0123456789";
    a.copy_within(0..6, 2);
    b.copy_within(3..9, 0);
    println!("{} {}", std::str::from_utf8(&a).unwrap(), std::str::from_utf8(&b).unwrap());
}
"""
    assert lab.same(c, stage1) == "0101234589 3456786789\n"
    lab.same(c, stage2)


def test_a11_memset_zero_struct_is_default(lab: Lab) -> None:
    """A11: `memset(p, 0, sizeof *p)` maps to `ptr::write_bytes` / `*p = T::default()`."""
    c = r"""
#include <stdio.h>
#include <string.h>
typedef struct { int a; double b; char c[4]; unsigned short d; } S;
int main(void) {
    S s = {5, 2.5, {'x', 'y', 'z', 0}, 9};
    printf("%d %.1f %s %u\n", s.a, s.b, s.c, s.d);
    memset(&s, 0, sizeof s);
    printf("%d %.1f %d%d%d%d %u\n", s.a, s.b, s.c[0], s.c[1], s.c[2], s.c[3], s.d);
    return 0;
}
"""
    stage1 = r"""
#[repr(C)] struct S { a: i32, b: f64, c: [u8; 4], d: u16 }
fn main() {
    let mut s = S { a: 5, b: 2.5, c: *b"xyz\0", d: 9 };
    println!("{} {:.1} {} {}", s.a, s.b, std::str::from_utf8(&s.c[..3]).unwrap(), s.d);
    unsafe { core::ptr::write_bytes(&mut s as *mut S, 0, 1); }
    println!("{} {:.1} {}{}{}{} {}", s.a, s.b, s.c[0], s.c[1], s.c[2], s.c[3], s.d);
}
"""
    stage2 = r"""
#[derive(Default)]
struct S { a: i32, b: f64, c: [u8; 4], d: u16 }
fn main() {
    let mut s = S { a: 5, b: 2.5, c: *b"xyz\0", d: 9 };
    println!("{} {:.1} {} {}", s.a, s.b, std::str::from_utf8(&s.c[..3]).unwrap(), s.d);
    s = S::default();
    println!("{} {:.1} {}{}{}{} {}", s.a, s.b, s.c[0], s.c[1], s.c[2], s.c[3], s.d);
}
"""
    assert lab.same(c, stage1) == "5 2.5 xyz 9\n0 0.0 0000 0\n"
    lab.same(c, stage2)
