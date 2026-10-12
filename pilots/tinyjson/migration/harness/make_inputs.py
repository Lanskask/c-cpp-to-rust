#!/usr/bin/env python3
"""Corpus generator for the tinyjson Differential harness (ticket 02, judge.md section 3).

Writes harness/inputs/ (git-ignored): one file per input, `<class>_<NNNN>.json`, and next to it
harness/inputs.manifest.tsv (name, bytes, description; outside inputs/, which holds inputs only). Fixed seed
20261012: the corpus is reproducible; run it twice and `diff -r` shows nothing. Byte-identical inputs are written
once.

Classes (the prefix of the file name):

    valid_   hand-written inputs, one or more per grammar feature (objects, arrays, strings with every escape,
             numbers, literals, nesting, whitespace)
    broken_  truncations of one rich document at every byte offset, unmatched delimiters, bad escapes, empty
             input, control bytes, NUL
    num_     number token zoo (sign, exponent, leading zeros, hex float, inf, nan, range)
    pos_     every byte 0..255 in each syntactic position the parser treats differently
    utf8_    UTF-8 edge cases (valid, overlong, stray continuation, surrogates, > U+10FFFF, BOM, truncated) and
             offset sweeps of an invalid byte (D5, the wbuffer_convert chunk rule)
    deep_    nesting up to depth 200 (D11: 100000 is deliberately not in the corpus)
    mut_     seeded mutations (flip, cut, insert, duplicate) of the valid_ inputs

Usage: python3 -I make_inputs.py [OUT_DIR]      (default: <this directory>/inputs)
"""
import os
import random
import shutil
import sys

SEED = 20261012

out: list[tuple[str, bytes, str]] = []


def add(cls: str, data: bytes, desc: str) -> None:
    out.append((f"{cls}_{sum(1 for o in out if o[0].startswith(cls + '_')):04d}", data, desc))


# --------------------------------------------------------------------------------------------------- valid
VALID: list[tuple[bytes, str]] = [
    (b"{}", "empty object"),
    (b"[]", "empty array"),
    (b" \t\r\n{ \t\r\n} \t\r\n", "whitespace around empty object"),
    (b'{"a":1}', "one int member"),
    (b'{"a":1,"b":2,"c":3}', "three members"),
    (b'{"b":1,"a":2}', "keys out of order"),
    (b'{"a":1,"a":2}', "duplicate key (last wins)"),
    (b'{"":1}', "empty key"),
    (b'{"a":{"b":{"c":{}}}}', "nested objects"),
    (b'[[],[[]],[[[]]]]', "nested arrays"),
    (b'[1,2,3]', "ints"),
    (b'[-1,0,-0,007,12345678901]', "ints with sign and leading zeros"),
    (b'[9223372036854775807,-9223372036854775808]', "int64 limits"),
    (b'[1.5,-2.25,0.0,-0.0,3.14159265358979]', "doubles"),
    (b'[1e5,1E5,1e+5,1e-5,1.5e3,-2.5E-3]', "exponents"),
    (b'[true,false,null]', "literals"),
    (b'[TRUE,False,NULL,nUlL,TrUe]', "literals, mixed case (Q8)"),
    (b'[ true , false , null ]', "literals with spaces"),
    (b'["a","b c","","  "]', "plain strings"),
    (b'["\\"","\\\\","\\/","\\b","\\f","\\n","\\r","\\t"]', "every simple escape"),
    (b'["\\u0041","\\u00e9","\\u4E16\\u754C","\\u0000","\\uFFFF"]', "u escapes (D3)"),
    (b'["\\uD83D\\uDE00"]', "surrogate pair escape (D3, D4)"),
    (b'{"k\\n\\"x":"v\\u0041"}', "escapes in key"),
    ('["héllo","世界","\U0001F600"]'.encode(), "valid UTF-8 in strings"),
    ('{"你好":"世界"}'.encode(), "valid UTF-8 in key and value"),
    (b'{"a":[1,2,{"b":[true,null,"x"]}],"c":"d"}', "mixed nesting"),
    (b'{ "a" : 1 , "b" : [ 1 , 2 ] , "c" : { "d" : null } }', "spaces everywhere"),
    (b'{\n\t"a":\r\n1,\n"b":\n[\n1\n]\n}', "newlines and tabs"),
    (b'{,"a":1,,"b":"x" "c":2}', "odd separators in object (Q9)"),
    (b'["a" "b" {} []]', "no commas in array (Q9)"),
    (b'[1 2 3]', "numbers without commas"),
    (b'["a\x01b\tc"]', "raw control bytes in string (Q12)"),
    (b'[.5,-.5,.]', "dot-led numbers"),
    (b'[0x10,0x1p3]', "hex numbers (D8)"),
    (b'{"a":1}  \n', "trailing whitespace"),
    (b'{"a":1} x', "trailing garbage"),
    (b'{"a":1}{"b":2}', "two documents"),
    (b'[1,2,3]\x00[4]', "NUL then more data (Q10)"),
    (b'\x0b\x0c[1]', "VT and FF as leading whitespace"),
    (b'[1\x0b,2]', "VT inside number (Q11)"),
    (b'{"a":"b"}', "member string"),
    (b'["' + b"a" * 300 + b'"]', "long string"),
    (b"[" + b",".join(str(i).encode() for i in range(200)) + b"]", "200 elements"),
    (b"{" + b",".join(b'"k%d":%d' % (i, i) for i in range(100)) + b"}", "100 members"),
]
for data, desc in VALID:
    add("valid", data, desc)

