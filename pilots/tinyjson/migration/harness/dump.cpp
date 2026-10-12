// Differential driver for tinyjson (ticket 02, judge.md section 2).
//
// Reads one input file (the LAST argument) and prints one canonical text dump of everything observable. The Rust
// driver (examples/dump.rs) must be a line-for-line port: same section order, same labels, same helpers.
//
// Output grammar (one record per line):
//   <label> = <value>                    a call that returned
//   <label> ! <class> <quoted what()>    a call that threw (class: out_of_range, invalid_argument, range_error,
//                                        runtime_error, bad_alloc, exception, unknown)
// Strings are printed as "..." with \\ \" escaped and every byte outside 0x20..0x7e as \xHH. Integers are %lld.
// A double is printed as its 16-digit IEEE bit pattern (0x%016llx), a NaN as "nan". Nothing else is printed for
// doubles: where the library itself prints one (to_string) the library text is printed as is.
//
// Built at -O0 (D3: parse_hex leaves an uninitialised value; see pin_d3_value). Exit 0 after a normal run, 2 for a driver error.
//
// Sections: parse (parser::parse), node (every node of the parsed value), roundtrip (to_string, re-parse, ==),
// copy (copy constructor and assignment), streams (U8ToU32 / U32ToU8 and every public parser function on a
// std::u32string stream), build (values built through constructors, add_member, add_element), matrix (fixed
// equality matrix).

#include "tinyjson.h"

#include <cmath>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <string>
#include <unistd.h>
#include <utility>
#include <vector>
#include <mach-o/dyld.h>

using namespace tinyjson;

// ---------------------------------------------------------------------------------------------------------------
// printing helpers (one per kind of value)
// ---------------------------------------------------------------------------------------------------------------

static std::string num(long long v)
{
    char buf[40];
    snprintf(buf, sizeof buf, "%lld", v);
    return buf;
}

static std::string q(const std::string& s)
{
    std::string r = "\"";
    for (size_t i = 0; i < s.size(); i++)
    {
        unsigned char c = (unsigned char)s[i];
        if (c == '\\')
            r += "\\\\";
        else if (c == '"')
            r += "\\\"";
        else if (c >= 0x20 && c <= 0x7e)
            r += (char)c;
        else
        {
            char buf[8];
            snprintf(buf, sizeof buf, "\\x%02x", c);
            r += buf;
        }
    }
    return r + "\"";
}

static std::string dbl(double d)
{
    if (std::isnan(d))
        return "nan";
    uint64_t bits;
    memcpy(&bits, &d, sizeof bits);
    char buf[24];
    snprintf(buf, sizeof buf, "0x%016llx", (unsigned long long)bits);
    return buf;
}

static std::string code(unsigned int c)
{
    if (c == 0xFFFFFFFFu)
        return "EOF";
    char buf[16];
    snprintf(buf, sizeof buf, "U+%04X", c);
    return buf;
}

static void line(const std::string& label, const std::string& value)
{
    printf("%s = %s\n", label.c_str(), value.c_str());
}

static void fail(const std::string& label, const char* cls, const char* what)
{
    printf("%s ! %s %s\n", label.c_str(), cls, q(what).c_str());
}

// Runs f (returns the printed value); prints "= value" or "! class what".
template <typename F>
static void attempt(const std::string& label, F f)
{
    try
    {
        line(label, f());
    }
    catch (std::out_of_range& e)
    {
        fail(label, "out_of_range", e.what());
    }
    catch (std::invalid_argument& e)
    {
        fail(label, "invalid_argument", e.what());
    }
    catch (std::range_error& e)
    {
        fail(label, "range_error", e.what());
    }
    catch (std::runtime_error& e)
    {
        fail(label, "runtime_error", e.what());
    }
    catch (std::bad_alloc& e)
    {
        fail(label, "bad_alloc", e.what());
    }
    catch (std::exception& e)
    {
        fail(label, "exception", e.what());
    }
    catch (...)
    {
        fail(label, "unknown", "");
    }
}

static std::string kind(const json& j)
{
    return num(j.type()) + " " + j.type_name();
}

