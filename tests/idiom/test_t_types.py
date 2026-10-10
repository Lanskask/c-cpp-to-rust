"""Idiom rows T*: types."""

from __future__ import annotations

from conftest import Lab


def test_t1_unsigned_int_wraps_like_c(lab: Lab) -> None:
    """T1: `unsigned` is `c_uint`/`u32`; the trap is that `+` panics in a Rust debug build where C wraps."""
    c = '#include <stdio.h>\nint main(void){ unsigned x = 4294967295u; x = x + 1u; printf("%u\\n", x); return 0; }\n'
    ok = 'fn main(){ let x: u32 = u32::MAX; println!("{}", x.wrapping_add(1)); }\n'
    naive = 'fn main(){ let x: u32 = std::hint::black_box(u32::MAX); println!("{}", x + 1); }\n'
    assert lab.same(c, ok) == "0\n"
    lab.differs(c, naive)


def test_t2_size_t_sign_mixing(lab: Lab) -> None:
    """T2: `size_t`/`ssize_t`/`ptrdiff_t` are `usize`/`isize`; trap: `int < size_t` converts int to unsigned."""
    c = r"""
#include <stdio.h>
#include <stddef.h>
#include <sys/types.h>
int main(void){
  size_t n = 3; int i = -1;
  printf("%zu %zu %zu\n", sizeof(size_t), sizeof(ssize_t), sizeof(ptrdiff_t));
  int a[10];
  ptrdiff_t d = &a[7] - &a[2];
  printf("%td\n", d);
  printf("%d\n", i < n);
  ssize_t neg = -5;
  printf("%zd %zu\n", neg, (size_t)neg);
  return 0;
}
"""
    ok = r"""
fn main() {
    let n: usize = 3;
    let i: i32 = -1;
    println!("{} {} {}", std::mem::size_of::<usize>(), std::mem::size_of::<isize>(), std::mem::size_of::<isize>());
    let a = [0i32; 10];
    let d: isize = unsafe { (&a[7] as *const i32).offset_from(&a[2]) };
    println!("{}", d);
    println!("{}", ((i as usize) < n) as i32);
    let neg: isize = -5;
    println!("{} {}", neg, neg as usize);
}
"""
    naive = ok.replace("(i as usize) < n", "(i as isize) < (n as isize)")
    assert lab.same(c, ok) == "8 8 8\n5\n0\n-5 18446744073709551611\n"
    lab.differs(c, naive)


def test_t3_char_is_signed_on_this_platform(lab: Lab) -> None:
    """T3: `char` as a byte is `c_char`; trap: `c >= 0x80` flips under `u8` (char is signed here)."""
    c = r"""
#include <stdio.h>
int main(void){ char c = (char)0xC8; printf("%d %d\n", c >= 0x80, (int)c); return 0; }
"""
    ok = r"""
use std::ffi::c_char;
fn main() {
    let c: c_char = 0xC8u8 as c_char;
    println!("{} {}", (c as i32 >= 0x80) as i32, c as i32);
}
"""
    naive = r"""
fn main() {
    let c: u8 = 0xC8;
    println!("{} {}", (c as i32 >= 0x80) as i32, c as i32);
}
"""
    assert lab.same(c, ok) == "0 -56\n"
    lab.differs(c, naive)


def test_t4_nul_terminated_string_stops_at_nul(lab: Lab) -> None:
    """T4: `char *` is `*const c_char` / `&[u8]`; trap: C stops at the first NUL even when a length is passed."""
    c = r"""
#include <stdio.h>
#include <string.h>
#include <stddef.h>
static size_t scan(const char *s, size_t len){ size_t i = 0; while (i < len && s[i] != '\0') i++; return i; }
int main(void){
  const char buf[5] = {'a','b','\0','c','d'};
  printf("%zu %zu\n", scan(buf, 5), strlen(buf));
  return 0;
}
"""
    ok = r"""
use std::ffi::{c_char, CStr};
fn scan(s: &[u8]) -> usize { s.iter().position(|&b| b == 0).unwrap_or(s.len()) }
fn main() {
    let buf: [u8; 5] = [b'a', b'b', 0, b'c', b'd'];
    let s1 = unsafe { CStr::from_ptr(buf.as_ptr() as *const c_char) }.to_bytes().len();
    println!("{} {}", scan(&buf), s1);
}
"""
    naive = ok.replace(".unwrap_or(s.len())", ".map(|_| s.len()).unwrap_or(s.len())")
    assert lab.same(c, ok) == "2 2\n"
    lab.differs(c, naive)


