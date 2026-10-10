"""Idiom rows G* (globals and state) and M* (macros and preprocessor)."""

from __future__ import annotations

from conftest import Lab

# ---------------------------------------------------------------- G


G1_C = r"""
#include <stdio.h>
static int counter = 0;
static int log_[8];
static int log_n = 0;
static void record(int v) { counter++; log_[log_n++] = v; }
int main(void) {
    record(10); record(20); record(30);
    printf("%d %d %d\n", counter, log_n, log_[1]);
    return 0;
}
"""


def test_g1_global_static_mut_stage1(lab: Lab) -> None:
    """G1 (Stage 1): file-level mutable global is `static mut`."""
    ok = r"""
static mut COUNTER: i32 = 0;
static mut LOG: [i32; 8] = [0; 8];
static mut LOG_N: usize = 0;
fn record(v: i32) { unsafe { COUNTER += 1; LOG[LOG_N] = v; LOG_N += 1; } }
fn main() {
    record(10); record(20); record(30);
    unsafe { println!("{} {} {}", COUNTER, LOG_N, LOG[1]); }
}
"""
    assert lab.same(G1_C, ok) == "3 3 20\n"


def test_g1_global_atomic_oncelock_stage2(lab: Lab) -> None:
    """G1 (Stage 2): `AtomicI32` for the counter, `OnceLock<Mutex<Vec>>` for the table; no `unsafe`."""
    ok = r"""
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
static COUNTER: AtomicI32 = AtomicI32::new(0);
static LOG: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();
fn record(v: i32) {
    COUNTER.fetch_add(1, Ordering::SeqCst);
    LOG.get_or_init(|| Mutex::new(Vec::new())).lock().unwrap().push(v);
}
fn main() {
    record(10); record(20); record(30);
    let log = LOG.get().unwrap().lock().unwrap();
    println!("{} {} {}", COUNTER.load(Ordering::SeqCst), log.len(), log[1]);
}
"""
    assert lab.same(G1_C, ok) == "3 3 20\n"


def test_g1_signal_handler_global_is_atomic(lab: Lab) -> None:
    """G1: a signal-handler-reachable global maps to an atomic (only const/static/atomic are safe there)."""
    c = r"""
#include <signal.h>
#include <stdio.h>
static volatile sig_atomic_t flag = 0;
static void h(int s) { flag = s; }
int main(void) {
    signal(SIGINT, h);
    printf("%d\n", (int)flag);
    raise(SIGINT);
    printf("%d\n", (int)flag);
    return 0;
}
"""
    ok = r"""
use std::sync::atomic::{AtomicI32, Ordering};
static FLAG: AtomicI32 = AtomicI32::new(0);
extern "C" fn h(s: i32) { FLAG.store(s, Ordering::SeqCst); }
extern "C" {
    fn signal(sig: i32, handler: extern "C" fn(i32)) -> usize;
    fn raise(sig: i32) -> i32;
}
fn main() {
    unsafe { signal(2, h); }
    println!("{}", FLAG.load(Ordering::SeqCst));
    unsafe { raise(2); }
    println!("{}", FLAG.load(Ordering::SeqCst));
}
"""
    assert lab.same(c, ok) == "0\n2\n"


def test_g2_function_local_static(lab: Lab) -> None:
    """G2: function-local `static` becomes module-level `static mut` (Stage 1) or a field (Stage 2); init runs once."""
    c = r"""
#include <stdio.h>
static int next_id(void) { static int n = 100; return ++n; }
static int next_even(void) { static int n = 0; n += 2; return n; }
int main(void) {
    printf("%d %d %d\n", next_id(), next_id(), next_id());
    printf("%d %d\n", next_even(), next_even());
    return 0;
}
"""
    stage1 = r"""
static mut NEXT_ID_N: i32 = 100;
static mut NEXT_EVEN_N: i32 = 0;
fn next_id() -> i32 { unsafe { NEXT_ID_N += 1; NEXT_ID_N } }
fn next_even() -> i32 { unsafe { NEXT_EVEN_N += 2; NEXT_EVEN_N } }
fn main() {
    let (a, b, c) = (next_id(), next_id(), next_id());
    println!("{} {} {}", a, b, c);
    let (d, e) = (next_even(), next_even());
    println!("{} {}", d, e);
}
"""
    stage2 = r"""
struct Ids { next_id: i32, next_even: i32 }
impl Ids {
    fn new() -> Ids { Ids { next_id: 100, next_even: 0 } }
    fn next_id(&mut self) -> i32 { self.next_id += 1; self.next_id }
    fn next_even(&mut self) -> i32 { self.next_even += 2; self.next_even }
}
fn main() {
    let mut ids = Ids::new();
    let (a, b, c) = (ids.next_id(), ids.next_id(), ids.next_id());
    println!("{} {} {}", a, b, c);
    let (d, e) = (ids.next_even(), ids.next_even());
    println!("{} {}", d, e);
}
"""
    naive = stage2.replace(
        "let mut ids = Ids::new();\n    let (a, b, c) = (ids.next_id(), ids.next_id(), ids.next_id());",
        "let (a, b, c) = (Ids::new().next_id(), Ids::new().next_id(), Ids::new().next_id());\n"
        "    let mut ids = Ids::new();",
    )
    assert lab.same(c, stage1) == "101 102 103\n2 4\n"
    assert lab.same(c, stage2) == "101 102 103\n2 4\n"
    # trap-ish: re-initialising the "static" on every call (a plain local) resets the sequence
    lab.differs(c, naive)


