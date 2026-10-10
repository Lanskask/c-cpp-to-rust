"""Idiom rows C*: control flow."""

from __future__ import annotations

from conftest import Lab

C1_C = r"""
#include <stdio.h>
static void find(int grid[3][3], int want) {
    int r, c;
    for (r = 0; r < 3; r++)
        for (c = 0; c < 3; c++)
            if (grid[r][c] == want) goto found;
    printf("not found\n");
    return;
found:
    printf("found at %d,%d\n", r, c);
}
int main(void) {
    int grid[3][3] = {{1, 2, 3}, {4, 2, 6}, {7, 8, 2}};
    find(grid, 2);
    find(grid, 5);
    return 0;
}
"""


def test_c1_goto_found_labelled_block(lab: Lab) -> None:
    """C1: forward `goto found` out of nested loops is a labelled block; trap: a plain `break` leaves one loop only."""
    ok = r"""
fn find(grid: &[[i32; 3]; 3], want: i32) {
    let (mut r, mut c) = (0, 0);
    let hit = 'found: {
        for rr in 0..3 {
            for cc in 0..3 {
                if grid[rr][cc] == want { r = rr; c = cc; break 'found true; }
            }
        }
        false
    };
    if hit { println!("found at {},{}", r, c); } else { println!("not found"); }
}
fn main() {
    let grid = [[1, 2, 3], [4, 2, 6], [7, 8, 2]];
    find(&grid, 2);
    find(&grid, 5);
}
"""
    naive = r"""
fn find(grid: &[[i32; 3]; 3], want: i32) {
    let (mut r, mut c, mut hit) = (0, 0, false);
    for rr in 0..3 {
        for cc in 0..3 {
            if grid[rr][cc] == want { r = rr; c = cc; hit = true; break; }
        }
    }
    if hit { println!("found at {},{}", r, c); } else { println!("not found"); }
}
fn main() {
    let grid = [[1, 2, 3], [4, 2, 6], [7, 8, 2]];
    find(&grid, 2);
    find(&grid, 5);
}
"""
    assert lab.same(C1_C, ok) == "found at 0,1\nnot found\n"
    lab.differs(C1_C, naive)


def test_c1_goto_found_early_return(lab: Lab) -> None:
    """C1 (Stage 2): the same search written as an early `return` with `Option`."""
    ok = r"""
fn find(grid: &[[i32; 3]; 3], want: i32) -> Option<(usize, usize)> {
    for (r, row) in grid.iter().enumerate() {
        for (c, &v) in row.iter().enumerate() {
            if v == want { return Some((r, c)); }
        }
    }
    None
}
fn main() {
    let grid = [[1, 2, 3], [4, 2, 6], [7, 8, 2]];
    for want in [2, 5] {
        match find(&grid, want) {
            Some((r, c)) => println!("found at {},{}", r, c),
            None => println!("not found"),
        }
    }
}
"""
    lab.same(C1_C, ok)


def test_c2_goto_cleanup_raii_order(lab: Lab) -> None:
    """C2: `goto cleanup` becomes RAII + early return; trap: cleanup order is observable (tuple drops a then b)."""
    c = r"""
#include <stdio.h>
static int process(int fail) {
    int rc = -1;
    printf("open a\n");
    printf("open b\n");
    if (fail) goto cleanup;
    printf("work\n");
    rc = 0;
cleanup:
    printf("close b\n");
    printf("close a\n");
    return rc;
}
int main(void) {
    printf("rc=%d\n", process(0));
    printf("rc=%d\n", process(1));
    return 0;
}
"""
    ok = r"""
struct Res(&'static str);
impl Res { fn open(n: &'static str) -> Res { println!("open {}", n); Res(n) } }
impl Drop for Res { fn drop(&mut self) { println!("close {}", self.0); } }
fn process(fail: bool) -> Result<(), i32> {
    let _a = Res::open("a");
    let _b = Res::open("b");
    if fail { return Err(-1); }
    println!("work");
    Ok(())
}
fn main() {
    println!("rc={}", process(false).map_or_else(|e| e, |_| 0));
    println!("rc={}", process(true).map_or_else(|e| e, |_| 0));
}
"""
    naive = ok.replace(
        'let _a = Res::open("a");\n    let _b = Res::open("b");',
        'let _both = (Res::open("a"), Res::open("b"));',
    )
    assert lab.same(c, ok).startswith("open a\nopen b\nwork\nclose b\nclose a\nrc=0\n")
    lab.differs(c, naive)


