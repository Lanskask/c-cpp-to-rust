#!/usr/bin/env python3
"""List the symbols of a Source project with risky-construct tags into migration/inventory.tsv.

Columns: symbol, kind (function|record|enum|typedef|macro|variable), file (relative to the source root),
lines (start-end), tags (comma-separated; empty if none).
Tags: virtual template goto union setjmp varargs macro-fn static-global errno malloc, and
`config-macro:NAME` for every #if/#ifdef macro that guards or appears inside the symbol (decision 24).
Risky-construct tags (G15): fnptr bitfield float-format strtod qsort ctype long asm signal thread.
`new`/`delete` count as `malloc` only with `--lang cpp`.

Engine, C++ without `--compile-commands`: `ast_facts.py` builds the library files itself and clang's AST names
every overload, method and operator exactly; it also writes `calls.tsv` (resolved call edges, read by depmap.py),
`facts.tsv` (throws, new/delete, user conversions) and `ast-symbols.tsv` next to the output. Test and example
files, and every file clang could not handle, use the regex pass. `--no-ast` forces the regex pass.

Engine: with `--compile-commands` and `clang` on PATH, `clang -Xclang -ast-dump=json` gives exact
declarations; otherwise (or if clang fails on a file) a regex pass over comment-stripped text is used.
The regex pass also always runs on files parsed by clang: its `config-macro`/`macro-fn` tags are merged
into the clang symbols by (file, line span), `#define` macros become `kind=macro` rows, and symbols in
`#if`-disabled code that clang never saw are added (G14).
The engine actually used is printed on stderr and recorded as the first line of the TSV (`# engine: ...`),
followed by the exact command line (`# cmd: ...`).
Exit code 0 on success, 1 on failure.
"""

from __future__ import annotations

import argparse
import fnmatch
import json
import os
import re
import shlex
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

C_EXT = {".c", ".h"}
CPP_EXT = {".c", ".h", ".cc", ".cpp", ".cxx", ".hpp", ".hh", ".hxx", ".inl"}
SKIP_DIRS = {".git", "build", "target", "node_modules", "migration"}  # migration/: the Skill's own output (decision 72)
# vendored or non-library directories that never become Parts (extend per project with --exclude)
VENDORED_DIRS = {
    "fuzzing", "fuzz", "third_party", "thirdparty", "vendor", "external", "unity", "catch2", "gtest", "gmock",
}  # fmt: skip
# vendored single-file test frameworks next to the project's own tests (tinyjson: test/catch.hpp, 1564 symbols)
VENDORED_FILES = {"catch.hpp", "catch_amalgamated.hpp", "catch_amalgamated.cpp", "doctest.h", "doctest.hpp", "gtest.h"}
KEYWORDS = {"if", "for", "while", "switch", "return", "sizeof", "else", "do", "catch", "defined"}
TAG_PATTERNS: dict[str, re.Pattern[str]] = {
    "virtual": re.compile(r"\bvirtual\b"),
    "template": re.compile(r"\btemplate\s*<"),
    "goto": re.compile(r"\bgoto\b"),
    "union": re.compile(r"\bunion\b"),
    "setjmp": re.compile(r"\b(setjmp|longjmp|sigsetjmp|siglongjmp)\b"),
    "varargs": re.compile(r"\.\.\.|\bva_(start|arg|end|list)\b"),
    "errno": re.compile(r"\berrno\b"),
    "malloc": re.compile(r"\b(malloc|calloc|realloc|free|strdup)\b"),
    "strtod": re.compile(r"\b(strtod|atof|strtof|strtold)\b"),
    "qsort": re.compile(r"\b(qsort|qsort_r|qsort_s)\b"),
    "ctype": re.compile(
        r"\b(isalnum|isalpha|isblank|iscntrl|isdigit|isgraph|islower|isprint|ispunct|isspace|isupper|isxdigit"
        r"|tolower|toupper)\b"
    ),
    "long": re.compile(r"\blong\b"),
    "asm": re.compile(r"\b(asm|__asm__|__asm|_asm)\b"),
    "signal": re.compile(r"\b(signal|raise)\s*\(|\bsigaction\b"),
    "thread": re.compile(r"\b(pthread_\w+|thrd_\w+|_Thread_local|thread_local|__thread)\b"),
}
MALLOC_CPP = re.compile(r"\b(malloc|calloc|realloc|free|strdup|new|delete)\b")
# function-pointer declarator/typedef `(*name)(` or `(*name[N])(`; in C also a call through a member `a->f(` / `a.f(`
FNPTR_DECL = re.compile(r"\(\s*\*[\s\w\[\]>.-]*\)\s*\(")
FNPTR_MEMBER_CALL = re.compile(r"->\s*[A-Za-z_]\w*\s*\(|(?<![.\d])\.\s*[A-Za-z_]\w*\s*\(")
# `type name : N;` / `type : N;` as a struct member (line start or after `{` `;`); never a ternary, case label or return
BITFIELD = re.compile(
    r"(?:^|(?<=[{;]))\s*(?!case\b|return\b|default\b|goto\b)[A-Za-z_][\w \t*]*?[ \t]+\*?\w*[ \t]*:[ \t]*\w+[ \t]*[,;]",
    re.MULTILINE,
)
PRINTF_CALL = re.compile(r"\b\w*printf\s*\(")
FLOAT_CONV = re.compile(r"%[-+ #0]*(?:\d+|\*)?(?:\.(?:\d+|\*)?)?(?:hh|h|ll|l|L|j|z|t)?[fFgGeEaA]")
COND = re.compile(r"^\s*#\s*(if|ifdef|ifndef|elif)\b(.*)$")
DEFINE = re.compile(r"^\s*#\s*define\s+([A-Za-z_]\w*)(\()?")
IDENT = re.compile(r"[A-Za-z_]\w*")