G3_C = r"""
#include <stdio.h>
static int err_pos = -1;
static int parse(const char *s) {
    for (int i = 0; s[i]; i++)
        if (s[i] == '!') { err_pos = i; return -1; }
    return 0;
}
static int get_error_pos(void) { return err_pos; }
int main(void) {
    int rc = parse("ab!d");
    printf("%d %d\n", rc, get_error_pos());
    rc = parse("abcd");
    printf("%d %d\n", rc, get_error_pos());
    return 0;
}
"""


def test_g3_global_error_state_thread_local_cell(lab: Lab) -> None:
    """G3: global error pointer (`cJSON_GetErrorPtr`) kept as a thread-local `Cell`; stale value survives a success."""
    ok = r"""
use std::cell::Cell;
thread_local! { static ERR_POS: Cell<i32> = Cell::new(-1); }
fn parse(s: &str) -> i32 {
    for (i, b) in s.bytes().enumerate() {
        if b == b'!' { ERR_POS.with(|e| e.set(i as i32)); return -1; }
    }
    0
}
fn get_error_pos() -> i32 { ERR_POS.with(|e| e.get()) }
fn main() {
    let mut rc = parse("ab!d");
    println!("{} {}", rc, get_error_pos());
    rc = parse("abcd");
    println!("{} {}", rc, get_error_pos());
}
"""
    assert lab.same(G3_C, ok) == "-1 2\n0 2\n"


def test_g3_error_in_err_loses_stale_state(lab: Lab) -> None:
    """G3: returning the position in `Err` is a different API: C's `get_error_pos` after a success is still stale."""
    naive = r"""
fn parse(s: &str) -> Result<(), usize> {
    for (i, b) in s.bytes().enumerate() { if b == b'!' { return Err(i); } }
    Ok(())
}
fn main() {
    // an `Err`-returning API has no stale slot: after a success there is no position to read
    for s in ["ab!d", "abcd"] {
        match parse(s) {
            Err(p) => println!("-1 {}", p),
            Ok(()) => println!("0 -1"),
        }
    }
}
"""
    lab.differs(G3_C, naive)


# ---------------------------------------------------------------- M


def test_m1_object_macro_keeps_promoted_type(lab: Lab) -> None:
    """M1: `#define N 42` is a typed `const`; trap: `0xFFFFFFFF` is `unsigned`, `4294967296` is `long`."""
    c = r"""
#include <stdio.h>
#define N 42
#define MASK 0xFFFFFFFF
#define BIG 4294967296
int main(void) {
    printf("%d\n", N * 2);
    printf("%d %u %d\n", MASK > 0, MASK / 2, (int)sizeof(MASK));
    printf("%ld %d\n", (long)(BIG / 2), (int)sizeof(BIG));
    return 0;
}
"""
    head = "pub const N: i32 = 42;\n"
    ok = (
        head
        + r"""
pub const MASK: u32 = 0xFFFF_FFFF;
pub const BIG: i64 = 4_294_967_296;
fn main() {
    println!("{}", N * 2);
    println!("{} {} {}", (MASK > 0) as i32, MASK / 2, std::mem::size_of_val(&MASK));
    println!("{} {}", BIG / 2, std::mem::size_of_val(&BIG));
}
"""
    )
    naive = ok.replace("pub const MASK: u32 = 0xFFFF_FFFF;", "pub const MASK: i32 = 0xFFFF_FFFFu32 as i32;").replace(
        "(MASK > 0) as i32, MASK / 2", "(MASK > 0) as i32, MASK as u32 / 2"
    )
    assert lab.same(c, ok) == "84\n1 2147483647 4\n2147483648 8\n"
    lab.differs(c, naive)


