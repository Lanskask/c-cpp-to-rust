#!/usr/bin/env python3
"""Generate the cJSON differential corpus into <output-dir> (deterministic: fixed seed).

Usage: python3 -I make_inputs.py <output-dir> [<upstream-tests-dir>]

A file is one JSON text "A", or "A" + "\\n===\\n" + "B" (see dump.c): B is the second document / patch array.
Groups (judge.md §3):
  1. valid       hand-written: every grammar feature, numbers, escapes, nesting, long strings and arrays
  2. broken      hand-written: truncation classes, unmatched delimiters, bad escapes, control and high bytes, NUL
  3. prefix      every prefix of a set of valid documents (every truncation point)
  4. byte        every byte value 0..255 in each syntactic position the parser or Utils treats differently
  5. mutation    seeded flip / cut / insert / duplicate of the valid inputs
  6. random      seeded random document pairs for patches, merge patches and compare
  7. patch       hand-written patch cases: ~0 / ~1 paths (Q20), every op, every status code (Q23)
  8. upstream    tests/inputs/* as documents and tests/json-patch-tests/*.json cases as (doc, patch) pairs
  9. judge       the user-approved cases: D3 locale numbers, D6 NaN text, D15 (all documents are printed with
                 prebuffer 0), D19 keys with bytes >= 0x80, Q15 minify input, Q20 pointer decoding
"""

from __future__ import annotations

import json
import random
import sys
from pathlib import Path

SEED = 20261010
SEP = b"\n===\n"

VALID: list[bytes] = [
    b"{}", b"[]", b"null", b"true", b"false", b"0", b"-0", b"1", b"-1", b"1.5", b"-1.5e3", b"1E+2", b"1e-2",
    b'""', b'"abc"', b'"a\\"b"', b'"\\\\"', b'"\\/"', b'"\\b\\f\\n\\r\\t"', b'"\\u00e9"', b'"\\u20ac"',
    b'"\\ud83d\\ude00"', b'"\\u0041\\u0042"', b'"\xc3\xa9\xe2\x82\xac"', b'"\xff\xfe"', b'"tab\there"',
    b'{"a":1}', b'{"a":1,"b":2}', b'{"a":{"b":{"c":1}}}', b'{"a":[1,2,3],"b":[]}', b'{"A":1,"a":2}',
    b'{"k":"v","k":"w"}', b'{"":1}', b'{"a b":" c "}', b'[1,2,3]', b'[[],[[]],{}]', b'[null,true,false]',
    b'[1,"a",[2,{"b":null}]]', b'  \n\t[ 1 , 2 ]  ', b'\xef\xbb\xbf{"bom":1}', b'\xef\xbb\xbf[]', b'\xef\xbb\xbf1',
    b'{"text":"line1\\nline2","num":3.14159,"neg":-1,"big":123456789012345}',
    b'[0.1,0.2,0.30000000000000004,1e308,1.7976931348623157e308,5e-324,4.9e-324,2147483647,2147483648,'
    b'-2147483648,-2147483649,1e999,-1e999,1e-999]',
    b'[123456789012345678901234567890,0.000001,0.0000001,100000000000000000000,1.0,10.0,1.10]',
    b'{"widget":{"debug":"on","window":{"title":"Sample","width":500,"height":500},"text":{"data":"Click",'
    b'"size":36,"style":"bold","hOffset":250}}}',
    b'[true,false,null,"s",1,{"a":[]}]',
    b'{"b":2,"a":1,"C":3,"c":4,"B":5}',
    b'{"a":"x\\u0000y"}', b'["\\u0000"]', b'{"a\\u0000b":1}',
    b"[" + b",".join(str(i).encode() for i in range(300)) + b"]",   # crosses the 256-byte print buffer
    b'["' + b"x" * 700 + b'"]',
    b'{"' + b"k" * 300 + b'":"' + b"v" * 300 + b'"}',
    b"[" * 10 + b"]" * 10,
    b"[" * 999 + b"]" * 999,
    b"[" * 1000 + b"]" * 1000,       # CJSON_NESTING_LIMIT: exactly at the limit
    b"[" * 1001 + b"]" * 1001,       # one over
    b'{"a":' * 999 + b"1" + b"}" * 999,
    b'{"a":' * 1000 + b"1" + b"}" * 1000,
    b"[" * 2000,
]