// ---------------------------------------------------------------------------------------------------------------
// equality guard: operator== is UB for the pairs of D1 (the Rust port returns false there), so the driver only
// calls it on pairs where the C++ result is defined (or D2, which the port reproduces).
// ---------------------------------------------------------------------------------------------------------------

static int rank_of(json_t t)
{
    switch (t)
    {
        case json_t::array: return 0;
        case json_t::object: return 1;
        case json_t::null: return 2;
        case json_t::string: return 3;
        case json_t::boolean: return 4;
        case json_t::number_integer: return 5;
        default: return 6;
    }
}

static bool eq_safe(json& a, json& b)
{
    json_t ta = a.type();
    json_t tb = b.type();
    if (ta == tb)
    {
        if (ta == json_t::array)
        {
            json_array x = a.get_array();
            json_array y = b.get_array();
            if (x.size() != y.size())
                return true;
            for (size_t i = 0; i < x.size(); i++)
                if (!eq_safe(x[i], y[i]))
                    return false;
            return true;
        }
        if (ta == json_t::object)
        {
            json_object x = a.get_object();
            json_object y = b.get_object();
            if (x.size() != y.size())
                return true;
            auto ix = x.begin();
            auto iy = y.begin();
            for (; ix != x.end(); ++ix, ++iy)
            {
                if (ix->first != iy->first)
                    return true;
                if (!eq_safe(ix->second, iy->second))
                    return false;
            }
            return true;
        }
        return true;
    }
    if (rank_of(ta) > rank_of(tb))
        return true;
    if ((ta == json_t::array || ta == json_t::object) && tb == json_t::null)
        return true;
    if (ta == json_t::number_integer && tb == json_t::number_double)
        return true;
    return false;
}

// "eq" record for (a, b): == both ways and !=, or "skip" for the pairs of D1.
static void eq_line(const std::string& label, json& a, json& b)
{
    if (!eq_safe(a, b) || !eq_safe(b, a))
    {
        line(label, "skip");
        return;
    }
    bool ab = (a == b);
    bool ba = (b == a);
    bool ne = (a != b);
    line(label, "ab=" + num(ab) + " ba=" + num(ba) + " ne=" + num(ne));
}

// ---------------------------------------------------------------------------------------------------------------
// section node: everything observable on one value, then its children
// ---------------------------------------------------------------------------------------------------------------

static std::string path_key(const std::string& path, const std::string& key)
{
    return path + "[" + q(key) + "]";
}

static std::string path_index(const std::string& path, size_t i)
{
    return path + "[" + num((long long)i) + "]";
}

