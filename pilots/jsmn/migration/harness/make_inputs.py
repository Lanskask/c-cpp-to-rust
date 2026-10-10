#!/usr/bin/env python3
"""Generate the jsmn differential corpus into ./inputs (deterministic: fixed seed).

Hand-written cases cover each branch of jsmn_parse (strings, escapes, \\u, primitives, nesting,
unmatched brackets, control bytes, truncation); seeded byte mutations of the valid cases add breadth.
Usage: python3 -I make_inputs.py <output-dir>
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

VALID = [
    b'{}', b'[]', b'{"a":0}', b'{"a":[1,2,{"b":null}],"c":"d"}', b'[1, "a", true, false, null, -12.5e3]',
    b'{"key":"va\\"lue","k2":"\\u00e9\\n\\t\\\\\\/"}', b'  \n\t[ ]  ', b'"just a string"', b'42', b'true',
    b'{"a":{"b":{"c":{"d":[[],[[]],{}]}}}}', b'[[[[[[[[[[1]]]]]]]]]]', b'{"k":"v","k":"w"}',
    b'{"x":1,"y":2,"z":[3,4,5],"w":"six"}', b'["a","b","c","d","e","f","g","h","i","j"]',
    b'{"text":"line1\\nline2","num":3.14159,"neg":-1,"big":123456789012345}',
]
BROKEN = [
    b'', b' ', b'{', b'[', b'}', b']', b'{]', b'[}', b'{"a"', b'{"a":', b'{"a":1', b'["abc', b'"a\\',
    b'"\\u12', b'"\\u12G4"', b'"\\x"', b'{"a":1}}', b'[1,2]]', b'{{}}', b'{[]}', b'[{]}', b'{"a" "b"}',
    b'{1:2}', b'{"a":1,}', b'[,]', b'[1 2]', b'tru', b'nul', b'1e', b'\x01', b'["\x01"]', b'[\x7f]',
    b'["\xc3\xa9"]', b'\xff\xfe', b'{"a":\x00"b"}', b'abc', b'{abc:def}', b'[:]', b'{:}', b'[1,,2]', b':',
    b',', b'{"a":[}', b'["\\', b'\\', b'"', b'""', b"'a'", b'{"a":"b"', b'[[]', b'[]]', b'{"a":{"b":1}',
]


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 1
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    cases: list[bytes] = [*VALID, *BROKEN]
    rng = random.Random(25)
    for seed in VALID:
        for _ in range(6):
            b = bytearray(seed)
            kind = rng.choice(["flip", "cut", "insert", "dup"])
            if kind == "flip" and b:
                b[rng.randrange(len(b))] = rng.choice(b'{}[]",:\\\x00 a1-')
            elif kind == "cut" and b:
                del b[rng.randrange(len(b)):]
            elif kind == "insert":
                b.insert(rng.randrange(len(b) + 1), rng.choice(b'{}[]",:\\ x9-'))
            elif kind == "dup" and b:
                i = rng.randrange(len(b))
                b[i:i] = b[i : i + 3]
            cases.append(bytes(b))
    # every byte value as a primitive start, as a string char, as an escape char, and as a primitive
    # continuation (`[1<c>]`: in strict mode a leading byte outside [-0-9tfn] is rejected before the
    # per-byte range check, so only a continuation position reaches it; found by E3 Stage 2 validation)
    for v in range(256):
        c = bytes([v])
        cases += [b"[" + c + b"a]", b'["' + c + b'"]', b'["\\' + c + b'"]', b"[1" + c + b"]"]
    for n, data in enumerate(cases):
        (out / f"case{n:03d}.json").write_bytes(data)
    print(f"make_inputs: {len(cases)} files in {out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