@dataclass
class Symbol:
    name: str
    kind: str
    file: str
    start: int
    end: int
    tags: list[str]


def operator_name(before: str, rest: str) -> str | None:
    """`json::operator bool` / `operator==` / `operator()` as one symbol name; None when `before` is no operator.

    `before` is the declaration header up to the first `(`, `rest` the text from that `(` on.
    """
    m = re.search(r"((?:~?\w+::)*)operator(?:\b|(?=[^\w\s]))\s*(.*)$", before)
    if not m:
        return None
    suffix = " ".join(m.group(2).split())
    if not suffix and rest.startswith(")("):  # operator()(args): the first "(" belongs to the name
        suffix = "()"
    if not suffix:
        return None
    sep = " " if (suffix[0].isalpha() or suffix[0] == "_") else ""
    return f"{m.group(1)}operator{sep}{suffix}"


NOT_NAMES = KEYWORDS | {"throw", "noexcept", "__attribute__", "__declspec", "alignas", "decltype", "asm", "__asm__"}


def declarator_prefix(h: str) -> str:
    """The header text before the `(` of the declared function.

    `CJSON_PUBLIC(cJSON *) cJSON_Parse(const char *s)` has two top-level groups: the first is a macro that wraps the
    return type, so the name is in front of the second. A constructor's `: member(x)` initialiser list is not a
    second declarator (a bare `:` sits between the groups).
    """
    groups: list[tuple[int, int]] = []  # (index of "(", index just after its ")") at nesting depth 0
    depth = 0
    start = -1
    for i, ch in enumerate(h):
        if ch == "(":
            if depth == 0:
                start = i
            depth += 1
        elif ch == ")" and depth > 0:
            depth -= 1
            if depth == 0:
                groups.append((start, i + 1))
    if not groups:
        return h
    pick = 0
    for k in range(len(groups) - 1):
        between = h[groups[k][1] : groups[k + 1][0]]
        words = IDENT.findall(between)
        nxt = h[: groups[k + 1][0]].rstrip()
        m = re.search(r"([A-Za-z_]\w*)$", nxt)
        macro_prefix = (
            bool(m)
            and m.group(1) not in NOT_NAMES
            and bool(words)
            and re.fullmatch(r"[\s\w:*&<>]*", between) is not None
            and not re.search(r"(^|[^:]):([^:]|$)", between)
            and (m.group(1) == words[-1])
        )
        if macro_prefix and not re.match(r"\s*(const|noexcept|override|final)\b", between):
            pick = k + 1
        else:
            break
    # drop the macro groups: `CJSON_PUBLIC(cJSON *) cJSON_Parse` -> `CJSON_PUBLIC cJSON_Parse`
    out, pos = "", 0
    for a, b in groups[:pick]:
        out += h[pos:a] + " "
        pos = b
    return out + h[pos : groups[pick][0]]