static void dump_node(json& n, const std::string& p)
{
    line(p + ".type", kind(n));

    attempt(p + ".get_string", [&] { return q(n.get_string()); });
    attempt(p + ".get_integer", [&] { return num(n.get_integer()); });
    attempt(p + ".get_double", [&] { return dbl(n.get_double()); });
    attempt(p + ".get_bool", [&] { return num(n.get_bool()); });
    attempt(p + ".get_null", [&] { return std::string(n.get_null() == nullptr ? "nullptr" : "other"); });
    attempt(p + ".get_object", [&] { return "size=" + num((long long)n.get_object().size()); });
    attempt(p + ".get_array", [&] { return "size=" + num((long long)n.get_array().size()); });
    attempt(p + ".size", [&] { return num((long long)n.size()); });

    attempt(p + ".conv_string", [&] { return q(static_cast<std::string>(n)); });
    attempt(p + ".conv_double", [&] { return dbl(static_cast<double>(n)); });
    attempt(p + ".conv_long_long", [&] { return num(static_cast<long long>(n)); });
    attempt(p + ".conv_bool", [&] { return num(static_cast<bool>(n)); });

    attempt(p + ".to_string", [&] { return q(n.to_string()); });

    // lookups by key and index (object, array and every other type)
    attempt(p + ".has_member(absent)", [&] { return num(n.has_member("\x01 absent")); });
    attempt(p + ".has_member(empty)", [&] { return num(n.has_member("")); });
    attempt(p + ".op[absent]", [&] { return kind(n["\x01 absent"]); });
    attempt(p + ".op[-1]", [&] { return kind(n[-1]); });
    attempt(p + ".op[0]", [&] { return kind(n[0]); });
    attempt(p + ".op[1]", [&] { return kind(n[1]); });
    if (n.type() == json_t::array)
    {
        long long sz = (long long)n.size();
        attempt(p + ".op[size-1]", [&] { return kind(n[(int)(sz - 1)]); });
        attempt(p + ".op[size]", [&] { return kind(n[(int)sz]); });
        attempt(p + ".op[size+1]", [&] { return kind(n[(int)(sz + 1)]); });
    }
    if (n.type() == json_t::object)
    {
        json_object m = n.get_object();
        for (auto it = m.begin(); it != m.end(); ++it)
        {
            std::string kp = path_key(p, it->first);
            attempt(kp + ".has_member", [&] { return num(n.has_member(it->first)); });
            attempt(kp + ".op[c_str]", [&] { return kind(n[it->first.c_str()]); });
        }
    }

    // copy, equality with the copy, then the mutators on the copy (the original stays untouched)
    json c(n);
    eq_line(p + ".copy_eq", n, c);
    attempt(p + ".copy.add_member", [&] { c.add_member("k", json(1)); return q(c.to_string()); });
    json c2(n);
    attempt(p + ".copy.add_element", [&] { c2.add_element(json("e")); return q(c2.to_string()); });
    json c3(n);
    attempt(p + ".copy.add_member_overwrite", [&] {
        c3.add_member("", json(true));
        c3.add_member("", json(false));
        return q(c3.to_string());
    });

    // children
    if (n.type() == json_t::array)
    {
        json_array a = n.get_array();
        for (size_t i = 0; i < a.size(); i++)
            dump_node(a[i], path_index(p, i));
    }
    if (n.type() == json_t::object)
    {
        json_object m = n.get_object();
        for (auto it = m.begin(); it != m.end(); ++it)
            dump_node(it->second, path_key(p, it->first));
    }
}

// ---------------------------------------------------------------------------------------------------------------
// section streams
// ---------------------------------------------------------------------------------------------------------------

static std::string u32_text(const std::u32string& s)
{
    std::string r = "[";
    for (size_t i = 0; i < s.size(); i++)
    {
        if (i)
            r += " ";
        r += code((unsigned int)s[i]);
    }
    return r + "]";
}

static void after(const std::string& label, u32_sstream& ss)
{
    ss.clear();
    long long pos = (long long)ss.tellg();
    ss.clear();
    unsigned int next = (unsigned int)ss.peek();
    line(label + ".after", "pos=" + num(pos) + " next=" + code(next));
}

static void probe_json(const std::string& label, const std::u32string& src, json (*fn)(u32_istream&))
{
    u32_sstream ss(src);
    attempt(label, [&] {
        json r = fn(ss);
        return kind(r) + " " + q(r.to_string());
    });
    after(label, ss);
}

static void probe_string(const std::string& label, const std::u32string& src, std::string (*fn)(u32_istream&))
{
    u32_sstream ss(src);
    attempt(label, [&] { return q(fn(ss)); });
    after(label, ss);
}

static void probe_char(const std::string& label, const std::u32string& src, char32_t (*fn)(u32_istream&))
{
    u32_sstream ss(src);
    attempt(label, [&] { return code((unsigned int)fn(ss)); });
    after(label, ss);
}

static void probe_int(const std::string& label, const std::u32string& src, int (*fn)(u32_istream&))
{
    u32_sstream ss(src);
    attempt(label, [&] { return code((unsigned int)fn(ss)); });
    after(label, ss);
}

static void probe_void(const std::string& label, const std::u32string& src, void (*fn)(u32_istream&))
{
    u32_sstream ss(src);
    attempt(label, [&] {
        fn(ss);
        return std::string("void");
    });
    after(label, ss);
}

static void skip_quote(u32_istream& s) { parser::skip_char(s, U'"'); }
static void skip_colon(u32_istream& s) { parser::skip_char(s, U':'); }