def test_t5_pointer_length_pair_and_limit_count(lab: Lab) -> None:
    """T5: pointer+length is `&[T]`; trap: a count that is only a limit must not become the slice length (jsmn RP14)."""
    c = r"""
#include <stdio.h>
#include <stddef.h>
static int sum(const int *p, size_t n){ int s = 0; for (size_t i = 0; i < n; i++) s += p[i]; return s; }
static int get(const int *p, size_t limit, size_t i){ (void)limit; return p[i]; }
int main(void){
  int a[6] = {1,2,3,4,5,6};
  printf("%d %d\n", sum(a, 6), get(a, 3, 4));
  return 0;
}
"""
    ok = r"""
fn sum(p: &[i32]) -> i32 { p.iter().sum() }
fn get(p: &[i32], _limit: usize, i: usize) -> i32 { p[i] }
fn main() {
    let a = [1, 2, 3, 4, 5, 6];
    println!("{} {}", sum(&a), get(&a, 3, 4));
}
"""
    naive = ok.replace("get(&a, 3, 4)", "get(&a[..std::hint::black_box(3)], 3, 4)")
    assert lab.same(c, ok) == "21 5\n"
    lab.differs(c, naive)


def test_t6_null_pointer_selects_a_mode(lab: Lab) -> None:
    """T6: nullable pointer is `Option<&mut T>`; trap: NULL selects count-only mode, not an error."""
    c = r"""
#include <stdio.h>
static int collect(int *out, int n){
  int c = 0;
  for (int i = 0; i < n; i++) if (i % 2 == 0) { if (out) out[c] = i * 10; c++; }
  return c;
}
int main(void){
  int buf[8] = {0};
  int cnt = collect(NULL, 7);
  int w = collect(buf, 7);
  printf("%d %d %d %d %d %d\n", cnt, w, buf[0], buf[1], buf[2], buf[3]);
  return 0;
}
"""
    ok = r"""
fn collect(mut out: Option<&mut [i32]>, n: i32) -> usize {
    let mut c = 0;
    for i in 0..n {
        if i % 2 == 0 {
            if let Some(o) = out.as_deref_mut() { o[c] = i * 10; }
            c += 1;
        }
    }
    c
}
fn main() {
    let mut buf = [0i32; 8];
    let cnt = collect(None, 7);
    let w = collect(Some(&mut buf), 7);
    println!("{} {} {} {} {} {}", cnt, w, buf[0], buf[1], buf[2], buf[3]);
}
"""
    naive = ok.replace("let mut c = 0;", "let mut c = 0;\n    if out.is_none() { return 0; }")
    assert lab.same(c, ok) == "4 4 0 20 40 60\n"
    lab.differs(c, naive)


def test_t7_truthy_int_is_not_a_bool(lab: Lab) -> None:
    """T7: `_Bool`/boolean int is `bool`; trap: `if (x)` on a non-0/1 int is true for any non-zero value."""
    c = r"""
#include <stdio.h>
int main(void){
  int flags = 4;
  if (flags) puts("yes"); else puts("no");
  _Bool b = 256;
  printf("%d %d\n", (int)b, flags);
  return 0;
}
"""
    ok = r"""
fn main() {
    let flags: i32 = 4;
    if flags != 0 { println!("yes"); } else { println!("no"); }
    let b: bool = std::hint::black_box(256) != 0;
    println!("{} {}", b as i32, flags);
}
"""
    naive = ok.replace("flags != 0", "flags == 1")
    assert lab.same(c, ok) == "yes\n1 4\n"
    lab.differs(c, naive)