def strip_comments_and_strings(text: str, keep_strings: bool = False) -> str:
    """Blank out comments and string/char literals but keep every newline so line numbers survive.

    With `keep_strings` the literals are kept (needed to see printf formats)."""
    out: list[str] = []
    i, n = 0, len(text)
    while i < n:
        two = text[i : i + 2]
        c = text[i]
        if two == "//":
            j = text.find("\n", i)
            j = n if j < 0 else j
            out.append(" " * (j - i))
            i = j
        elif two == "/*":
            j = text.find("*/", i + 2)
            j = n if j < 0 else j + 2
            out.append("".join(ch if ch == "\n" else " " for ch in text[i:j]))
            i = j
        elif c in "\"'":
            j = i + 1
            while j < n and text[j] != c and text[j] != "\n":
                j += 2 if text[j] == "\\" else 1
            j = min(j + 1, n)
            if keep_strings:
                out.append(text[i:j])
                i = j
                continue
            out.append(c + " " * (j - i - 2) + c if j - i >= 2 else c)
            i = j
        else:
            out.append(c)
            i += 1
    return "".join(out)


def macro_names(cond_text: str) -> list[str]:
    return [m for m in IDENT.findall(cond_text) if m not in KEYWORDS and m != "__cplusplus"]


def has_float_format(text: str) -> bool:
    """True if a printf-family call in `text` (strings kept) has a %f %g %e %a conversion."""
    for m in PRINTF_CALL.finditer(text):
        end = text.find(";", m.end())
        stmt = text[m.end() : len(text) if end < 0 else end].replace("%%", "")
        if FLOAT_CONV.search(stmt):
            return True
    return False


def has_bitfield(text: str) -> bool:
    return any("?" not in m.group(0) for m in BITFIELD.finditer(text))


def collect_tags(span_text: str, kind: str, cond: list[str], lang: str = "c", fmt_text: str | None = None) -> list[str]:
    """Tags of one symbol. `span_text` has comments and strings blanked; `fmt_text` keeps string literals."""
    tags = []
    for t, pat in TAG_PATTERNS.items():
        if t == "malloc" and lang == "cpp":
            pat = MALLOC_CPP
        if pat.search(span_text):
            tags.append(t)
    if FNPTR_DECL.search(span_text) or (lang == "c" and FNPTR_MEMBER_CALL.search(span_text)):
        tags.append("fnptr")
    if has_bitfield(span_text):
        tags.append("bitfield")
    if has_float_format(span_text if fmt_text is None else fmt_text):
        tags.append("float-format")
    if kind == "macro-fn":
        tags.append("macro-fn")
    for m in sorted(set(cond)):
        tags.append(f"config-macro:{m}")
    return tags


