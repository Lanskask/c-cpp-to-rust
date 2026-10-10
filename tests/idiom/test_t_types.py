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