# a rich document used for truncations
RICH = (b'{"s":"a\\tb\\u0041\\\\","n":[-1,2.5e3,0],"t":true,"f":false,"z":null,'
        b'"o":{"k":[],"e":{}},"u":"\xc3\xa9\xe4\xb8\x96"}')
add("valid", RICH, "rich document")

# --------------------------------------------------------------------------------------------------- broken
for i in range(len(RICH)):
    add("broken", RICH[:i], f"rich document truncated to {i} bytes")
for data, desc in [
    (b"", "empty input"),
    (b" ", "only a space"),
    (b"\x00", "only NUL"),
    (b"\x00{}", "NUL first"),
    (b"{", "open brace"),
    (b"[", "open bracket"),
    (b"}", "close brace"),
    (b"]", "close bracket"),
    (b"{]", "mismatched"),
    (b"[}", "mismatched"),
    (b"{{}}", "object as key"),
    (b"[[]", "unclosed inner"),
    (b"[]]", "extra close"),
    (b"{}}", "extra close brace"),
    (b'{"a"}', "missing colon and value"),
    (b'{"a":}', "missing value"),
    (b'{"a" 1}', "missing colon"),
    (b'{a:1}', "bare key"),
    (b"{'a':1}", "single quotes"),
    (b'{"a":1,}', "trailing comma object"),
    (b'[1,]', "trailing comma array"),
    (b'[,1]', "leading comma array"),
    (b'["a]', "unterminated string"),
    (b'["\\"]', "escaped quote ends input"),
    (b'["\\q"]', "bad escape letter"),
    (b'["\\u12"]', "short hex"),
    (b'["\\u12G4"]', "non-hex digit"),
    (b'["\\u"]', "u without digits"),
    (b'["\\', "backslash at end"),
    (b'["\\u00', "truncated hex"),
    (b'[tru]', "short true"),
    (b'[truee]', "long true"),
    (b'[nul]', "short null"),
    (b'[nulll]', "long null"),
    (b'[fals]', "short false"),
    (b'[x]', "unknown value"),
    (b'[-]', "lone minus"),
    (b'[--1]', "double minus"),
    (b'[1-2]', "minus inside int"),
    (b'[1.2.3]', "two dots"),
    (b'[1e]', "dangling exponent"),
    (b'[99999999999999999999]', "int overflow"),
    (b'[1e999]', "double overflow"),
    (b'42', "number at top level"),
    (b'"str"', "string at top level"),
    (b'true', "literal at top level"),
    (b'null', "null at top level"),
    (b'\xef\xbb\xbf{}', "UTF-8 BOM before object"),
    (b'{"a":1}\x00garbage', "NUL then garbage"),
    (b'[\x01]', "control byte as value"),
    (b'[\x7f]', "DEL as value"),
    (b'\x1b[1]', "ESC first"),
]:
    add("broken", data, desc)

