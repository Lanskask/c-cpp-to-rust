#!/usr/bin/env python3
"""e1: where does an invalid UTF-8 byte end the input of tinyjson::parser::parse (RULEBOOK D5)?

QUESTION. parser::parse feeds its bytes through std::wbuffer_convert<codecvt_utf8<char32_t>> (tinyjson.h:630-633).
D5 says an invalid byte is not an error but an early EOF "at the start of the libstdc++ conversion chunk that
holds it". For which input offsets, and which preceding bytes, does the EOF land where? The Rust port must
reproduce the same offset.

METHOD. Source reading (GCC 16.1.0 bits/locale_conv.h, wbuffer_convert::_M_conv_get) gives a model; this script
tests the model against the C++ driver (harness/work/dump, reference build: GCC 16 -O0, libstdc++).
Model (code `delivered_bytes`):
  * the converter works on a window of at most 29 bytes (`_M_get_buf` is 32-3 bytes): it reads
    min(29 - unconv, bytes left) new bytes after the `unconv` bytes kept from the previous window;
  * `codecvt_utf8::in` converts the window. Complete characters are delivered. An incomplete character at the
    end of the window (lead byte whose continuation bytes are not all in the window) stays as `unconv` bytes
    for the next window. A byte sequence that is invalid makes `in` return `error`: _M_conv_get returns false,
    underflow returns EOF, and the characters already converted in that window are LOST (the get area is set but
    never read: the stream has hit eofbit). So the input ends at the START of the window that holds the first
    invalid byte, which is the offset of the first byte after the last character delivered by earlier windows.
  * the stream never recovers: nothing after that offset is seen.
Observable. input = '["' + 'é'*m + 'a'*n + '"]' + ' '*s + BAD + 'x'. The document is complete before BAD, so
parse succeeds iff the window holding BAD starts at or after the end of the document. Sweeping s and m moves BAD
and the window starts relative to the document end. `e(p)` (the window start for BAD at offset p) is measured
directly with a whitespace-only document `[` + ' '*k + `]`, whose length k+2 is varied by binary search.

Run: python3 -I e1_tinyjson_d5_chunk_rule.py        (needs harness/work/dump from migration/harness/ref/build_ref.sh)

ANSWER (measured 2026-10-11, GCC 16.1.0 -O0, libstdc++ 16.1.0, macOS arm64; numbers printed by this script):
  The rule is exact. The input is delivered in windows of at most 29 bytes. Window 0 starts at byte 0. A window
  is converted as a whole; its last bytes may be a valid but incomplete UTF-8 sequence (u bytes), in which case
  the next window starts at those u bytes (it holds the u kept bytes plus 29-u new ones). If a window holds an
  invalid byte (judged against the Unicode well-formed table, also for a sequence cut by the window end), the
  parser sees EOF at the START of that window: every character of that window is lost, nothing after is read.
  Part 1 (ASCII prefix, FF at offset p, p = 2..124, 123 offsets): the document-survival boundary e(p) took the
  values 0, 29, 58, 87, 116 = 29*floor(p/29), 0 mismatches against the model; a document of at least 2 bytes
  survives from p = 29 on (the RULEBOOK example: 26 'a' + FF is p = 28 -> 'invalid json format').
  Part 2 (12 invalid sequences x 4 prefix units (none, 2-, 3-, 4-byte char) x 0..5 units x 70 offsets, 20160
  inputs): the model and the driver agree on all 20160. Mismatches of a first model that tested "complete"
  before "valid" were 232 of 26880, all sequences cut by a window end (C0 80, E0 80 80, ED A0 80, F4 90 80 80).
  A port: keep a 29-byte window, decode with the well-formed table, on an error stop at the window start.
  Not covered: invalid bytes beyond offset about 95 (part 2 reaches 4 windows; part 1 reaches offset 124 with
  ASCII only), so a chain of 5 or more windows that each end in a split character is untested.
"""
import concurrent.futures
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
PILOT = os.path.dirname(HERE)
DUMP = os.path.join(PILOT, "migration", "harness", "work", "dump")
WINDOW = 29
MAXCODE = 0x10FFFF


# ---------------------------------------------------------------------------------------------------- model
def decode(buf: bytes) -> tuple[int, str]:
    """Model of codecvt_utf8<char32_t>::in over one window. Returns (bytes consumed, 'ok'|'partial'|'error').

    A sequence is checked byte by byte against the well-formed table of the Unicode standard (Table 3-7: lead
    C2..F4; second byte A0..BF after E0, 80..9F after ED, 90..BF after F0, 80..8F after F4, else 80..BF; later
    bytes 80..BF). The first byte that breaks the table is an error even when the sequence is cut by the end of
    the window; a valid but incomplete prefix at the end of the window is 'partial'. (A first model that only
    checked 'is the sequence complete' before 'is it valid' disagreed with the driver in 232 of 26880 inputs,
    all of them sequences cut by a window end: C0 80, E0 80 80, ED A0 80, F4 90 80 80.)
    """
    i = 0
    n = len(buf)
    while i < n:
        c = buf[i]
        if c < 0x80:
            i += 1
            continue
        if c < 0xC2 or c > 0xF4:
            return i, "error"
        need = 2 if c <= 0xDF else 3 if c <= 0xEF else 4
        lo, hi = 0x80, 0xBF
        if c == 0xE0:
            lo = 0xA0
        elif c == 0xED:
            hi = 0x9F
        elif c == 0xF0:
            lo = 0x90
        elif c == 0xF4:
            hi = 0x8F
        for k in range(1, min(n - i, need)):
            b = buf[i + k]
            if k == 1 and not (lo <= b <= hi):
                return i, "error"
            if k > 1 and not (0x80 <= b <= 0xBF):
                return i, "error"
        if n - i < need:
            return i, "partial"
        i += need
    return i, "ok"