def test_t8_enum_holds_unknown_value(lab: Lab) -> None:
    """T8: `enum` is `#[repr(i32)] enum` + `TryFrom<i32>`; trap: a C enum variable may hold any integer."""
    c = r"""
#include <stdio.h>
enum E { A = 1, B = 2, C = 40 };
static const char *name(int v){
  switch (v) { case A: return "A"; case B: return "B"; case C: return "C"; default: return "?"; }
}
int main(void){
  enum E e = (enum E)7;
  printf("%d %s %s %d\n", (int)e, name(e), name(B), (int)C);
  return 0;
}
"""
    ok = r"""
use std::ffi::c_uint;
#[repr(i32)]
#[derive(Clone, Copy)]
enum E { A = 1, B = 2, C = 40 }
impl TryFrom<i32> for E {
    type Error = i32;
    fn try_from(v: i32) -> Result<E, i32> {
        match v { 1 => Ok(E::A), 2 => Ok(E::B), 40 => Ok(E::C), _ => Err(v) }
    }
}
fn name(v: i32) -> &'static str {
    match E::try_from(v) { Ok(E::A) => "A", Ok(E::B) => "B", Ok(E::C) => "C", Err(_) => "?" }
}
type Ec = c_uint;
const EA: Ec = 1;
fn name1(v: Ec) -> &'static str { if v == EA { "A" } else { "?" } }
fn main() {
    let e: i32 = 7;
    assert_eq!(name1(7), "?");
    println!("{} {} {} {}", e, name(e), name(E::B as i32), E::C as i32);
}
"""
    naive = ok.replace('Err(_) => "?"', 'Err(_) => "B"')
    assert lab.same(c, ok) == "7 ? B 40\n"
    lab.differs(c, naive)


def test_t9_struct_layout_repr_c_vs_plain(lab: Lab) -> None:
    """T9: `struct` is `#[repr(C)]` in Stage 1; trap: layout differs without `repr(C)` (Stage 2 may reorder)."""
    c = r"""
#include <stdio.h>
#include <stddef.h>
struct S { char a; int b; char c; };
int main(void){
  struct S s = {0};
  printf("%zu %zu %zu %zu %d %d %d\n", sizeof(struct S), offsetof(struct S, a), offsetof(struct S, b),
         offsetof(struct S, c), s.a, s.b, s.c);
  return 0;
}
"""
    ok = r"""
use std::mem::{offset_of, size_of};
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct S { a: i8, b: i32, c: i8 }
fn main() {
    let s = S::default();
    let (oa, ob, oc) = (offset_of!(S, a), offset_of!(S, b), offset_of!(S, c));
    println!("{} {} {} {} {} {} {}", size_of::<S>(), oa, ob, oc, s.a, s.b, s.c);
}
"""
    naive = ok.replace("#[repr(C)]\n", "")
    assert lab.same(c, ok) == "12 0 4 8 0 0 0\n"
    lab.differs(c, naive)


def test_t10_union_punning_and_endianness(lab: Lab) -> None:
    """T10: `union` is `#[repr(C)] union` or `to_le_bytes`; trap: punning reads platform byte order (pick one)."""
    c = r"""
#include <stdio.h>
#include <stdint.h>
union U { float f; uint32_t u; unsigned char b[4]; };
int main(void){
  union U x;
  x.f = 1.0f;
  printf("%08x %u %u %u %u\n", x.u, x.b[0], x.b[1], x.b[2], x.b[3]);
  return 0;
}
"""
    stage1 = r"""
#[repr(C)]
#[derive(Copy, Clone)]
union U { f: f32, u: u32, b: [u8; 4] }
fn main() {
    let x = U { f: 1.0 };
    let (u, b) = unsafe { (x.u, x.b) };
    println!("{:08x} {} {} {} {}", u, b[0], b[1], b[2], b[3]);
}
"""
    stage2 = r"""
fn main() {
    let b = 1.0f32.to_le_bytes();
    let u = u32::from_le_bytes(b);
    println!("{:08x} {} {} {} {}", u, b[0], b[1], b[2], b[3]);
}
"""
    naive = stage2.replace("to_le_bytes", "to_be_bytes")
    assert lab.same(c, stage1) == "3f800000 0 0 128 63\n"
    assert lab.same(c, stage2) == "3f800000 0 0 128 63\n"
    lab.differs(c, naive)