BROKEN: list[bytes] = [
    b"", b" ", b"\n", b"\x00", b"{", b"[", b"}", b"]", b"{]", b"[}", b'{"a"', b'{"a":', b'{"a":1', b'{"a":1,',
    b'["abc', b'"a\\', b'"\\u12', b'"\\u12G4"', b'"\\x"', b'"\\ud83d"', b'"\\ude00"', b'"\\ud83dx"',
    b'"\\ud83d\\u0041"', b'"\\ud83d\\ud83d"', b'{"a":1}}', b"[1,2]]", b"{{}}", b"{[]}", b"[{]}", b'{"a" "b"}',
    b"{1:2}", b'{"a":1,}', b"[,]", b"[1,]", b"[1 2]", b"tru", b"nul", b"fals", b"truex", b"nulll", b"1e",
    b"1e+", b"1.", b".5", b"-", b"--1", b"+1", b"01", b"1.5.3", b"0x10", b"1e5e5", b"- 1", b"NaN", b"Infinity",
    b"-Infinity", b"\x01", b'["\x01"]', b"[\x7f]", b'["\xc3\xa9"]', b"\xff\xfe", b'{"a":\x00"b"}', b"abc",
    b"{abc:def}", b"[:]", b"{:}", b"[1,,2]", b":", b",", b'{"a":[}', b'["\\', b"\\", b'"', b'""', b"'a'",
    b'{"a":"b"', b"[[]", b"[]]", b'{"a":{"b":1}', b'"a"b', b"[1]x", b"[1] x", b"1 2", b"{}{}", b"/* c */ 1",
    b"// c\n1", b"[1,2", b'{"a":[1,2', b'{"a":"\\u00', b"\xef\xbb", b"\xef\xbb\xbf", b"\xef\xbb\xbf\xef\xbb\xbf1",
    b"\xef\xbb\xbf{", b"\xef\xbb\xbf[1,", b'{"a":tru}', b"[nul]", b"\t\r\n 1", b"\x1f1", b"1\x00 2", b'"a\x00b"',
]