M2_C = r"""
#include <stdio.h>
#define MAX(a, b) ((a) > (b) ? (a) : (b))
int main(void) {
    int i = 5, j = 3;
    int m = MAX(i++, j);
    printf("%d %d\n", m, i);
    int k = 1, l = 2;
    printf("%d\n", MAX(k + 1, l));
    return 0;
}
"""


def test_m2_function_macro_double_evaluation(lab: Lab) -> None:
    """M2: function-like macro becomes `fn`; trap: `MAX(i++, j)` evaluates `i++` twice, a `fn` evaluates once."""
    as_fn = r"""
#[inline]
fn max(a: i32, b: i32) -> i32 { if a > b { a } else { b } }
fn main() {
    let (mut i, j) = (5, 3);
    let m = max({ let t = i; i += 1; t }, j);
    println!("{} {}", m, i);
    let (k, l) = (1, 2);
    println!("{}", max(k + 1, l));
}
"""
    as_macro = r"""
macro_rules! max { ($a:expr, $b:expr) => { if $a > $b { $a } else { $b } }; }
fn main() {
    let (mut i, j) = (5, 3);
    let m = max!({ let t = i; i += 1; t }, j);
    println!("{} {}", m, i);
    let (k, l) = (1, 2);
    println!("{}", max!(k + 1, l));
}
"""
    assert lab.same(M2_C, as_macro) == "6 7\n2\n"
    lab.differs(M2_C, as_fn)
    # side-effect-free arguments: the `fn` mapping is exact
    c2 = (
        "#include <stdio.h>\n#define MAX(a,b) ((a)>(b)?(a):(b))\n"
        'int main(void){ int k=1,l=2; printf("%d\\n", MAX(k+1,l)); return 0; }\n'
    )
    r2 = 'fn max(a:i32,b:i32)->i32{ if a>b {a} else {b} }\nfn main(){ let (k,l)=(1,2); println!("{}", max(k+1,l)); }\n'
    assert lab.same(c2, r2) == "2\n"


def test_m3_configuration_macro_two_configs(lab: Lab) -> None:
    """M3: `#ifdef FEATURE` is a cargo feature cfg; trap: one run = one configuration, both must be run."""
    c = r"""
#include <stdio.h>
struct S {
    int a;
#ifdef FEATURE
    double extra;
#endif
};
static int f(int x) {
#ifdef FEATURE
    return x * 2;
#else
    return x + 1;
#endif
}
int main(void) {
    printf("%d %d\n", f(10), (int)sizeof(struct S));
    return 0;
}
"""
    ok = r"""
#[repr(C)]
#[allow(dead_code)]
struct S {
    a: i32,
    #[cfg(feature = "x")]
    extra: f64,
}
#[cfg(feature = "x")]
fn f(x: i32) -> i32 { x * 2 }
#[cfg(not(feature = "x"))]
fn f(x: i32) -> i32 { x + 1 }
fn main() { println!("{} {}", f(10), std::mem::size_of::<S>()); }
"""
    off_c, on_c = ("-std=c11",), ("-std=c11", "-DFEATURE")
    on_r = ("--cfg", 'feature="x"')
    off = lab.c(c, off_c).out
    on = lab.c(c, on_c).out
    assert off == lab.rust(ok).out == "11 4\n"
    assert on == lab.rust(ok, on_r).out == "20 16\n"
    assert off != on  # the configurations really differ, so one run cannot cover both


def test_m5_platform_macro_target_os(lab: Lab) -> None:
    """M5: `__APPLE__`/`_WIN32` become `#[cfg(target_os = ...)]`; trap: a misspelt os name silently drops the branch."""
    c = r"""
#include <stdio.h>
int main(void) {
#ifdef __APPLE__
    printf("apple\n");
#elif defined(_WIN32)
    printf("windows\n");
#else
    printf("other\n");
#endif
    return 0;
}
"""
    ok = r"""
#[cfg(target_os = "macos")]
fn name() -> &'static str { "apple" }
#[cfg(target_os = "windows")]
fn name() -> &'static str { "windows" }
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn name() -> &'static str { "other" }
fn main() { println!("{}", name()); }
"""
    naive = ok.replace('target_os = "macos"', 'target_os = "apple"')
    assert lab.same(c, ok) == "apple\n"
    lab.differs(c, naive)