static void stream_probes(const std::string& tag, const std::u32string& src)
{
    line(tag + ".src", u32_text(src));
    probe_json(tag + ".parse_value", src, parser::parse_value);
    probe_json(tag + ".parse_object", src, parser::parse_object);
    probe_json(tag + ".parse_array", src, parser::parse_array);
    probe_json(tag + ".parse_string", src, parser::parse_string);
    probe_json(tag + ".parse_number", src, parser::parse_number);
    probe_json(tag + ".parse_bool", src, parser::parse_bool);
    probe_json(tag + ".parse_null", src, parser::parse_null);
    probe_string(tag + ".parse_member", src, parser::parse_member);
    probe_char(tag + ".escape_char", src, parser::escape_char);
    probe_char(tag + ".parse_hex", src, parser::parse_hex);
    probe_char(tag + ".get_next_non_space", src, parser::get_next_non_space);
    probe_int(tag + ".peek_next_non_space", src, parser::peek_next_non_space);
    probe_void(tag + ".skip_space", src, parser::skip_space);
    probe_void(tag + ".skip_char(quote)", src, skip_quote);
    probe_void(tag + ".skip_char(colon)", src, skip_colon);
}

static void section_streams(const std::string& data)
{
    std::u32string utf8;
    bool utf8_ok = false;
    attempt("streams.U8ToU32", [&] {
        utf8 = U8ToU32(data);
        utf8_ok = true;
        return u32_text(utf8);
    });
    if (utf8_ok)
    {
        attempt("streams.U32ToU8(utf8)", [&] { return q(U32ToU8(utf8)); });
        stream_probes("streams.utf8", utf8);
    }

    // every byte as one code point (reaches 0x80..0xFF in the u32 parsers without the UTF-8 decoder)
    std::u32string latin1;
    for (size_t i = 0; i < data.size(); i++)
        latin1.push_back((char32_t)(unsigned char)data[i]);
    attempt("streams.U32ToU8(latin1)", [&] { return q(U32ToU8(latin1)); });
    stream_probes("streams.latin1", latin1);

    // 4 bytes little endian per code point (reaches surrogates and values above 0x10FFFF)
    std::u32string wide;
    for (size_t i = 0; i + 4 <= data.size(); i += 4)
    {
        uint32_t v = 0;
        for (int k = 0; k < 4; k++)
            v |= ((uint32_t)(unsigned char)data[i + k]) << (8 * k);
        wide.push_back((char32_t)v);
    }
    line("streams.wide", u32_text(wide));
    attempt("streams.U32ToU8(wide)", [&] { return q(U32ToU8(wide)); });

    // scalar helpers on the whole input (they take std::string by value)
    attempt("streams.to_bool", [&] { return num(parser::to_bool(data)); });
    attempt("streams.to_integer", [&] { return num(parser::to_integer(data)); });
    attempt("streams.to_double", [&] { return dbl(parser::to_double(data)); });
    attempt("streams.trim", [&] { return q(trim(std::string(data))); });
}

// ---------------------------------------------------------------------------------------------------------------
// section build: construction API driven from the input bytes
// ---------------------------------------------------------------------------------------------------------------

static json make_elem(unsigned char b)
{
    switch (b % 7)
    {
        case 0: return json(std::string(1, (char)b));
        case 1: return json((int)b);
        case 2: return json((double)b / 8.0 - 10.0);
        case 3: return json((bool)(b & 1));
        case 4: return json();
        case 5: return json(json_array{});
        default: return json(json_object{});
    }
}

static long long le_bits(const std::string& d, size_t n)
{
    unsigned long long v = 0;
    for (size_t i = 0; i < n && i < d.size(); i++)
        v |= ((unsigned long long)(unsigned char)d[i]) << (8 * i);
    return (long long)v;
}