def scan_regex(text: str, rel: str, lang: str = "c") -> list[Symbol]:
    clean = strip_comments_and_strings(text)
    lines = clean.split("\n")
    kept = strip_comments_and_strings(text, keep_strings=True).split("\n")
    if len(kept) != len(lines):
        kept = lines
    syms: list[Symbol] = []
    # --- preprocessor pass: macros, conditional stack, per-line condition names
    cond_stack: list[list[str]] = []
    line_cond: list[list[str]] = []  # enclosing config macros at each line
    inline_cond: dict[int, list[str]] = {}  # macros named by directives on that line
    guard: str | None = None
    code_lines: list[str] = []
    ln = 0
    while ln < len(lines):
        raw = lines[ln]
        start_ln = ln
        # join backslash continuations for directives
        while raw.rstrip().endswith("\\") and ln + 1 < len(lines):
            ln += 1
            raw = raw.rstrip()[:-1] + " " + lines[ln]
        if raw.lstrip().startswith("#"):
            m = COND.match(raw)
            if m:
                names = macro_names(m.group(2))
                if guard is None and m.group(1) == "ifndef" and len(names) == 1:
                    guard = names[0]
                    cond_stack.append([])  # header guard is not a configuration macro
                else:
                    cond_stack.append(names)
                    inline_cond[start_ln + 1] = names
            elif re.match(r"^\s*#\s*elif\b", raw):
                pass
            elif re.match(r"^\s*#\s*else\b", raw):
                pass
            elif re.match(r"^\s*#\s*endif\b", raw):
                if cond_stack:
                    cond_stack.pop()
            d = DEFINE.match(raw)
            if d and d.group(1) != guard:
                kind = "macro-fn" if d.group(2) else "macro"
                flat = [n for names in cond_stack for n in names]
                syms.append(Symbol(d.group(1), "macro", rel, start_ln + 1, ln + 1, collect_tags(raw, kind, flat, lang)))
            for _ in range(start_ln, ln + 1):
                line_cond.append([n for names in cond_stack for n in names])
                code_lines.append("")
        else:
            line_cond.append([n for names in cond_stack for n in names])
            code_lines.append(raw)
        ln += 1
    code = "\n".join(code_lines)
    # --- declaration pass over code with preprocessor lines removed
    depth = 0
    ns_depth = 0
    header_start = 0  # index in `code` where the current top-level declaration began
    brace_open_idx = -1
    ns_stack: list[int] = []
    n = len(code)

    def line_of(idx: int) -> int:
        return code.count("\n", 0, idx) + 1

    def emit(name: str, kind: str, a: int, b: int) -> None:
        a += len(code[a:]) - len(code[a:].lstrip())  # skip whitespace left over from the previous declaration
        sl, el = line_of(a), line_of(b)
        span = "\n".join(code_lines[sl - 1 : el])
        fmt = "\n".join(kept[i] if code_lines[i] else "" for i in range(sl - 1, min(el, len(code_lines))))
        cond = list(line_cond[sl - 1]) if sl - 1 < len(line_cond) else []
        for k in range(sl, el + 1):
            cond += inline_cond.get(k, [])
        # directives inside the span were blanked in `code`; re-scan the original lines for config macros
        for k in range(sl - 1, min(el, len(lines))):
            cond += line_cond[k] if k < len(line_cond) else []
            cond += inline_cond.get(k + 1, [])
        syms.append(Symbol(name, kind, rel, sl, el, collect_tags(span, kind, cond, lang, fmt)))

    def classify(header: str) -> tuple[str, str] | None:
        h = " ".join(header.split())
        if not h:
            return None
        m = re.match(r"^(?:typedef\s+)?(?:\w+\s+)*?(struct|class|union|enum)\s+(?:class\s+)?(\w+)", h)
        # `static enum E f(void) {` is a function returning an enum, not the definition of E (found on cJSON)
        if m and "(" not in h.split(m.group(2))[0] and "(" not in h[m.end() :]:
            return ("enum" if m.group(1) == "enum" else "record", m.group(2))
        if re.match(r"^(typedef\s+)?(struct|union|enum|class)\b", h):
            return ("enum" if "enum" in h.split()[:2] else "record", "")  # anonymous: name from trailing typedef
        if "(" in h:
            before = declarator_prefix(h)
            op = operator_name(before, h[len(before) :])
            if op:
                return ("function", op)
            ids = [x for x in IDENT.findall(before) if x not in KEYWORDS]
            if ids and "=" not in before:
                # Qualified C++ names: Class::method -> keep the last two components
                qual = re.findall(r"[A-Za-z_~]\w*(?:::~?\w+)*", before)
                return ("function", qual[-1] if qual else ids[-1])
        return None

    k = 0
    while k < n:
        ch = code[k]
        if ch == "{":
            if depth == 0:
                header = code[header_start:k]
                h = " ".join(header.split())
                if re.match(r"^(extern\s+\"\s*\w*\s*\"|namespace\b.*)$", h) or re.match(r'^extern\s+"', h):
                    ns_stack.append(depth)  # transparent scope: do not count as depth
                    header_start = k + 1
                    k += 1
                    ns_depth += 1
                    continue
                brace_open_idx = k
            depth += 1
        elif ch == "}":
            if depth == 0 and ns_depth > 0:
                ns_depth -= 1
                header_start = k + 1
            else:
                depth -= 1
                if depth == 0:
                    header = code[header_start:brace_open_idx]
                    cls = classify(header)
                    # consume trailing declarators up to ';' for struct/enum/typedef
                    end = k
                    tail_end = code.find(";", k)
                    after = code[k + 1 : tail_end] if tail_end >= 0 else ""
                    if cls and cls[0] in ("record", "enum"):
                        name = cls[1] or (IDENT.findall(after)[-1] if IDENT.findall(after) else "<anonymous>")
                        end = tail_end if tail_end >= 0 else k
                        emit(
                            name,
                            "typedef" if header.strip().startswith("typedef") and cls[1] == "" else cls[0],
                            header_start,
                            end,
                        )
                        # typedef struct X {...} Y; also names Y
                        if header.strip().startswith("typedef") and cls[1] and IDENT.findall(after):
                            emit(IDENT.findall(after)[-1], "typedef", header_start, end)
                        k = end
                        header_start = k + 1
                    elif cls and cls[0] == "function":
                        emit(cls[1], "function", header_start, k)
                        header_start = k + 1
                    else:
                        header_start = k + 1
        elif ch == ";" and depth == 0:
            decl = " ".join(code[header_start:k].split())
            if decl and "(" not in decl and not decl.startswith(("typedef", "using", "extern", "friend")):
                names = IDENT.findall(decl.split("=")[0])
                if names and not re.match(r"^(struct|union|enum|class)\s+\w+$", decl):
                    emit(names[-1], "variable", header_start, k)
                    syms[-1].tags.append("static-global") if "const" not in decl else None
            elif decl.startswith("typedef") and "(" not in decl:
                names = IDENT.findall(decl)
                if names:
                    emit(names[-1], "typedef", header_start, k)
            header_start = k + 1
        k += 1
    del ns_stack
    return syms