def test_c3_goto_retry_loop_continue(lab: Lab) -> None:
    """C3: backward `goto retry` becomes `loop { .. continue; }`."""
    c = r"""
#include <stdio.h>
int main(void) {
    int attempts = 0, v;
retry:
    attempts++;
    v = attempts * attempts;
    if (v < 20) goto retry;
    printf("%d %d\n", attempts, v);
    return 0;
}
"""
    ok = r"""
fn main() {
    let mut attempts = 0;
    let v = loop {
        attempts += 1;
        let v = attempts * attempts;
        if v < 20 { continue; }
        break v;
    };
    println!("{} {}", attempts, v);
}
"""
    assert lab.same(c, ok) == "5 25\n"


def test_c4_switch_fallthrough_break_in_loop(lab: Lab) -> None:
    """C4: `switch` fallthrough is combined arms; trap: `break` in a switch leaves the switch, not the loop."""
    c = r"""
#include <stdio.h>
int main(void) {
    int s = 0;
    for (int i = 0; i < 6; i++) {
        switch (i) {
        case 0: case 1: s += 1; break;
        case 2: s += 10; /* fallthrough */
        case 3: s += 100; break;
        case 4: s += 1000; break;
        default: s += 5;
        }
        s += 7;
    }
    printf("%d\n", s);
    return 0;
}
"""
    ok = r"""
fn main() {
    let mut s = 0;
    for i in 0..6 {
        match i {
            0 | 1 => s += 1,
            2 => { s += 10; s += 100; }
            3 => s += 100,
            4 => s += 1000,
            _ => s += 5,
        }
        s += 7;
    }
    println!("{}", s);
}
"""
    naive = ok.replace("4 => s += 1000,", "4 => { s += 1000; break; }")
    assert lab.same(c, ok) == "1259\n"
    lab.differs(c, naive)


C5_C = r"""
#include <stdio.h>
int main(void) {
    int a[] = {3, 7, 7, 2, 9};
    int n = 5, i, j, k;
    for (i = n - 1; i >= 0; i--) if (a[i] == 7) break;
    printf("last %d\n", i);
    for (j = i - 1; j >= 0; j--) if (a[j] == 7) break;
    printf("prev %d\n", j);
    for (k = j - 1; k >= 0; k--) if (a[k] == 7) break;
    printf("none %d\n", k);
    return 0;
}
"""


def test_c5_backward_loop_signed_stage1(lab: Lab) -> None:
    """C5 (Stage 1): `while i >= 0` with a signed `c_int`; exit value -1 and resumed scans keep the same `i`."""
    ok = r"""
fn main() {
    let a = [3i32, 7, 7, 2, 9];
    let n = 5i32;
    let mut i = n - 1;
    while i >= 0 { if a[i as usize] == 7 { break; } i -= 1; }
    println!("last {}", i);
    let mut j = i - 1;
    while j >= 0 { if a[j as usize] == 7 { break; } j -= 1; }
    println!("prev {}", j);
    let mut k = j - 1;
    while k >= 0 { if a[k as usize] == 7 { break; } k -= 1; }
    println!("none {}", k);
}
"""
    assert lab.same(C5_C, ok) == "last 2\nprev 1\nnone -1\n"


def test_c5_backward_loop_rposition_stage2(lab: Lab) -> None:
    """C5 (Stage 2): `rposition` (None = -1) and `slice[..i].iter().rposition` for the resumed scan."""
    ok = r"""
fn main() {
    let a = [3i32, 7, 7, 2, 9];
    let last = a.iter().rposition(|&x| x == 7);
    println!("last {}", last.map_or(-1, |v| v as i32));
    let i = last.unwrap();
    let prev = a[..i].iter().rposition(|&x| x == 7);
    println!("prev {}", prev.map_or(-1, |v| v as i32));
    let j = prev.unwrap();
    let none = a[..j].iter().rposition(|&x| x == 7);
    println!("none {}", none.map_or(-1, |v| v as i32));
}
"""
    assert lab.same(C5_C, ok) == "last 2\nprev 1\nnone -1\n"


C5_UNSIGNED_C = r"""
#include <stdio.h>
int main(void) {
    unsigned n = 2, i;
    int steps = 0;
    for (i = n - 1; i >= 0 && steps < 4; i--) { printf("i=%u\n", i); steps++; }
    return 0;
}
"""


def test_c5_unsigned_index_never_ends(lab: Lab) -> None:
    """C5 trap: with an unsigned index `i >= 0` is always true (wraps to UINT_MAX); `usize` `i -= 1` panics at 0."""
    ok = r"""
fn main() {
    let n: u32 = 2;
    let mut i = n - 1;
    let mut steps = 0;
    while steps < 4 { println!("i={}", i); steps += 1; i = i.wrapping_sub(1); }
}
"""
    naive = ok.replace("i = i.wrapping_sub(1);", "i -= 1;")
    assert lab.same(C5_UNSIGNED_C, ok) == "i=1\ni=0\ni=4294967295\ni=4294967294\n"
    lab.differs(C5_UNSIGNED_C, naive)