# --------------------------------------------------------------------------------------------------- numbers
for tok in ["0", "-0", "+1", "1", "-1", "00", "007", "1e5", "1E5", "1e+5", "1e-5", "1.", ".1", "-.1", "1.5e3",
            "0x10", "0X1F", "0x1p3", "0x.8p1", "inf", "-inf", "+inf", "infinity", "-infinity", "nan", "-nan",
            "nan(abc)", "NaN", "Infinity", "-", "--1", "1-", "1 2", " 1 ", "\t1\t", "\r1\r", "\n1\n", "1\x0b",
            "1\x0c", "9223372036854775807", "9223372036854775808", "-9223372036854775808",
            "-9223372036854775809", "99999999999999999999", "1e308", "1e309", "-1e309", "1e-310", "1e-400",
            "4.9e-324", "1,5", "1;5", "0.1e1", "1e1.5", "1_000", "1d5", "1f", "١٢٣", "1e+", "1e-", "e5", "E",
            "-e5", "1.7976931348623157e308", "123456789012345678901234567890", "0.000000000000000000001"]:
    add("num", b"[" + tok.encode() + b"]", f"number token {tok!r} in array")
    add("num", b'{"n":' + tok.encode() + b"}", f"number token {tok!r} in object")

# --------------------------------------------------------------------------------------------------- byte x position
POS: list[tuple[str, bytes, bytes]] = [
    ("array value start", b"[", b"]"),
    ("array value after comma", b"[1,", b"]"),
    ("inside string value", b'["a', b'b"]'),
    ("inside key", b'{"a', b'b":1}'),
    ("after backslash in string", b'["\\', b'"]'),
    ("after backslash in key", b'{"\\', b'":1}'),
    ("first hex digit", b'["\\u', b'000"]'),
    ("inside number", b"[1", b"2]"),
    ("inside true", b"[t", b"ue]"),
    ("inside false", b"[fa", b"se]"),
    ("inside null", b"[nu", b"ll]"),
    ("whitespace before colon", b'{"a"', b":1}"),
    ("object key start after open brace", b"{", b'"a":1}'),
    ("object key start after comma", b'{"a":1,', b'"b":2}'),
    ("top level before document", b"", b"[]"),
    ("top level after document", b"[]", b""),
]
for name, pre, post in POS:
    for b in range(256):
        add("pos", pre + bytes([b]) + post, f"byte 0x{b:02x}: {name}")

# --------------------------------------------------------------------------------------------------- UTF-8
UTF8: list[tuple[bytes, str]] = [
    (b"\xc3\xa9", "valid 2-byte"),
    (b"\xe4\xb8\x96", "valid 3-byte"),
    (b"\xf0\x9f\x98\x80", "valid 4-byte"),
    (b"\xef\xbf\xbd", "U+FFFD"),
    (b"\xef\xbf\xbf", "U+FFFF"),
    (b"\xf4\x8f\xbf\xbf", "U+10FFFF"),
    (b"\xef\xbb\xbf", "BOM inside"),
    (b"\xc0\x80", "overlong NUL (2-byte)"),
    (b"\xc1\xbf", "overlong (2-byte)"),
    (b"\xe0\x80\x80", "overlong NUL (3-byte)"),
    (b"\xe0\x9f\xbf", "overlong (3-byte)"),
    (b"\xf0\x80\x80\x80", "overlong NUL (4-byte)"),
    (b"\xf0\x8f\xbf\xbf", "overlong (4-byte)"),
    (b"\x80", "stray continuation 0x80"),
    (b"\xbf", "stray continuation 0xbf"),
    (b"\x80\x80\x80", "three continuations"),
    (b"\xed\xa0\x80", "surrogate U+D800"),
    (b"\xed\xbf\xbf", "surrogate U+DFFF"),
    (b"\xed\xa0\xbd\xed\xb8\x80", "surrogate pair as two 3-byte"),
    (b"\xf4\x90\x80\x80", "U+110000"),
    (b"\xf5\x80\x80\x80", "F5 lead"),
    (b"\xf8\x88\x80\x80\x80", "5-byte form"),
    (b"\xfc\x84\x80\x80\x80\x80", "6-byte form"),
    (b"\xfe", "FE"),
    (b"\xff", "FF"),
    (b"\xc3", "truncated 2-byte"),
    (b"\xe4\xb8", "truncated 3-byte"),
    (b"\xf0\x9f\x98", "truncated 4-byte"),
    (b"\xc3\x28", "bad continuation after C3"),
    (b"\xe4\xb8\x28", "bad continuation after E4 B8"),
]
for bytes_, desc in UTF8:
    add("utf8", b'["' + bytes_ + b'"]', f"{desc} in string")
    add("utf8", b'{"' + bytes_ + b'":1}', f"{desc} in key")
    add("utf8", bytes_ + b"[]", f"{desc} at start")
    add("utf8", b"[]" + bytes_, f"{desc} after document")
    add("utf8", b"[1," + bytes_ + b"]", f"{desc} as value")
    add("utf8", b'{"a":1' + bytes_ + b"}", f"{desc} after member")