PATCH_DOC = (
    b'{"a/b":1,"ab/cd":2,"a~b":3,"x/":4,"~":5,"/":6,"foo":[1,2,3],"bar":{"baz":[{"q":1},{"q":2}]},'
    b'"Foo":"upper","":"empty"}'
)
PATCH_OPS: list[bytes] = [
    b'[{"op":"add","path":"/new","value":1}]',
    b'[{"op":"add","path":"/foo/1","value":9}]',
    b'[{"op":"add","path":"/foo/-","value":9}]',
    b'[{"op":"add","path":"/foo/3","value":9}]',
    b'[{"op":"add","path":"/foo/4","value":9}]',
    b'[{"op":"add","path":"/foo/01","value":9}]',
    b'[{"op":"add","path":"/foo/-1","value":9}]',
    b'[{"op":"add","path":"/foo/x","value":9}]',
    b'[{"op":"add","path":"","value":{"whole":1}}]',
    b'[{"op":"add","path":"/a~1b","value":9}]',
    b'[{"op":"add","path":"/a~0b","value":9}]',
    b'[{"op":"add","path":"/~0","value":9}]',
    b'[{"op":"add","path":"/~1","value":9}]',
    b'[{"op":"add","path":"/x~1","value":9}]',
    b'[{"op":"add","path":"/bar/baz/0/r","value":9}]',
    b'[{"op":"add","path":"/nope/deep","value":9}]',
    b'[{"op":"remove","path":"/a~1b"}]',        # Q20: ~1 not last -> status 13
    b'[{"op":"remove","path":"/ab~1cd"}]',
    b'[{"op":"remove","path":"/x~1"}]',
    b'[{"op":"remove","path":"/a~0b"}]',
    b'[{"op":"remove","path":"/~0"}]',
    b'[{"op":"remove","path":"/~1"}]',
    b'[{"op":"remove","path":"/foo/0"}]',
    b'[{"op":"remove","path":"/foo/9"}]',
    b'[{"op":"remove","path":"/nope"}]',
    b'[{"op":"remove","path":""}]',
    b'[{"op":"replace","path":"/a~1b","value":9}]',
    b'[{"op":"replace","path":"/ab~1cd","value":9}]',
    b'[{"op":"replace","path":"/a~0b","value":9}]',
    b'[{"op":"replace","path":"/x~1","value":9}]',
    b'[{"op":"replace","path":"/foo/2","value":"r"}]',
    b'[{"op":"replace","path":"/FOO","value":"r"}]',
    b'[{"op":"replace","path":"","value":[1]}]',
    b'[{"op":"move","from":"/foo/0","path":"/foo/2"}]',
    b'[{"op":"move","from":"/a~1b","path":"/moved"}]',
    b'[{"op":"move","from":"/nope","path":"/moved"}]',
    b'[{"op":"move","path":"/moved"}]',
    b'[{"op":"copy","from":"/foo/0","path":"/copied"}]',
    b'[{"op":"copy","from":"/bar","path":"/bar2"}]',
    b'[{"op":"copy","from":"/nope","path":"/copied"}]',
    b'[{"op":"test","path":"/foo/0","value":1}]',
    b'[{"op":"test","path":"/foo/0","value":2}]',
    b'[{"op":"test","path":"/bar","value":{"baz":[{"q":1},{"q":2}]}}]',
    b'[{"op":"test","path":"/nope","value":1}]',
    b'[{"op":"bogus","path":"/foo"}]',
    b'[{"path":"/foo"}]',
    b'[{"op":"add","value":1}]',
    b'[{"op":"add","path":"/n"}]',
    b'[{"op":1,"path":"/n","value":1}]',
    b'[{"op":"add","path":7,"value":1}]',
    b'{"op":"add","path":"/n","value":1}',
    b'[1]',
    b'[]',
    b'[{"op":"add","path":"/n1","value":1},{"op":"remove","path":"/a~1b"},{"op":"add","path":"/n2","value":2}]',
    b'[{"op":"add","path":"/n1","value":1},{"op":"test","path":"/n1","value":2},{"op":"add","path":"/n2","value":2}]',
    b'[{"OP":"add","path":"/n","value":1}]',
    b'[{"op":"ADD","path":"/n","value":1}]',
]


def rand_json(rng: random.Random, depth: int = 0) -> bytes:
    keys = [b"a", b"b", b"A", b"B", b"c", b"key", b"Key", b"a b", b"a/b", b"a~b", b"x", b"y", b"\xc3\xa9", b""]
    r = rng.random()
    if depth >= 3 or r < 0.35:
        k = rng.randrange(8)
        return [b"null", b"true", b"false", str(rng.randrange(-5, 50)).encode(),
                f"{rng.uniform(-100, 100):.3f}".encode(), b'"s%d"' % rng.randrange(5), b'"S%d"' % rng.randrange(5),
                b'""'][k]
    if r < 0.65:
        return b"[" + b",".join(rand_json(rng, depth + 1) for _ in range(rng.randrange(0, 5))) + b"]"
    ks = [rng.choice(keys) for _ in range(rng.randrange(0, 5))]
    return b"{" + b",".join(b'"' + k + b'":' + rand_json(rng, depth + 1) for k in ks) + b"}"


def flip_latin1(b: int) -> int:
    if 0x41 <= b <= 0x5A or 0x61 <= b <= 0x7A or 0xC0 <= b <= 0xDE or 0xE0 <= b <= 0xFE:
        return b ^ 0x20
    return b