def test_c6_setjmp_longjmp_result(lab: Lab) -> None:
    """C6: `setjmp`/`longjmp` as error escape becomes `Result` + `?` (Stage 2); Stage 1 is an unknown tier."""
    c = r"""
#include <setjmp.h>
#include <stdio.h>
static jmp_buf env;
static void deep(int n) {
    if (n == 3) longjmp(env, 7);
    printf("deep %d\n", n);
    deep(n + 1);
}
int main(void) {
    int r = setjmp(env);
    if (r == 0) { deep(0); printf("unreachable\n"); }
    else printf("error %d\n", r);
    return 0;
}
"""
    ok = r"""
fn deep(n: i32) -> Result<(), i32> {
    if n == 3 { return Err(7); }
    println!("deep {}", n);
    deep(n + 1)?;
    Ok(())
}
fn main() {
    match deep(0) {
        Ok(()) => println!("unreachable"),
        Err(r) => println!("error {}", r),
    }
}
"""
    assert lab.same(c, ok) == "deep 0\ndeep 1\ndeep 2\nerror 7\n"


C7_C = r"""
#include <assert.h>
#include <stdio.h>
int main(void) {
    int x = 1;
    assert(x++ > 0);
    printf("%d\n", x);
    return 0;
}
"""
C7_RUST = r"""
fn main() {
    let mut x = 1;
    debug_assert!({ let ok = x > 0; x += 1; ok });
    println!("{}", x);
}
"""


def test_c7_assert_with_side_effect_ndebug(lab: Lab) -> None:
    """C7: `assert` maps to `debug_assert!` (NDEBUG = no debug-assertions); trap: side effect vanishes under NDEBUG."""
    c_on, c_off = ("-std=c11",), ("-std=c11", "-DNDEBUG")
    r_on, r_off = ("-C", "debug-assertions=yes"), ("-C", "debug-assertions=no")
    a, b = lab.c(C7_C, c_on), lab.rust(C7_RUST, r_on)
    assert (a.out, b.out) == ("2\n", "2\n")
    a, b = lab.c(C7_C, c_off), lab.rust(C7_RUST, r_off)
    assert (a.out, b.out) == ("1\n", "1\n")
    # trap: evaluating the side effect outside the assertion always runs it
    naive = 'fn main(){ let mut x = 1; let ok = x > 0; x += 1; assert!(ok); println!("{}", x); }\n'
    assert lab.rust(naive, r_off).out == "2\n" != a.out


def test_c7_failed_assert_aborts_both(lab: Lab) -> None:
    """C7: a failing `assert` stops the program before later output, in C (abort) and Rust (panic)."""
    c = (
        "#include <assert.h>\n#include <stdio.h>\n"
        'int main(void){ int x = 0; assert(x > 0); printf("after\\n"); return 0; }\n'
    )
    r = 'fn main(){ let x = std::hint::black_box(0); assert!(x > 0); println!("after"); }\n'
    a, b = lab.c(c), lab.rust(r)
    assert a.code != 0 and b.code != 0
    assert a.out == "" and b.out == ""


def test_c8_exit_in_library_vs_err(lab: Lab) -> None:
    """C8: `exit`/`abort` in a library: Stage 1 keeps them (same stdout and code); Stage 2 returns `Err` (Deviation)."""
    c = r"""
#include <stdio.h>
#include <stdlib.h>
static int lib_check(int v) {
    if (v < 0) { printf("fatal\n"); exit(3); }
    return v * 2;
}
int main(void) {
    printf("%d\n", lib_check(2));
    printf("%d\n", lib_check(-1));
    printf("after\n");
    return 0;
}
"""
    stage1 = r"""
fn lib_check(v: i32) -> i32 {
    if v < 0 { println!("fatal"); std::process::exit(3); }
    v * 2
}
fn main() {
    println!("{}", lib_check(2));
    println!("{}", lib_check(-1));
    println!("after");
}
"""
    stage2 = r"""
fn lib_check(v: i32) -> Result<i32, ()> {
    if v < 0 { return Err(()); }
    Ok(v * 2)
}
fn main() {
    println!("{}", lib_check(2).unwrap());
    match lib_check(-1) { Ok(v) => println!("{}", v), Err(()) => println!("fatal") }
    println!("after");
}
"""
    a, b, d = lab.c(c), lab.rust(stage1), lab.rust(stage2)
    assert (a.out, a.code) == ("4\nfatal\n", 3)
    assert (b.out, b.code) == (a.out, a.code)
    # Stage 2 trap: observable behaviour changed on bad input (continues, exit code 0)
    assert d.out != a.out and d.code != a.code
    # abort(): both die by SIGABRT
    ca = lab.c("#include <stdlib.h>\nint main(void){ abort(); }\n")
    ra = lab.rust("fn main(){ std::process::abort(); }\n")
    assert ca.code == ra.code != 0