def test_t10_tagged_union_becomes_enum(lab: Lab) -> None:
    """T10: `union` with a tag field is an `enum` in Stage 2."""
    c = r"""
#include <stdio.h>
struct Val { int tag; union { int i; double d; } v; };
static void show(struct Val x){ if (x.tag == 0) printf("i%d\n", x.v.i); else printf("d%.2f\n", x.v.d); }
int main(void){
  struct Val a; a.tag = 0; a.v.i = 42;
  struct Val b; b.tag = 1; b.v.d = 2.5;
  show(a); show(b);
  return 0;
}
"""
    ok = r"""
enum Val { I(i32), D(f64) }
fn show(x: Val) { match x { Val::I(i) => println!("i{}", i), Val::D(d) => println!("d{:.2}", d) } }
fn main() { show(Val::I(42)); show(Val::D(2.5)); }
"""
    assert lab.same(c, ok) == "i42\nd2.50\n"


def test_t11_bitfields_manual_masks(lab: Lab) -> None:
    """T11: bitfields are manual masks; trap: bit order/storage unit are implementation-defined (LSB-first here)."""
    c = r"""
#include <stdio.h>
#include <string.h>
struct B { unsigned a:3; unsigned b:5; };
int main(void){
  struct B s;
  memset(&s, 0, sizeof s);
  s.a = 5; s.b = 17;
  unsigned char raw;
  memcpy(&raw, &s, 1);
  s.a = 9;
  printf("%u %u %u %zu\n", raw, s.a, s.b, sizeof s);
  return 0;
}
"""
    ok = r"""
#[derive(Copy, Clone, Default)]
struct B(u32);
impl B {
    fn set_a(&mut self, v: u32) { self.0 = (self.0 & !0x7) | (v & 0x7); }
    fn set_b(&mut self, v: u32) { self.0 = (self.0 & !(0x1F << 3)) | ((v & 0x1F) << 3); }
    fn a(&self) -> u32 { self.0 & 0x7 }
    fn b(&self) -> u32 { (self.0 >> 3) & 0x1F }
}
fn main() {
    let mut s = B::default();
    s.set_a(5);
    s.set_b(17);
    let raw = (s.0 & 0xFF) as u8;
    s.set_a(9);
    println!("{} {} {} {}", raw, s.a(), s.b(), std::mem::size_of::<B>());
}
"""
    naive = ok.replace("(self.0 & 0x7)", "((self.0 >> 5) & 0x7)").replace(
        "let raw = (s.0 & 0xFF) as u8;", "let raw = ((s.a() << 5) | s.b()) as u8;"
    )
    assert lab.same(c, ok) == "141 1 17 4\n"
    lab.differs(c, naive)


def test_t12_null_function_pointer_means_default(lab: Lab) -> None:
    """T12: function pointer is `Option<unsafe extern "C" fn>`; trap: NULL means default behaviour."""
    c = r"""
#include <stdio.h>
static int dbl(int x){ return x * 2; }
static int apply(int x, int (*f)(int)){ if (!f) return x; return f(x); }
int main(void){ printf("%d %d\n", apply(5, dbl), apply(5, NULL)); return 0; }
"""
    ok = r"""
unsafe extern "C" fn dbl(x: i32) -> i32 { x * 2 }
fn apply(x: i32, f: Option<unsafe extern "C" fn(i32) -> i32>) -> i32 {
    match f { Some(f) => unsafe { f(x) }, None => x }
}
fn apply2<F: Fn(i32) -> i32>(x: i32, f: Option<F>) -> i32 { f.map_or(x, |f| f(x)) }
fn main() {
    assert_eq!(apply2(5, Some(|v| v * 2)), 10);
    assert_eq!(apply2::<fn(i32) -> i32>(5, None), 5);
    println!("{} {}", apply(5, Some(dbl)), apply(5, None));
}
"""
    naive = ok.replace("None => x }", "None => 0 }")
    assert lab.same(c, ok) == "10 5\n"
    lab.differs(c, naive)