def scan_clang(db_entry: dict[str, object], root: Path, lang: str = "c") -> list[Symbol] | None:
    """Parse one translation unit with clang's JSON AST; None if clang fails (caller falls back)."""
    args = db_entry.get("arguments") or shlex.split(str(db_entry.get("command", "")))
    assert isinstance(args, list)
    cmd = [str(a) for a in args if a not in ("-c",)]
    cmd = [a for i, a in enumerate(cmd) if not (a == "-o" or (i > 0 and cmd[i - 1] == "-o"))]
    directory = Path(str(db_entry.get("directory") or root))
    r = subprocess.run(
        [*cmd, "-fsyntax-only", "-Xclang", "-ast-dump=json"], capture_output=True, text=True, cwd=directory
    )
    if r.returncode != 0 or not r.stdout.strip():
        return None
    tree = json.loads(r.stdout)
    syms: list[Symbol] = []
    cur_file = ""

    def locate(node: dict[str, object]) -> tuple[str, int, int] | None:
        nonlocal cur_file
        rng = node.get("range")
        if not isinstance(rng, dict):
            return None
        loc = node.get("loc", {})
        for part in (loc, rng.get("begin", {}), rng.get("end", {})):
            if isinstance(part, dict) and "file" in part:
                cur_file = str(part["file"])
        b, e = rng.get("begin", {}), rng.get("end", {})
        if not isinstance(b, dict) or not isinstance(e, dict) or "line" not in b:
            return None
        return cur_file, int(b["line"]), int(e.get("line", b["line"]))

    for node in tree.get("inner", []):
        if not isinstance(node, dict):
            continue
        where = locate(node)
        if where is None:
            continue
        file, a, b = where
        try:
            rel = str((directory / file).resolve().relative_to(root.resolve()))
        except ValueError:
            continue  # system header or outside the source root
        kind_map = {
            "FunctionDecl": "function",
            "CXXRecordDecl": "record",
            "RecordDecl": "record",
            "EnumDecl": "enum",
            "TypedefDecl": "typedef",
            "VarDecl": "variable",
            "FunctionTemplateDecl": "function",
            "ClassTemplateDecl": "record",
        }
        k = kind_map.get(str(node.get("kind")))
        name = node.get("name")
        if k is None or not name:
            continue
        if k == "function" and not any(
            isinstance(c, dict) and c.get("kind") == "CompoundStmt" for c in node.get("inner", [])
        ):
            continue  # prototype
        if k == "record" and not node.get("completeDefinition"):
            continue
        text = (root / rel).read_text(errors="replace").split("\n")[a - 1 : b]
        span = "\n".join(text)
        tags = collect_tags(
            strip_comments_and_strings(span), k, [], lang, strip_comments_and_strings(span, keep_strings=True)
        )
        if node.get("kind", "").endswith("TemplateDecl") and "template" not in tags:
            tags.append("template")
        if k == "variable" and node.get("storageClass") == "static":
            tags.append("static-global")
        syms.append(Symbol(str(name), k, rel, a, b, tags))
    return syms