def byte_cases() -> list[tuple[str, bytes]]:
    out: list[tuple[str, bytes]] = []
    for b in range(256):
        c = bytes([b])
        out += [
            (f"byte-top-{b:02x}", c),
            (f"byte-str-{b:02x}", b'["' + c + b'"]'),
            (f"byte-esc-{b:02x}", b'["\\' + c + b'"]'),
            (f"byte-num-{b:02x}", b"[1" + c + b"2]"),
            (f"byte-key-{b:02x}", b'{"k' + c + b'":1}'),
            (f"byte-val-{b:02x}", b"[" + c + b"1]"),
            (f"byte-hex-{b:02x}", b'["\\u00' + c + b'1"]'),
            (f"byte-lit-{b:02x}", b"[tr" + c + b"e]"),
            (f"byte-ptr-{b:02x}", b'{"a":{"b":1}}' + SEP + b'[{"op":"remove","path":"/a/' + c + b'"}]'),
            (f"byte-keyci-{b:02x}", b'{"' + c + b'":1,"z":0}' + SEP + b'{"' + bytes([flip_latin1(b)]) + b'":2}'),
            (f"byte-neg-{b:02x}", b"-" + c),
        ]
    return out


def main() -> int:
    if len(sys.argv) not in (2, 3):
        print(__doc__, file=sys.stderr)
        return 1
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    for old in out.glob("*"):
        if old.is_file():
            old.unlink()
    rng = random.Random(SEED)
    cases: list[tuple[str, bytes]] = []

    cases += [(f"valid-{i:03d}", v) for i, v in enumerate(VALID)]
    cases += [(f"broken-{i:03d}", v) for i, v in enumerate(BROKEN)]

    prefix_sources = [v for v in VALID if 3 < len(v) < 70][:14]
    for i, src in enumerate(prefix_sources):
        for n in range(1, len(src)):
            cases.append((f"prefix-{i:02d}-{n:03d}", src[:n]))

    cases += byte_cases()

    for i, seed in enumerate(VALID):
        if len(seed) > 120:
            continue
        for j in range(8):
            b = bytearray(seed)
            kind = rng.choice(["flip", "cut", "insert", "dup"])
            if kind == "flip" and b:
                b[rng.randrange(len(b))] = rng.choice(b'{}[]",:\\\x00 a1-.eE+/~\xc3\xff')
            elif kind == "cut" and b:
                del b[rng.randrange(len(b)):]
            elif kind == "insert":
                b.insert(rng.randrange(len(b) + 1), rng.choice(b'{}[]",:\\ x9-.eE+/~\xc3\xff\x00'))
            elif kind == "dup" and b:
                k = rng.randrange(len(b))
                b[k:k] = b[k:k + 3]
            cases.append((f"mutation-{i:03d}-{j}", bytes(b)))

    for i in range(120):
        cases.append((f"random-{i:03d}", rand_json(rng) + SEP + rand_json(rng)))
    for i in range(40):
        d = rand_json(rng)
        cases.append((f"random-same-{i:03d}", d + SEP + d))

    for i, p in enumerate(PATCH_OPS):
        cases.append((f"patch-{i:03d}", PATCH_DOC + SEP + p))
    for i, p in enumerate(PATCH_OPS[:20]):
        cases.append((f"patch-small-{i:03d}", b'{"a":{"b":[1,2]},"c":"d"}' + SEP + p))

    # judge cases (user-approved deviations and proved quirks)
    judge: list[tuple[str, bytes]] = [
        ("judge-d3-number", b"1.5"),
        ("judge-d3-array", b"[1.5,2.25,-0.5e2,1e-3,123456.789]"),
        ("judge-d3-object", b'{"pi":3.14159,"e":2.71828}'),
        ("judge-d3-comma", b"[1,5]" + SEP + b"1,5"),
        ("judge-d6-nan-text", b"NaN"),
        ("judge-d6-nan-array", b"[NaN,nan,-nan]"),
        ("judge-d15-small", b"[1]"),
        ("judge-d15-empty-object", b"{}"),
        ("judge-d19-latin1", b'{"\xc9":1,"\xe9":2}' + SEP + b'{"\xe9":3}'),
        ("judge-d19-latin1-sort", b'{"\xe9":1,"\xc9":2,"z":3,"A":4,"\xc0":5,"\xe0":6}'),
        ("judge-d19-latin1-ptr", b'{"\xc9t\xe9":1}' + SEP + b'[{"op":"replace","path":"/\xe9t\xe9","value":2}]'),
        ("judge-d19-utf8", b'{"\xc3\x89":1,"\xc3\xa9":2}' + SEP + b'{"\xc3\xa9":3}'),
        ("judge-d19-mixed", b'{"\xc9t\xe9":1,"b":2,"\xe9t\xc9":3,"B":4}'),
        ("judge-q15-minify", b'["\\\\", "x y"]'),
        ("judge-q15-minify-comment", b'["a" /* c */ , // x\n "b c"] / '),
        ("judge-q15-minify-unterminated", b'[1] /* never ends'),
        ("judge-q20-doc", b'{"a/b":1,"ab/cd":2,"a~b":3,"x/":4}'),
        ("judge-q20-pointer", b'{"a/b":1,"a~b":2}' + SEP + b'[{"op":"test","path":"/a~1b","value":1}]'),
        ("judge-q20-replace", b'{"a/b":1}' + SEP + b'[{"op":"replace","path":"/a~1b","value":9}]'),
        ("judge-q20-last", b'{"a~":1}' + SEP + b'[{"op":"remove","path":"/a~0"}]'),
    ]
    cases += judge

    # array-index digits (decode_array_index_from_pointer): every digit as units, every digit 1-9 as tens
    idx_doc = json.dumps(list(range(100)), separators=(",", ":")).encode()
    cases.append(("judge-idx-array12", b"[0,1,2,3,4,5,6,7,8,9,10,11]"))
    cases.append(("judge-idx-array100", idx_doc))
    for d in range(10):
        t = d % 9 + 1
        ops = [{"op": "replace", "path": f"/{d}", "value": "x"},
               {"op": "replace", "path": f"/{t}{d}", "value": "y"}]
        cases.append((f"judge-idx-digit-{d}", idx_doc + SEP + json.dumps(ops, separators=(",", ":")).encode()))
        for n, o in enumerate(ops):
            cases.append((f"judge-idx-digit-{d}-{n}", idx_doc + SEP + json.dumps([o], separators=(",", ":")).encode()))

    # upstream data (documents and json-patch-tests), when the tests dir is given and present
    tests_dir = Path(sys.argv[2]) if len(sys.argv) == 3 else None
    if tests_dir and (tests_dir / "inputs").is_dir():
        for p in sorted((tests_dir / "inputs").iterdir()):
            if p.is_file() and p.stat().st_size < 200_000:
                cases.append((f"upstream-input-{p.name}", p.read_bytes()))
    if tests_dir and (tests_dir / "json-patch-tests").is_dir():
        for p in sorted((tests_dir / "json-patch-tests").glob("*.json")):
            try:
                data = json.loads(p.read_text())
            except (OSError, ValueError):
                continue
            n = 0
            for case in data:
                if isinstance(case, dict) and "doc" in case and "patch" in case:
                    doc = json.dumps(case["doc"], separators=(",", ":")).encode()
                    pat = json.dumps(case["patch"], separators=(",", ":")).encode()
                    cases.append((f"upstream-patch-{p.stem}-{n:03d}", doc + SEP + pat))
                    n += 1

    seen: set[str] = set()
    for name, data in cases:
        assert name not in seen, name
        seen.add(name)
        (out / f"{name}.json").write_bytes(data)
    print(f"make_inputs: {len(cases)} inputs -> {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