def test_t13_void_pointer_user_data_to_closure(lab: Lab) -> None:
    """T13: `void *` user data is `*mut c_void` in Stage 1 and a capturing closure in Stage 2."""
    c = r"""
#include <stdio.h>
struct Acc { int sum; int count; };
static void visit(int n, void (*cb)(int, void *), void *ud){ for (int i = 1; i <= n; i++) cb(i, ud); }
static void add(int v, void *ud){ struct Acc *a = ud; a->sum += v; a->count++; }
int main(void){
  struct Acc a = {0, 0};
  visit(5, add, &a);
  printf("%d %d\n", a.sum, a.count);
  return 0;
}
"""
    ok = r"""
use std::ffi::c_void;
struct Acc { sum: i32, count: i32 }
unsafe extern "C" fn add(v: i32, ud: *mut c_void) {
    let a = unsafe { &mut *(ud as *mut Acc) };
    a.sum += v;
    a.count += 1;
}
fn visit1(n: i32, cb: unsafe extern "C" fn(i32, *mut c_void), ud: *mut c_void) {
    for i in 1..=n { unsafe { cb(i, ud) } }
}
fn visit2<F: FnMut(i32)>(n: i32, mut cb: F) { for i in 1..=n { cb(i); } }
fn main() {
    let mut a = Acc { sum: 0, count: 0 };
    visit1(5, add, &mut a as *mut Acc as *mut c_void);
    let (mut s, mut n) = (0, 0);
    visit2(5, |v| { s += v; n += 1; });
    assert_eq!((s, n), (a.sum, a.count));
    println!("{} {}", a.sum, a.count);
}
"""
    assert lab.same(c, ok) == "15 5\n"


def test_t14_flexible_array_member_to_vec(lab: Lab) -> None:
    """T14: flexible array member is raw pointer arithmetic (Stage 1) and `Vec<T>` (Stage 2)."""
    c = r"""
#include <stdio.h>
#include <stdlib.h>
#include <stddef.h>
struct V { size_t n; int d[]; };
int main(void){
  struct V *v = malloc(sizeof *v + 5 * sizeof(int));
  v->n = 5;
  int sum = 0;
  for (size_t i = 0; i < v->n; i++) { v->d[i] = (int)(i * i); sum += v->d[i]; }
  printf("%zu %d %zu\n", v->n, sum, sizeof(struct V));
  free(v);
  return 0;
}
"""
    ok = r"""
use std::ffi::c_void;
extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}
#[repr(C)]
struct V { n: usize, d: [i32; 0] }
fn main() {
    let (n1, s1) = unsafe {
        let v = malloc(std::mem::size_of::<V>() + 5 * 4) as *mut V;
        (*v).n = 5;
        let d = (*v).d.as_mut_ptr();
        let mut sum = 0;
        for i in 0..(*v).n { *d.add(i) = (i * i) as i32; sum += *d.add(i); }
        let n = (*v).n;
        free(v as *mut c_void);
        (n, sum)
    };
    let d: Vec<i32> = (0..5).map(|i| i * i).collect();
    assert_eq!((d.len(), d.iter().sum::<i32>()), (n1, s1));
    println!("{} {} {}", n1, s1, std::mem::size_of::<V>());
}
"""
    assert lab.same(c, ok) == "5 30 8\n"


def test_t15_out_parameter_partial_value_on_error(lab: Lab) -> None:
    """T15: out-parameter is a return value/tuple; trap: C writes `*out` and also returns an error code (partial)."""
    c = r"""
#include <stdio.h>
static int parse_digits(const char *s, int *out){
  *out = 0;
  for (; *s; s++) {
    if (*s < '0' || *s > '9') return -1;
    *out = *out * 10 + (*s - '0');
  }
  return 0;
}
int main(void){
  int v = -99;
  int rc = parse_digits("123x5", &v);
  int v2 = -99;
  int rc2 = parse_digits("987", &v2);
  printf("%d %d %d %d\n", rc, v, rc2, v2);
  return 0;
}
"""
    ok = r"""
fn parse_digits(s: &[u8]) -> (i32, i32) {
    let mut out = 0;
    for &b in s {
        if !b.is_ascii_digit() { return (-1, out); }
        out = out * 10 + (b - b'0') as i32;
    }
    (0, out)
}
fn parse_digits_out(s: &[u8], out: *mut i32) -> i32 {
    let (rc, v) = parse_digits(s);
    unsafe { *out = v; }
    rc
}
fn main() {
    let (rc, v) = parse_digits(b"123x5");
    let mut v2 = -99;
    let rc2 = parse_digits_out(b"987", &mut v2);
    println!("{} {} {} {}", rc, v, rc2, v2);
}
"""
    naive = ok.replace("return (-1, out);", "return (-1, 0);")
    assert lab.same(c, ok) == "-1 123 0 987\n"
    lab.differs(c, naive)