static void section_build(const std::string& data)
{
    // scalars
    // (an lvalue std::string is ambiguous between json(std::string&) and json(std::string): nothing to port)
    json js{std::string(data)};
    dump_node(js, "build.string");
    json jr(std::string(data.rbegin(), data.rend()));
    dump_node(jr, "build.string_rvalue");
    std::vector<char> buf(data.begin(), data.end());
    buf.push_back('\0');
    json jc(buf.data());
    dump_node(jc, "build.char_array");

    json ji((int)le_bits(data, 4));
    dump_node(ji, "build.int");
    json jl((long long)le_bits(data, 8));
    dump_node(jl, "build.long_long");
    long long raw = le_bits(data, 8);
    double dv;
    memcpy(&dv, &raw, sizeof dv);
    json jd(dv);
    dump_node(jd, "build.double");
    json jb((bool)(data.size() > 0 && (data[0] & 1)));
    dump_node(jb, "build.bool");
    json jn;
    dump_node(jn, "build.null");

    // array of one element per byte (first 12 bytes), by constructor and by add_element
    size_t n = data.size() < 12 ? data.size() : 12;
    json_array arr;
    for (size_t i = 0; i < n; i++)
        arr.push_back(make_elem((unsigned char)data[i]));
    json ja(arr);
    dump_node(ja, "build.array_lvalue");
    json_array arr_copy(arr);
    json ja2(std::move(arr_copy));
    line("build.array_rvalue.to_string", q(ja2.to_string()));
    json jb2(json_array{});
    for (size_t i = 0; i < n; i++)
        jb2.add_element(make_elem((unsigned char)data[i]));
    dump_node(jb2, "build.array_add_element");
    eq_line("build.array_eq", ja, jb2);

    // object: key = one byte (so NUL and high bytes occur), value = element of the next byte; duplicates overwrite
    json_object obj;
    for (size_t i = 0; i + 1 < n; i++)
        obj[std::string(1, data[i])] = make_elem((unsigned char)data[i + 1]);
    json jo(obj);
    dump_node(jo, "build.object_lvalue");
    json jo2(json_object{});
    for (size_t i = 0; i + 1 < n; i++)
        jo2.add_member(std::string(1, data[i]), make_elem((unsigned char)data[i + 1]));
    dump_node(jo2, "build.object_add_member");
    eq_line("build.object_eq", jo, jo2);

    // nesting through references: operator[] then a mutator on the result
    json_object doc_members;
    json doc(doc_members);
    doc.add_member("arr", ja);
    doc.add_member("obj", jo2);
    attempt("build.nested.arr_add", [&] {
        doc["arr"].add_element(json(data));
        return q(doc.to_string());
    });
    attempt("build.nested.obj_add", [&] {
        doc["obj"].add_member(data, json((int)data.size()));
        return q(doc.to_string());
    });
    attempt("build.nested.index", [&] {
        doc["arr"][0] = json(std::string("replaced"));
        return q(doc.to_string());
    });
    dump_node(doc, "build.nested");

    // get_object / get_array return copies (Q19)
    attempt("build.copy_object", [&] {
        json_object m = jo.get_object();
        m["x"] = json(1);
        return "copy_size=" + num((long long)m.size()) + " orig_size=" + num((long long)jo.size());
    });
    attempt("build.copy_array", [&] {
        json_array a = ja.get_array();
        a.push_back(json(1));
        return "copy_size=" + num((long long)a.size()) + " orig_size=" + num((long long)ja.size());
    });

    // type-error paths on the built values
    attempt("build.add_member_on_array", [&] {
        ja.add_member("k", json(1));
        return std::string("ok");
    });
    attempt("build.add_element_on_object", [&] {
        jo.add_element(json(1));
        return std::string("ok");
    });
    attempt("build.size_on_string", [&] { return num((long long)js.size()); });
}

// ---------------------------------------------------------------------------------------------------------------
// section matrix: fixed values, every ordered pair (independent of the input)
// ---------------------------------------------------------------------------------------------------------------

static void section_matrix()
{
    std::vector<json> v;
    std::vector<std::string> names;
    v.push_back(json());                               names.push_back("null");
    v.push_back(json(true));                           names.push_back("true");
    v.push_back(json(false));                          names.push_back("false");
    v.push_back(json(0));                              names.push_back("int0");
    v.push_back(json(1));                              names.push_back("int1");
    v.push_back(json(-1));                             names.push_back("int-1");
    v.push_back(json(0.0));                            names.push_back("dbl0");
    v.push_back(json(-0.0));                           names.push_back("dbl-0");
    v.push_back(json(1.5));                            names.push_back("dbl1.5");
    v.push_back(json(std::nan("")));                   names.push_back("nan");
    v.push_back(json(std::string("")));                names.push_back("str-empty");
    v.push_back(json(std::string("a")));               names.push_back("str-a");
    v.push_back(json(std::string("b")));               names.push_back("str-b");
    v.push_back(json(json_array{}));                   names.push_back("arr-empty");
    v.push_back(json(json_array{json(1)}));            names.push_back("arr-1");
    v.push_back(json(json_array{json(1.0)}));          names.push_back("arr-1.0");
    v.push_back(json(json_array{json(std::string("a")), json()})); names.push_back("arr-a-null");
    json_object o1;
    o1["a"] = json(1);
    v.push_back(json(json_object{}));                  names.push_back("obj-empty");
    v.push_back(json(o1));                             names.push_back("obj-a1");
    for (size_t i = 0; i < v.size(); i++)
        for (size_t j = 0; j < v.size(); j++)
            eq_line("matrix." + names[i] + "." + names[j], v[i], v[j]);
}