def delivered_bytes(data: bytes) -> int:
    """Number of input bytes whose characters reach the parser before EOF (model)."""
    start = 0          # offset of the first byte of the window
    unconv = 0
    read = 0           # bytes read from the stream so far
    delivered = 0
    while True:
        n = min(WINDOW - unconv, len(data) - read)
        if n < 1:
            return delivered
        window = data[start:read + n]
        read += n
        used, status = decode(window)
        if status == "error":
            return start                      # characters of this window are lost
        delivered = start + used
        unconv = len(window) - used
        start = start + used


# ---------------------------------------------------------------------------------------------------- driver
def parse_result(tmpdir: str, data: bytes, tag: str) -> str:
    path = os.path.join(tmpdir, tag + ".json")
    with open(path, "wb") as f:
        f.write(data)
    out = subprocess.run([DUMP, path], capture_output=True, check=True).stdout
    first = out.split(b"\n", 2)[1]            # line 0 is input.size, line 1 is the parse record
    return first.decode("latin-1")


def ok(rec: str) -> bool:
    return rec.startswith("parse = ")


def main() -> int:
    if not os.path.exists(DUMP):
        print("build the driver first: bash migration/harness/ref/build_ref.sh", file=sys.stderr)
        return 2
    tmp = tempfile.mkdtemp(prefix="e1_d5_")
    pool = concurrent.futures.ThreadPoolExecutor(max_workers=8)

    # --- part 1: e(p), ASCII only ------------------------------------------------------------------------
    def e_of(p: int) -> int:
        """Largest document length j (2..p) that still parses with BAD at offset p; 0 if none (EOF before `[`)."""
        lo, hi, best = 2, p, 0
        while lo <= hi:
            mid = (lo + hi) // 2
            doc = b"[" + b" " * (mid - 2) + b"]" + b" " * (p - mid) + b"\xff" + b"x"
            if ok(parse_result(tmp, doc, f"a{p}_{mid}")):
                best, lo = mid, mid + 1
            else:
                hi = mid - 1
        return best

    ps = list(range(2, 125))
    measured = list(pool.map(e_of, ps))
    expected = [delivered_bytes(b" " * p + b"\xff") for p in ps]
    bad = [(p, m, x) for p, m, x in zip(ps, measured, expected) if m != x and not (m == 0 and x < 2)]
    print(f"part 1: ASCII prefix, BAD=FF at offset p in 2..124: {len(ps)} offsets, e(p) measured vs model "
          f"mismatches: {len(bad)}")
    steps = sorted({m for m in measured})
    print(f"        distinct measured e(p): {steps}")
    first_cut = next(p for p, m in zip(ps, measured) if m >= 2)
    print(f"        first offset with e(p) >= 2 (document survives): p = {first_cut}")

    # --- part 2: multi-byte prefix, several invalid sequences, model against driver ----------------------
    bads = {
        "FF": b"\xff",
        "80": b"\x80",
        "C0 80": b"\xc0\x80",
        "C2 + x": b"\xc2x",
        "C3 28": b"\xc3\x28",
        "E4 B8 + x": b"\xe4\xb8x",
        "ED A0 80": b"\xed\xa0\x80",
        "E0 80 80": b"\xe0\x80\x80",
        "F0 80 80 80": b"\xf0\x80\x80\x80",
        "F4 90 80 80": b"\xf4\x90\x80\x80",
        "F5 80 80 80": b"\xf5\x80\x80\x80",
        "F8": b"\xf8",
    }
    prefixes = {
        "": b"",
        "e9": "\u00e9".encode(),
        "CJK": "\u4e16".encode(),
        "emoji": "\U0001F600".encode(),
    }
    jobs = []
    for bname, bad_bytes in bads.items():
        for pname, unit in prefixes.items():
            for m in range(0, 6):
                for s in range(0, 70):
                    doc = b'["' + unit * m + b'"]'
                    data = doc + b" " * s + bad_bytes + b"x"
                    jobs.append((bname, pname, m, s, data, len(doc)))

    def run(job):
        bname, pname, m, s, data, doclen = job
        rec = parse_result(tmp, data, f"b{abs(hash(data)) % 10**12}_{len(data)}")
        return job, ok(rec), rec

    results = list(pool.map(run, jobs))
    agree = disagree = 0
    worst = []
    for job, is_ok, rec in results:
        bname, pname, m, s, data, doclen = job
        pred_ok = delivered_bytes(data) >= doclen
        if pred_ok == is_ok:
            agree += 1
        else:
            disagree += 1
            if len(worst) < 8:
                worst.append((bname, pname, m, s, is_ok, rec[:70]))
    print(f"part 2: {len(jobs)} inputs ({len(bads)} invalid sequences x {len(prefixes)} prefix units x m 0..5 "
          f"x s 0..69): model agrees on {agree}, disagrees on {disagree}")
    for w in worst:
        print("        disagree:", w)
    # error texts seen when parse fails, for the record
    texts: dict[str, int] = {}
    for job, is_ok, rec in results:
        if not is_ok:
            texts[rec] = texts.get(rec, 0) + 1
    for t, c in sorted(texts.items(), key=lambda kv: -kv[1])[:6]:
        print(f"        {c:6d}  {t}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