def source_files(root: Path, lang: str, exclude: tuple[str, ...] = ()) -> list[Path]:
    exts = C_EXT if lang == "c" else CPP_EXT
    skip = SKIP_DIRS | VENDORED_DIRS

    def kept(p: Path) -> bool:
        rel = p.relative_to(root)
        if skip & set(rel.parts) or rel.name in VENDORED_FILES:
            return False
        return not any(fnmatch.fnmatch(str(rel), g) for g in exclude)

    return sorted(p for p in root.rglob("*") if p.suffix in exts and p.is_file() and kept(p))


def overlaps(a: Symbol, b: Symbol) -> bool:
    return a.start <= b.end and b.start <= a.end


def merge_regex(clang_syms: list[Symbol], rx: list[Symbol]) -> list[Symbol]:
    """G14: add config-macro/macro-fn tags from the regex pass to clang symbols (same file, overlapping
    line span; same name preferred), add `#define` macros, and add regex symbols clang never saw
    (code disabled by `#if`)."""
    code = [r for r in rx if r.kind != "macro"]
    for s in clang_syms:
        cands = [r for r in code if overlaps(s, r)]
        same = [r for r in cands if r.name == s.name] or cands
        for r in same:
            for t in r.tags:
                if (t.startswith("config-macro:") or t == "macro-fn") and t not in s.tags:
                    s.tags.append(t)
    extra = [r for r in code if not any(overlaps(r, s) for s in clang_syms)]
    macros = [r for r in rx if r.kind == "macro"]
    return macros + sorted([*clang_syms, *extra], key=lambda s: (s.start, s.end))


TEST_LIKE_DIRS = {"test", "tests", "unittest", "unittests", "example", "examples", "sample", "samples", "demo", "bench"}
AST_OUTPUTS = ("ast-symbols.tsv", "calls.tsv", "facts.tsv")


def run_ast_facts(
    root: Path, files: list[str], ast_dir: Path, cxx: str, flags: tuple[str, ...]
) -> dict[str, list[tuple[str, str, int, int]]] | None:
    """Run ast_facts.py as a subprocess (scripts stay self-contained); {file: [(name, kind, start, end)]} or None."""
    for n in AST_OUTPUTS:
        (ast_dir / n).unlink(missing_ok=True)  # never leave a stale clang result beside a regex inventory
    if not files or shutil.which(cxx) is None:
        return None
    cmd = [sys.executable, "-I", str(Path(__file__).with_name("ast_facts.py")), str(root), "--out-dir", str(ast_dir)]
    cmd += ["--cxx", cxx]
    for f in files:
        cmd += ["--file", f]
    for fl in flags:
        cmd.append(f"--cxx-flag={fl}")  # `=` form: argparse refuses a separate value that starts with "-"
    r = subprocess.run(cmd, capture_output=True, text=True)
    sys.stderr.write(r.stderr)
    out = ast_dir / "ast-symbols.tsv"
    if r.returncode != 0 or not out.is_file():
        print(f"inventory: warning: ast_facts.py failed (exit {r.returncode}); using the regex pass", file=sys.stderr)
        return None
    got: dict[str, list[tuple[str, str, int, int]]] = {}
    for ln in out.read_text().splitlines():
        if ln.startswith("#") or ln.startswith("symbol\t"):
            continue
        name, kind, file, lines = ln.split("\t")
        a, b = lines.split("-")
        got.setdefault(file, []).append((name, kind, int(a), int(b)))
    return got


