"""C++ inventory fixes reported by the tinyjson run: operator names, vendored test headers, dead-code roots."""

from __future__ import annotations

from pathlib import Path

import depmap
import inventory

CPP = """\
namespace json {
class value;
}
json::value::operator bool() const { return true; }
bool json::value::operator==(const value &o) const { return false; }
value &json::value::operator[](int i) { return *this; }
json::value::operator std::string() const { return ""; }
int plain(int x) { return x; }
"""


def test_operator_name_forms() -> None:
    assert inventory.operator_name("explicit json::operator bool", ")") == "json::operator bool"
    assert inventory.operator_name("bool json::operator==", ")(") == "json::operator=="
    assert inventory.operator_name("T& A::operator", ")(int)") == "A::operator()"
    assert inventory.operator_name("int plain", ")") is None


def test_regex_engine_names_operators_not_their_types() -> None:
    names = {s.name for s in inventory.scan_regex(CPP, "a.cpp", "cpp") if s.kind == "function"}
    assert "json::value::operator bool" in names
    assert "json::value::operator==" in names
    assert "json::value::operator std::string" in names
    assert "plain" in names
    assert not names & {"bool", "std::string", "json::value::operator"}


def test_vendored_test_header_is_skipped_but_tests_are_not(tmp_path: Path) -> None:
    (tmp_path / "test").mkdir()
    (tmp_path / "test" / "catch.hpp").write_text("int f() { return 1; }\n")
    (tmp_path / "test" / "tests.cpp").write_text("int g() { return 1; }\n")
    (tmp_path / "lib.hpp").write_text("int h() { return 1; }\n")
    found = {p.name for p in inventory.source_files(tmp_path, "cpp")}
    assert found == {"tests.cpp", "lib.hpp"}


def test_example_files_are_dead_code_roots() -> None:
    syms = [
        ("api", "function", "lib.hpp", 1, 2, ""),
        ("main", "function", "samples/main.cpp", 1, 5, ""),
        ("orphan", "function", "lib.hpp", 3, 4, ""),
    ]
    edges = [("main@samples/main.cpp", "api@lib.hpp", "call")]
    assert depmap.dead_candidates(syms, edges) == ["orphan@lib.hpp"]


def test_qualified_method_is_reached_by_its_bare_call_name(tmp_path: Path) -> None:
    (tmp_path / "lib.hpp").write_text("int json::get() { return 1; }\nint json::unused() { return 2; }\n")
    (tmp_path / "test").mkdir()
    (tmp_path / "test" / "t.cpp").write_text("int check() { return x.get(); }\n")
    syms = [
        ("json::get", "function", "lib.hpp", 1, 1, ""),
        ("json::unused", "function", "lib.hpp", 2, 2, ""),
        ("check", "function", "test/t.cpp", 1, 1, ""),
    ]
    _, _, dead = depmap.build(tmp_path, syms)
    assert dead == ["json::unused@lib.hpp"]


def test_macro_wrapped_return_type_names_the_function_not_the_macro() -> None:
    code = "CJSON_PUBLIC(cJSON *) cJSON_Parse(const char *value)\\n{\\n  return 0;\\n}\\n".replace("\\n", "\n")
    names = [s.name for s in inventory.scan_regex(code, "c.c", "c") if s.kind == "function"]
    assert names == ["cJSON_Parse"]


def test_constructor_initialiser_list_is_not_a_second_declarator() -> None:
    code = "json::json(int x) : _type(a), _value(b)\n{\n}\n"
    names = [s.name for s in inventory.scan_regex(code, "a.cpp", "cpp") if s.kind == "function"]
    assert names == ["json::json"]