# offset sweeps: where does an invalid byte end the input (D5)? k filler bytes, then the bad sequence, then a tail
for bad_name, bad in [("FF", b"\xff"), ("80", b"\x80"), ("C0 80", b"\xc0\x80"), ("ED A0 80", b"\xed\xa0\x80"),
                      ("F4 90 80 80", b"\xf4\x90\x80\x80"), ("E4 B8 truncated", b"\xe4\xb8")]:
    for k in range(0, 71):
        add("utf8", b'["' + b"a" * k + bad + b'"]', f"sweep {bad_name} after {k} filler bytes")
# the same with a valid 3-byte char before the bad byte (do multi-byte chars shift the chunk?)
for k in range(0, 24):
    add("utf8", b'["' + b"a" * k + b"\xe4\xb8\x96" + b"b" * 3 + b'\xff"]', f"sweep FF after {k} a + CJK + bbb")
for k in range(0, 24):
    add("utf8", b'["' + b"a" * k + b'\xff' + b'a' * 60 + b'"]', f"sweep FF after {k} a, then 60 a")

# --------------------------------------------------------------------------------------------------- depth
for d in (1, 2, 3, 5, 10, 20, 50, 100, 150, 200):
    add("deep", b"[" * d + b"]" * d, f"{d} nested arrays")
for d in (1, 2, 5, 10, 50, 100):
    add("deep", b'{"a":' * d + b"1" + b"}" * d, f"{d} nested objects")
for d in (2, 10, 50, 100):
    add("deep", b'[{"a":' * d + b"null" + b"}]" * d, f"{d} alternating array and object")
add("deep", b"[" * 100, "100 unclosed arrays")
add("deep", b"[" * 200 + b"1" + b"]" * 199, "200 arrays, one close missing")

# --------------------------------------------------------------------------------------------------- mutations
rng = random.Random(SEED)
bases = [d for (n, d, _) in out if n.startswith("valid_") and 2 < len(d) < 400]
interesting = b'{}[]",:\\ \t\n\x00\x01-+.eE0123456789truefalsnuTRUEFALSN\xc3\xa9\xff\x80u'
for i in range(500):
    base = rng.choice(bases)
    op = rng.choice(["flip", "cut", "insert", "duplicate"])
    pos = rng.randrange(len(base))
    if op == "flip":
        m = bytearray(base)
        m[pos] ^= 1 << rng.randrange(8)
        data = bytes(m)
    elif op == "cut":
        n = rng.randrange(1, 1 + min(8, len(base) - pos))
        data = base[:pos] + base[pos + n:]
    elif op == "insert":
        data = base[:pos] + bytes([rng.choice(interesting)]) + base[pos:]
    else:
        n = rng.randrange(1, 1 + min(12, len(base) - pos))
        data = base[:pos + n] + base[pos:pos + n] + base[pos + n:]
    add("mut", data, f"{op} at {pos}")


def main() -> int:
    here = os.path.dirname(os.path.abspath(__file__))
    dest = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, "inputs")
    if os.path.isdir(dest):
        shutil.rmtree(dest)
    os.makedirs(dest)
    seen: dict[bytes, str] = {}
    kept = 0
    with open(dest.rstrip("/") + ".manifest.tsv", "w") as man:
        for name, data, desc in out:
            if data in seen:  # byte-identical input already present: keep one copy only
                continue
            seen[data] = name
            with open(os.path.join(dest, name + ".json"), "wb") as f:
                f.write(data)
            man.write(f"{name}\t{len(data)}\t{desc}\n")
            kept += 1
    print(f"{kept} inputs written to {dest} ({len(out) - kept} byte-identical duplicates skipped)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