def ast_symbols(root: Path, rel: str, rows: list[tuple[str, str, int, int]], lang: str) -> list[Symbol]:
    text = (root / rel).read_text(errors="replace").split("\n")
    syms: list[Symbol] = []
    for name, kind, a, b in rows:
        span = "\n".join(text[a - 1 : b])
        tags = collect_tags(
            strip_comments_and_strings(span), kind, [], lang, strip_comments_and_strings(span, keep_strings=True)
        )
        syms.append(Symbol(name, kind, rel, a, b, tags))
    return syms


def inventory(
    root: Path,
    lang: str,
    db: Path | None,
    exclude: tuple[str, ...] = (),
    ast_dir: Path | None = None,
    cxx: str = "clang++",
    cxx_flags: tuple[str, ...] = (),
) -> tuple[list[Symbol], str]:
    syms: list[Symbol] = []
    engine = "regex"
    entries: dict[str, dict[str, object]] = {}
    if db and db.exists() and shutil.which("clang"):
        for e in json.loads(db.read_text()):
            entries[str(Path(str(e["directory"])) / str(e["file"]))] = e
    used_clang = False
    files = source_files(root, lang, exclude)
    ast_rows: dict[str, list[tuple[str, str, int, int]]] = {}
    if lang == "cpp" and ast_dir is not None:
        lib = [
            str(f.relative_to(root))
            for f in files
            if not (TEST_LIKE_DIRS & set(f.relative_to(root).parts[:-1]))
            and not f.stem.lower().startswith("test")
            and str(f.resolve()) not in entries
            and str(f) not in entries
        ]
        ast_rows = run_ast_facts(root, lib, ast_dir, cxx, cxx_flags) or {}
    used_ast = False
    for f in files:
        rel = str(f.relative_to(root))
        entry = entries.get(str(f.resolve())) or entries.get(str(f))
        got = scan_clang(entry, root, lang) if entry else None
        text = f.read_text(errors="replace")
        rx = scan_regex(text, rel, lang)
        if got is not None:
            used_clang = True
            syms += merge_regex([s for s in got if s.file == rel], rx)
        elif rel in ast_rows:
            used_ast = True
            syms += merge_regex(ast_symbols(root, rel, ast_rows[rel], lang), rx)
        else:
            syms += rx
    if used_clang:
        engine = "clang-ast+regex-fallback" if entries else "clang-ast"
    elif used_ast:
        engine = "clang-ast+regex"
    return syms, engine


def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("source_root")
    p.add_argument("--lang", choices=["c", "cpp"], required=True)
    p.add_argument("--compile-commands", help="path to compile_commands.json")
    p.add_argument("--out", default="migration/inventory.tsv")
    p.add_argument(
        "--exclude",
        action="append",
        default=[],
        metavar="GLOB",
        help="source-root-relative glob to leave out (repeatable); vendored dirs like fuzzing/ and unity/ are "
        "always skipped",
    )
    p.add_argument(
        "--cxx", default=os.environ.get("CXX", "clang++"), help="C++ compiler for the AST engine (--lang cpp)"
    )
    p.add_argument(
        "--cxx-flag",
        action="append",
        default=[],
        metavar="FLAG",
        help="extra flag for the AST engine, repeatable (for example -std=c++11, -include, locale)",
    )
    p.add_argument("--no-ast", action="store_true", help="--lang cpp: use the regex pass only")
    return p


def main(argv: list[str] | None = None) -> int:
    argv_list = sys.argv[1:] if argv is None else list(argv)
    cmd = shlex.join([Path(sys.argv[0]).name if argv is None else "inventory.py", *argv_list])
    args = build_parser().parse_args(argv_list)
    root = Path(args.source_root)
    if not root.is_dir():
        print(f"inventory: {root} is not a directory", file=sys.stderr)
        return 1
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    syms, engine = inventory(
        root,
        args.lang,
        Path(args.compile_commands) if args.compile_commands else None,
        tuple(args.exclude),
        None if args.no_ast else out.parent,
        args.cxx,
        tuple(args.cxx_flag),
    )
    rows = [f"{s.name}\t{s.kind}\t{s.file}\t{s.start}-{s.end}\t{','.join(dict.fromkeys(s.tags))}" for s in syms]
    out.write_text(f"# engine: {engine}\n# cmd: {cmd}\nsymbol\tkind\tfile\tlines\ttags\n" + "\n".join(rows) + "\n")
    print(f"inventory: {len(syms)} symbols, engine={engine} -> {out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