// ---------------------------------------------------------------------------------------------------------------
// D3 pinning (driver only, not part of the dump, nothing to port)
//
// parse_hex leaves `uc` uninitialised. Measured at -O0: the value is constant inside one process and every \u
// context, but differs between processes (100 runs: U+0001 in 71, U+0007 5, U+0008 4, U+0009 6, U+000A 5,
// U+000B 7, U+000C 2). So an input with \u gives two different dumps in two runs. The driver decodes
// "\u0041" once at start; when the value is not the modal U+0001 it re-executes itself (a fresh process draws
// again), at most 60 times. The exec counter is the environment variable TINYJSON_DUMP_TRY.
// ---------------------------------------------------------------------------------------------------------------

static void pin_d3_value(char** argv)
{
    json probe = parser::parse("[\"\\u0041\"]");
    unsigned char v = (unsigned char)probe[0].get_string()[0];
    if (v == 1)
        return;
    const char* t = getenv("TINYJSON_DUMP_TRY");
    int tries = t ? atoi(t) : 0;
    if (tries >= 60)
        return;
    char tb[16];
    snprintf(tb, sizeof tb, "%d", tries + 1);
    setenv("TINYJSON_DUMP_TRY", tb, 1);
    char path[4096];
    uint32_t size = sizeof path;
    if (_NSGetExecutablePath(path, &size) != 0)
        return;
    execv(path, argv);
}

// ---------------------------------------------------------------------------------------------------------------
// main
// ---------------------------------------------------------------------------------------------------------------

int main(int argc, char** argv)
{
    if (argc < 2)
    {
        fprintf(stderr, "usage: dump <input-file>\n");
        return 2;
    }
    pin_d3_value(argv);
    std::ifstream in(argv[argc - 1], std::ios::binary);
    if (!in)
    {
        fprintf(stderr, "cannot open %s\n", argv[argc - 1]);
        return 2;
    }
    std::string data((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());

    line("input.size", num((long long)data.size()));

    // parse (reads its const char* up to the first NUL, Q10)
    json parsed;
    bool ok = false;
    attempt("parse", [&] {
        parsed = parser::parse(data.c_str());
        ok = true;
        return kind(parsed);
    });

    if (ok)
    {
        dump_node(parsed, "$");

        // roundtrip
        std::string ser = parsed.to_string();
        line("roundtrip.ser", q(ser));
        json again;
        bool ok2 = false;
        attempt("roundtrip.parse", [&] {
            again = parser::parse(ser.c_str());
            ok2 = true;
            return kind(again);
        });
        if (ok2)
        {
            line("roundtrip.ser2", q(again.to_string()));
            eq_line("roundtrip.eq", parsed, again);
        }

        // copy constructor and assignment
        json copy(parsed);
        eq_line("copy.eq", parsed, copy);
        line("copy.ser", q(copy.to_string()));
        json assigned(5);
        assigned = parsed;
        eq_line("assign.eq", parsed, assigned);
        line("assign.ser", q(assigned.to_string()));
        json chain;
        chain = assigned = copy;
        eq_line("chain.eq", parsed, chain);
        json nulled(parsed);
        nulled = json();
        line("assign_null", kind(nulled) + " " + q(nulled.to_string()));
        eq_line("assign_null.eq", parsed, nulled);
    }

    section_streams(data);
    section_build(data);
    section_matrix();
    return 0;
}
