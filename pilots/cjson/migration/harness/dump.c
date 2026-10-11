/* Differential driver for cJSON + cJSON_Utils (ticket 02, judge.md §2).
 *
 * Reads one input file (path = last argument) and prints a canonical text dump of everything observable.
 * Built twice from this one source: against the original cJSON.c/cJSON_Utils.c (built with the feature set's
 * defines), and against the Rust staticlib (public headers only). It uses only the public API; nothing here
 * prints an address, a timing or anything non-deterministic: the error position is an offset into the input.
 *
 * Input format: one JSON text "A", or "A" + "\n===\n" + "B". With one text, B is a copy of A. A and B are
 * the documents of the pair operations (patches, merge patches, compare); B doubles as the patch array for
 * ApplyPatches and the merge patch for MergePatch.
 *
 * Modes, all in one run (so one input covers many paths):
 *   M1 parse (Parse, ParseWithOpts x2, ParseWithLength x4), tree dump with every field
 *   M2 print: formatted, unformatted, PrintBuffered (prebuffer 0 = D15, 1, 256, formatted and not),
 *      PrintPreallocated at three buffer sizes, Duplicate, Compare
 *   M3 lookups and mutations: array/object accessors, detach/replace/insert/delete, Minify
 *   M4 allocator hooks: counting hooks (allocations, frees, bytes) and a fail-the-Nth-allocation sweep
 *      over Parse, Print, PrintBuffered and Duplicate, with the allocations left alive after each (D1: every
 *      allocation guard frees what it built)
 *   M5 cJSON_Utils: pointers, GeneratePatches/ApplyPatches, MergePatch, GenerateMergePatch, SortObject
 *   M6 locale: LC_NUMERIC set to a comma locale if the OS has one (D3); LC_CTYPE set to a Latin-1 locale and
 *      keys with bytes >= 0x80 looked up case-insensitively (D19)
 *   M7 fixed constructors: numbers incl. NaN (D6), strings, references, Add*ToObject; writes through the
 *      char * the library returns (D21) and into valueint/valuedouble (Q3)
 */
#include <ctype.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "cJSON.h"
#include "cJSON_Utils.h"

#define MAXFAIL 48

static const char *g_base; /* start of the text currently parsed, for error offsets */

/* ---- printing helpers ---------------------------------------------------------------------------------- */

static void put_str(const char *s)
{
    if (s == NULL)
    {
        printf("NULL");
        return;
    }
    putchar('"');
    for (const unsigned char *p = (const unsigned char *)s; *p; p++)
    {
        if (*p == '"' || *p == '\\')
            printf("\\%c", *p);
        else if (*p < 0x20 || *p >= 0x7f)
            printf("\\x%02x", *p);
        else
            putchar(*p);
    }
    putchar('"');
}

static void put_line(const char *label, const char *s)
{
    printf("%s=", label);
    put_str(s);
    putchar('\n');
}

static void put_err(void)
{
    const char *e = cJSON_GetErrorPtr();
    if (e == NULL)
        printf("err=-1\n");
    else if (g_base != NULL && e >= g_base)
        printf("err=%ld\n", (long)(e - g_base));
    else
        printf("err=outside\n");
}

static void dump_tree(const cJSON *n, int depth)
{
    for (; n != NULL; n = n->next)
    {
        const cJSON *last = n->child;
        int lastok = 1;
        if (last != NULL)
        {
            while (last->next != NULL)
                last = last->next;
            lastok = (n->child->prev == last);
        }
        printf("%*stype=%d vi=%d vd=%.17g key=", depth * 2 + 2, "", n->type, n->valueint, n->valuedouble);
        put_str(n->string);
        printf(" vs=");
        put_str(n->valuestring);
        printf(" first_prev_is_last=%d prev_null=%d\n", lastok, n->prev == NULL);
        if (n->child != NULL)
            dump_tree(n->child, depth + 1);
    }
}

static void show(const char *label, cJSON *item)
{
    char *s = cJSON_PrintUnformatted(item);
    put_line(label, s);
    cJSON_free(s);
}

static void show_fmt(const char *label, cJSON *item)
{
    char *s = cJSON_Print(item);
    put_line(label, s);
    cJSON_free(s);
}

/* ---- counting / failing allocator hooks ---------------------------------------------------------------- */

static long n_alloc, n_free, n_bytes, fail_at;

/* allocation trace ("a<size>" per allocation, "f" per free), switched on for the Utils allocator section only;
 * capped at TRACE_MAX bytes, the same cap and format as dump.rs */
#define TRACE_MAX 4000
static char trace_buf[TRACE_MAX + 16];
static size_t trace_len;
static int trace_on;
static int utils_probe;  /* input is judge-utils-oom: the fixed allocator probes run */
static int utils_subset; /* input is patch-* or judge-*: Utils allocator section and link dumps after sorts */

static void trace_add(const char *s)
{
    size_t n = strlen(s);
    if (trace_len + n <= TRACE_MAX)
    {
        memcpy(trace_buf + trace_len, s, n);
        trace_len += n;
        trace_buf[trace_len] = 0;
    }
}

static void *h_malloc(size_t n)
{
    n_alloc++;
    if (trace_on)
    {
        char t[40];
        sprintf(t, " a%lu", (unsigned long)n);
        trace_add(t);
    }
    if (fail_at != 0 && n_alloc == fail_at)
        return NULL;
    n_bytes += (long)n;
    return malloc(n);
}

static void h_free(void *p)
{
    if (p != NULL)
    {
        n_free++;
        if (trace_on)
            trace_add(" f");
    }
    free(p);
}

static void hooks_on(long failing)
{
    cJSON_Hooks h;
    h.malloc_fn = h_malloc;
    h.free_fn = h_free;
    n_alloc = n_free = n_bytes = 0;
    fail_at = failing;
    cJSON_InitHooks(&h);
}

/* allocations still alive after a fail-the-Nth run (the failed allocation is not one); 0 = nothing leaked (D1) */
static long live_now(void)
{
    return (n_alloc - (fail_at != 0 && fail_at <= n_alloc ? 1 : 0)) - n_free;
}

static void hooks_off(void)
{
    cJSON_InitHooks(NULL);
    fail_at = 0;
}

/* cJSON_Compare on objects recurses a->b and b->a at every level: 2^depth calls (upstream behaviour, measured:
 * a 999-deep object did not finish in 100 s). Compare is only called below this depth. */
#define COMPARE_MAX_DEPTH 14

static int tree_depth(const cJSON *n)
{
    int d = 0;
    for (; n != NULL; n = n->next)
    {
        int c = tree_depth(n->child) + 1;
        if (c > d)
            d = c;
    }
    return d;
}

/* ---- M1/M2: parse and print ---------------------------------------------------------------------------- */

static void print_modes(const char *tag, cJSON *root)
{
    char *s;
    char buf[64];
    printf("-- print %s\n", tag);
    show_fmt("formatted", root);
    show("unformatted", root);
    for (int fmt = 0; fmt <= 1; fmt++)
    {
        static const int pre[] = {0, 1, 256};
        for (int i = 0; i < 3; i++)
        {
            s = cJSON_PrintBuffered(root, pre[i], fmt);
            snprintf(buf, sizeof buf, "buffered[pre=%d,fmt=%d]", pre[i], fmt);
            put_line(buf, s);
            cJSON_free(s);
        }
    }
    s = cJSON_PrintUnformatted(root);
    if (s != NULL)
    {
        int need = (int)strlen(s) + 1;
        static const int extra[] = {0, 5, 64};
        for (int i = 0; i < 3; i++)
        {
            int len = need + extra[i];
            char *b = calloc((size_t)len + 1, 1);
            cJSON_bool ok = cJSON_PrintPreallocated(root, b, len, 0);
            printf("prealloc[len=need+%d,fmt=0] ok=%d ", extra[i], (int)ok);
            put_str(b);
            putchar('\n');
            free(b);
        }
        for (int i = 0; i < 2; i++)
        {
            int len = need - 1 - i; /* too small */
            if (len < 0)
                len = 0;
            char *b = calloc((size_t)len + 1, 1);
            cJSON_bool ok = cJSON_PrintPreallocated(root, b, len, 1);
            printf("prealloc[len=need-%d,fmt=1] ok=%d ", 1 + i, (int)ok);
            put_str(b);
            putchar('\n');
            free(b);
        }
        cJSON_free(s);
    }
}

static void parse_modes(const char *label, const char *text, size_t len)
{
    cJSON *root;
    const char *end;
    size_t lens[4];
    printf("== parse %s len=%zu\n", label, len);
    g_base = text;
    root = cJSON_Parse(text);
    printf("Parse root=%s ", root ? "ok" : "NULL");
    put_err();
    if (root != NULL)
        dump_tree(root, 0);
    cJSON_Delete(root);

    for (int req = 0; req <= 1; req++)
    {
        end = NULL;
        root = cJSON_ParseWithOpts(text, &end, req);
        printf("ParseWithOpts[req=%d] root=%s end=%ld ", req, root ? "ok" : "NULL", end ? (long)(end - text) : -1L);
        put_err();
        if (root != NULL)
            show("  unformatted", root);
        cJSON_Delete(root);
    }
    lens[0] = len;
    lens[1] = len / 2;
    lens[2] = len > 0 ? len - 1 : 0;
    lens[3] = 0;
    for (int i = 0; i < 4; i++)
    {
        end = NULL;
        root = cJSON_ParseWithLengthOpts(text, lens[i], &end, i & 1);
        printf("ParseWithLengthOpts[len=%zu,req=%d] root=%s end=%ld ", lens[i], i & 1, root ? "ok" : "NULL",
               end ? (long)(end - text) : -1L);
        put_err();
        if (root != NULL)
            show("  unformatted", root);
        cJSON_Delete(root);
    }
    root = cJSON_ParseWithLength(text, len);
    printf("ParseWithLength root=%s ", root ? "ok" : "NULL");
    put_err();
    cJSON_Delete(root);
    g_base = NULL;
    root = cJSON_Parse(NULL);
    printf("Parse(NULL) root=%s ", root ? "ok" : "NULL");
    put_err();
    cJSON_Delete(root);
}

/* ---- M3: lookups, mutations, minify -------------------------------------------------------------------- */

/* flip ASCII case and Latin-1 case (0xC0-0xDE <-> 0xE0-0xFE, not 0xD7/0xF7) */
static char *flip_case(const char *s)
{
    char *r = malloc(strlen(s) + 1);
    size_t i;
    for (i = 0; s[i]; i++)
    {
        unsigned char c = (unsigned char)s[i];
        if (c >= 'a' && c <= 'z')
            c = (unsigned char)(c - 32);
        else if (c >= 'A' && c <= 'Z')
            c = (unsigned char)(c + 32);
        else if (c >= 0xE0 && c <= 0xFE && c != 0xF7)
            c = (unsigned char)(c - 32);
        else if (c >= 0xC0 && c <= 0xDE && c != 0xD7)
            c = (unsigned char)(c + 32);
        r[i] = (char)c;
    }
    r[i] = 0;
    return r;
}

static void lookups(cJSON *root)
{
    int n = cJSON_GetArraySize(root);
    printf("-- lookups size=%d isinvalid=%d isfalse=%d istrue=%d isbool=%d isnull=%d isnumber=%d isstring=%d "
           "isarray=%d isobject=%d israw=%d\n",
           n, cJSON_IsInvalid(root), cJSON_IsFalse(root), cJSON_IsTrue(root), cJSON_IsBool(root),
           cJSON_IsNull(root), cJSON_IsNumber(root), cJSON_IsString(root), cJSON_IsArray(root),
           cJSON_IsObject(root), cJSON_IsRaw(root));
    put_line("stringvalue", cJSON_GetStringValue(root));
    printf("numbervalue=%.17g\n", cJSON_GetNumberValue(root));
    for (int i = -1; i <= n && i < 6; i++)
    {
        cJSON *it = cJSON_GetArrayItem(root, i);
        printf("item[%d]=%s", i, it ? "ok" : "NULL");
        if (it)
            show(" ", it);
        else
            putchar('\n');
    }
    for (cJSON *c = root->child; c != NULL && cJSON_IsObject(root); c = c->next)
    {
        char *flip;
        if (c->string == NULL)
            continue;
        flip = flip_case(c->string);
        printf("key ");
        put_str(c->string);
        printf(": ci=%s cs=%s has=%d | flip ", cJSON_GetObjectItem(root, c->string) == c ? "self" : "other",
               cJSON_GetObjectItemCaseSensitive(root, c->string) == c ? "self" : "other",
               (int)cJSON_HasObjectItem(root, c->string));
        put_str(flip);
        printf(": ci=%s cs=%s has=%d\n", cJSON_GetObjectItem(root, flip) ? "found" : "NULL",
               cJSON_GetObjectItemCaseSensitive(root, flip) ? "found" : "NULL", (int)cJSON_HasObjectItem(root, flip));
        free(flip);
    }
    printf("get missing=%s null_object=%s\n", cJSON_GetObjectItem(root, "no-such-key") ? "found" : "NULL",
           cJSON_GetObjectItem(NULL, "a") ? "found" : "NULL");
}

static void mutations(const cJSON *src)
{
    cJSON *d = cJSON_Duplicate(src, 1);
    cJSON *x;
    printf("-- mutations\n");
    if (d == NULL)
    {
        printf("duplicate NULL\n");
        return;
    }
    if (cJSON_IsArray(d))
    {
        x = cJSON_DetachItemFromArray(d, 0);
        printf("detach[0]=%s\n", x ? "ok" : "NULL");
        if (x)
            show("  detached", x);
        show("  after-detach", d);
        printf("insert[0]=%d\n", (int)cJSON_InsertItemInArray(d, 0, x ? x : cJSON_CreateNull()));
        printf("insert[99]=%d\n", (int)cJSON_InsertItemInArray(d, 99, cJSON_CreateTrue()));
        printf("replace[1]=%d\n", (int)cJSON_ReplaceItemInArray(d, 1, cJSON_CreateNumber(7)));
        printf("replace[-1]=%d\n", (int)cJSON_ReplaceItemInArray(d, -1, cJSON_CreateNull()));
        printf("addref=%d\n", (int)cJSON_AddItemReferenceToArray(d, cJSON_CreateStringReference("ref")));
        show("  after-edits", d);
        cJSON_DeleteItemFromArray(d, cJSON_GetArraySize(d) - 1);
        cJSON_DeleteItemFromArray(d, 1000);
        show("  after-delete", d);
        x = cJSON_DetachItemViaPointer(d, d->child);
        printf("detach-via-pointer=%s\n", x ? "ok" : "NULL");
        cJSON_Delete(x);
        show("  final", d);
    }
    else if (cJSON_IsObject(d))
    {
        const char *k = d->child ? d->child->string : NULL;
        char *kc = k ? strdup(k) : NULL;
        if (kc)
        {
            char *fl = flip_case(kc);
            x = cJSON_DetachItemFromObject(d, fl);
            printf("detach-ci[flip]=%s\n", x ? "ok" : "NULL");
            if (x)
            {
                printf("re-add=%d\n", (int)cJSON_AddItemToObject(d, kc, x));
            }
            printf("replace-ci=%d\n", (int)cJSON_ReplaceItemInObject(d, fl, cJSON_CreateString("rep")));
            printf("replace-cs=%d\n",
                   (int)cJSON_ReplaceItemInObjectCaseSensitive(d, "no-such-key", cJSON_CreateString("rep2")));
            show("  after-replace", d);
            printf("addCS=%d\n", (int)cJSON_AddItemToObjectCS(d, "const-key", cJSON_CreateNumber(1.25)));
            printf("addref=%d\n", (int)cJSON_AddItemReferenceToObject(d, "ref-key", d->child));
            cJSON_DeleteItemFromObjectCaseSensitive(d, "const-key");
            cJSON_DeleteItemFromObject(d, "ref-key"); /* the reference first: it aliases d->child's data */
            cJSON_DeleteItemFromObject(d, fl);
            show("  after-delete", d);
            free(fl);
            free(kc);
        }
        else
            printf("object without keys\n");
        printf("AddStringToObject=%s\n", cJSON_AddStringToObject(d, "added", "v") ? "ok" : "NULL");
        printf("AddNumberToObject=%s\n", cJSON_AddNumberToObject(d, "n", 2.5) ? "ok" : "NULL");
        printf("AddNullToObject=%s\n", cJSON_AddNullToObject(d, "z") ? "ok" : "NULL");
        printf("AddRawToObject=%s\n", cJSON_AddRawToObject(d, "raw", "[1,2]") ? "ok" : "NULL");
        printf("AddObjectToObject=%s\n", cJSON_AddObjectToObject(d, "o") ? "ok" : "NULL");
        printf("AddArrayToObject=%s\n", cJSON_AddArrayToObject(d, "a") ? "ok" : "NULL");
        show("  final", d);
    }
    else
    {
        printf("setters ");
        printf("SetNumberHelper=%.17g ", cJSON_SetNumberHelper(d, 3.75));
        put_line("SetValuestring", cJSON_SetValuestring(d, "replaced"));
        show("  final", d);
    }
    cJSON_Delete(d);
}

static void minify_pass(const char *text, size_t len)
{
    char *m = malloc(len + 1);
    cJSON *r;
    memcpy(m, text, len + 1);
    cJSON_Minify(m);
    put_line("minified", m);
    r = cJSON_Parse(m);
    printf("minified-parses=%s\n", r ? "ok" : "NULL");
    if (r)
        show("minified-reprint", r);
    cJSON_Delete(r);
    free(m);
}

/* ---- M4: hooks ------------------------------------------------------------------------------------------ */

static void hooks_pass(const char *text, size_t len)
{
    cJSON *root;
    long total, i, limit;
    char *s;
    printf("== hooks\n");
    hooks_on(0);
    root = cJSON_ParseWithLength(text, len);
    printf("count parse allocs=%ld frees=%ld bytes=%ld root=%s\n", n_alloc, n_free, n_bytes, root ? "ok" : "NULL");
    total = n_alloc;
    if (root != NULL)
    {
        long a0 = n_alloc, f0 = n_free, b0 = n_bytes;
        s = cJSON_Print(root);
        printf("count print allocs=%ld frees=%ld bytes=%ld\n", n_alloc - a0, n_free - f0, n_bytes - b0);
        cJSON_free(s);
        a0 = n_alloc;
        f0 = n_free;
        b0 = n_bytes;
        s = cJSON_PrintUnformatted(root);
        printf("count print-unformatted allocs=%ld frees=%ld bytes=%ld\n", n_alloc - a0, n_free - f0, n_bytes - b0);
        cJSON_free(s);
        a0 = n_alloc;
        f0 = n_free;
        b0 = n_bytes;
        {
            cJSON *d = cJSON_Duplicate(root, 1);
            printf("count duplicate allocs=%ld bytes=%ld\n", n_alloc - a0, n_bytes - b0);
            cJSON_Delete(d);
        }
    }
    cJSON_Delete(root);
    printf("count after delete: allocs=%ld frees=%ld\n", n_alloc, n_free);
    hooks_off();

    limit = total < MAXFAIL ? total : MAXFAIL;
    for (i = 1; i <= limit; i++)
    {
        hooks_on(i);
        root = cJSON_ParseWithLength(text, len);
        cJSON_Delete(root);
        printf("fail-parse[%ld] root=%s live=%ld\n", i, root ? "ok" : "NULL", (n_alloc - (fail_at <= n_alloc ? 1 : 0)) - n_free);
        hooks_off();
    }
    root = cJSON_ParseWithLength(text, len);
    if (root == NULL)
        return;
    hooks_on(0);
    s = cJSON_Print(root);
    total = n_alloc;
    cJSON_free(s);
    hooks_off();
    limit = total < MAXFAIL ? total : MAXFAIL;
    for (i = 1; i <= limit; i++)
    {
        int ok;
        hooks_on(i);
        s = cJSON_Print(root);
        ok = s != NULL;
        cJSON_free(s);
        printf("fail-print[%ld] out=%s live=%ld\n", i, ok ? "ok" : "NULL", live_now());
        hooks_off();
        hooks_on(i);
        s = cJSON_PrintBuffered(root, 8, 0);
        ok = s != NULL;
        cJSON_free(s);
        printf("fail-buffered[%ld] out=%s live=%ld\n", i, ok ? "ok" : "NULL", live_now());
        hooks_off();
    }
    hooks_on(0);
    {
        cJSON *d = cJSON_Duplicate(root, 1);
        total = n_alloc;
        cJSON_Delete(d);
    }
    hooks_off();
    limit = total < MAXFAIL ? total : MAXFAIL;
    for (i = 1; i <= limit; i++)
    {
        cJSON *d;
        int ok;
        hooks_on(i);
        d = cJSON_Duplicate(root, 1);
        ok = d != NULL;
        cJSON_Delete(d);
        printf("fail-duplicate[%ld] out=%s live=%ld\n", i, ok ? "ok" : "NULL", live_now());
        hooks_off();
    }
    cJSON_Delete(root);
}

/* ---- M5: cJSON_Utils ------------------------------------------------------------------------------------ */

static void pointers(cJSON *root)
{
    cJSON *c;
    int n = 0;
    printf("-- pointers\n");
    for (c = root->child; c != NULL && n < 12; c = c->next, n++)
    {
        char *p = cJSONUtils_FindPointerFromObjectTo(root, c);
        cJSON *back1 = p ? cJSONUtils_GetPointer(root, p) : NULL;
        cJSON *back2 = p ? cJSONUtils_GetPointerCaseSensitive(root, p) : NULL;
        put_line("  pointer-to-child", p);
        printf("  roundtrip ci=%s cs=%s\n", back1 == c ? "same" : (back1 ? "other" : "NULL"),
               back2 == c ? "same" : (back2 ? "other" : "NULL"));
        if (p)
            cJSON_free(p);
    }
    {
        static const char *fixed[] = {"",        "/",       "/0",    "/1",     "/-",      "/00",    "/a",   "/A",
                                      "/a/b",    "/a~1b",   "/a~0b", "/~0",    "/~1",     "/~01",   "/x~1", "/ab~1cd",
                                      "/é",      "/\xC9",   "/\xE9", "a",      "/key",    "/0/0",   "/1/a", "//",
                                      "/18446744073709551616", "/0/18446744073709551617"};
        for (size_t i = 0; i < sizeof fixed / sizeof fixed[0]; i++)
        {
            cJSON *g = cJSONUtils_GetPointer(root, fixed[i]);
            cJSON *gc = cJSONUtils_GetPointerCaseSensitive(root, fixed[i]);
            printf("  GetPointer ");
            put_str(fixed[i]);
            printf(" ci=%s cs=%s", g ? "ok" : "NULL", gc ? "ok" : "NULL");
            if (g)
                show(" ", g);
            else
                putchar('\n');
        }
    }
    /* object keys as encoded pointers */
    for (c = root->child; c != NULL && cJSON_IsObject(root) && c->string != NULL; c = c->next)
    {
        cJSON *tgt;
        char enc[512];
        size_t o = 0;
        enc[o++] = '/';
        for (const char *k = c->string; *k && o < sizeof enc - 3; k++)
        {
            if (*k == '~')
            {
                enc[o++] = '~';
                enc[o++] = '0';
            }
            else if (*k == '/')
            {
                enc[o++] = '~';
                enc[o++] = '1';
            }
            else
                enc[o++] = *k;
        }
        enc[o] = 0;
        tgt = cJSONUtils_GetPointerCaseSensitive(root, enc);
        printf("  key-pointer ");
        put_str(enc);
        printf(" -> %s\n", tgt == c ? "same" : (tgt ? "other" : "NULL"));
    }
}

static void pair_ops(const char *ta, const char *tb)
{
    cJSON *a = cJSON_Parse(ta);
    cJSON *b = cJSON_Parse(tb);
    printf("== pair a=%s b=%s\n", a ? "ok" : "NULL", b ? "ok" : "NULL");
    if (a != NULL && b != NULL)
    {
        if (tree_depth(a) <= COMPARE_MAX_DEPTH && tree_depth(b) <= COMPARE_MAX_DEPTH)
            printf("Compare ci=%d cs=%d self=%d\n", (int)cJSON_Compare(a, b, 0), (int)cJSON_Compare(a, b, 1),
                   (int)cJSON_Compare(a, a, 1));
        else
            printf("Compare skipped (depth > %d)\n", COMPARE_MAX_DEPTH);
        for (int cs = 0; cs <= 1; cs++)
        {
            cJSON *patches = cs ? cJSONUtils_GeneratePatchesCaseSensitive(a, b) : cJSONUtils_GeneratePatches(a, b);
            cJSON *target;
            int rc;
            printf("-- GeneratePatches[cs=%d]\n", cs);
            show("  a-after", a); /* Q18: sorting in place */
            show("  b-after", b);
            if (patches != NULL)
            {
                show("  patches", patches);
                target = cJSON_Duplicate(a, 1);
                rc = cs ? cJSONUtils_ApplyPatchesCaseSensitive(target, patches)
                        : cJSONUtils_ApplyPatches(target, patches);
                printf("  apply-own rc=%d\n", rc);
                show("  apply-own-result", target);
                cJSON_Delete(target);
            }
            else
                printf("  patches NULL\n");
            cJSON_Delete(patches);

            target = cJSON_Duplicate(a, 1);
            rc = cs ? cJSONUtils_ApplyPatchesCaseSensitive(target, b) : cJSONUtils_ApplyPatches(target, b);
            printf("-- ApplyPatches(a, b as patches)[cs=%d] rc=%d\n", cs, rc);
            show("  result", target);
            cJSON_Delete(target);

            target = cJSON_Duplicate(a, 1);
            target = cs ? cJSONUtils_MergePatchCaseSensitive(target, b) : cJSONUtils_MergePatch(target, b);
            printf("-- MergePatch(a, b)[cs=%d]\n", cs);
            show("  result", target);
            cJSON_Delete(target);

            {
                cJSON *mp = cs ? cJSONUtils_GenerateMergePatchCaseSensitive(a, b) : cJSONUtils_GenerateMergePatch(a, b);
                printf("-- GenerateMergePatch[cs=%d]\n", cs);
                if (mp != NULL)
                {
                    cJSON *t2 = cJSON_Duplicate(a, 1);
                    show("  patch", mp);
                    t2 = cs ? cJSONUtils_MergePatchCaseSensitive(t2, mp) : cJSONUtils_MergePatch(t2, mp);
                    show("  applied", t2);
                    cJSON_Delete(t2);
                }
                else
                    printf("  NULL\n");
                cJSON_Delete(mp);
            }
        }
        {
            cJSON *s = cJSON_Duplicate(a, 1);
            cJSONUtils_SortObject(s);
            show("sorted-ci", s);
            if (utils_subset)
                dump_tree(s, 0);
            cJSON_Delete(s);
            s = cJSON_Duplicate(a, 1);
            cJSONUtils_SortObjectCaseSensitive(s);
            show("sorted-cs", s);
            if (utils_subset)
                dump_tree(s, 0);
            cJSON_Delete(s);
        }
        {
            cJSON *p = cJSON_CreateArray();
            cJSONUtils_AddPatchToArray(p, "add", "/x", b);
            cJSONUtils_AddPatchToArray(p, "remove", "/y", NULL);
            show("AddPatchToArray", p);
            cJSON_Delete(p);
        }
        pointers(a);
    }
    cJSON_Delete(a);
    cJSON_Delete(b);
}

/* ---- M6: locales ---------------------------------------------------------------------------------------- */

static void locale_pass(const char *ta)
{
    static const char *comma[] = {"de_DE.UTF-8", "fr_FR.UTF-8", "ru_RU.UTF-8", "nl_NL.UTF-8", "de_DE.ISO8859-1",
                                  NULL};
    static const char *latin1[] = {"de_DE.ISO8859-1", "fr_FR.ISO8859-1", "en_US.ISO8859-15", NULL};
    const char *used = NULL;
    printf("== locale\n");
    for (int i = 0; comma[i] != NULL && used == NULL; i++)
    {
        if (setlocale(LC_NUMERIC, comma[i]) != NULL && localeconv()->decimal_point[0] == ',')
            used = comma[i];
    }
    if (used == NULL)
    {
        setlocale(LC_NUMERIC, "C");
        printf("numeric-locale none (no comma locale available)\n");
    }
    else
    {
        cJSON *r;
        cJSON *n;
        const char *texts[] = {"1.5", "[1.5,2.25,-0.5e2]", "{\"a\":1.5}", "1,5", ta};
        printf("numeric-locale %s decimal_point=%s\n", used, localeconv()->decimal_point);
        if (getenv("DUMP_PROBE_IN_C_LOCALE") != NULL) /* Stage 2 crate always uses the C locale (D3) */
            setlocale(LC_NUMERIC, "C");
        for (int i = 0; i < 5; i++)
        {
            g_base = texts[i];
            r = cJSON_Parse(texts[i]);
            printf("parse[%d] root=%s ", i, r ? "ok" : "NULL");
            put_err();
            if (r)
            {
                show("  print", r);
                dump_tree(r, 1);
            }
            cJSON_Delete(r);
        }
        g_base = NULL;
        n = cJSON_CreateNumber(1.5);
        show("create(1.5)", n);
        cJSON_Delete(n);
        n = cJSON_CreateNumber(0.1);
        show("create(0.1)", n);
        cJSON_Delete(n);
        n = cJSON_CreateNumber(123456.789012345678);
        show("create(123456.789012345678)", n);
        cJSON_Delete(n);
        setlocale(LC_NUMERIC, "C");
    }

    used = NULL;
    for (int i = 0; latin1[i] != NULL && used == NULL; i++)
    {
        if (setlocale(LC_CTYPE, latin1[i]) != NULL)
            used = latin1[i];
    }
    if (used == NULL)
    {
        setlocale(LC_CTYPE, "C");
        printf("ctype-locale none (no Latin-1 locale available)\n");
    }
    else
    {
        cJSON *o, *a, *b;
        const char *objs[] = {"{\"\xC9t\xE9\":1,\"b\":2,\"\xE9t\xC9\":3,\"B\":4}", "{\"\xC9\":1}", "{\"\xE9\":1}"};
        printf("ctype-locale %s tolower(0xC9)=%d tolower(0xDF)=%d tolower(0xE9)=%d\n", used, tolower(0xC9),
               tolower(0xDF), tolower(0xE9));
        if (getenv("DUMP_PROBE_IN_C_LOCALE") != NULL) /* Stage 2 crate always uses the C locale (D19) */
            setlocale(LC_CTYPE, "C");
        o = cJSON_Parse(objs[0]);
        {
            const char *keys[] = {"\xC9t\xE9", "\xE9t\xE9", "\xE9t\xC9", "\xC9T\xC9", "b", "B", "\xE9", NULL};
            for (int i = 0; keys[i] != NULL; i++)
            {
                cJSON *f = cJSON_GetObjectItem(o, keys[i]);
                cJSON *g = cJSON_GetObjectItemCaseSensitive(o, keys[i]);
                printf("lookup ");
                put_str(keys[i]);
                printf(" ci=%s cs=%s\n", f ? (f->valueint == 0 ? "found0" : "found") : "NULL", g ? "found" : "NULL");
                if (f)
                    printf("  ci-value=%d\n", f->valueint);
            }
            for (int i = 0; keys[i] != NULL; i++)
            {
                char ptr[32];
                cJSON *p;
                snprintf(ptr, sizeof ptr, "/%s", keys[i]);
                p = cJSONUtils_GetPointer(o, ptr);
                printf("pointer ");
                put_str(ptr);
                printf(" -> %s\n", p ? "found" : "NULL");
            }
        }
        a = cJSON_Parse(objs[1]);
        b = cJSON_Parse(objs[2]);
        printf("compare ci=%d cs=%d\n", (int)cJSON_Compare(a, b, 0), (int)cJSON_Compare(a, b, 1));
        {
            cJSON *s = cJSON_Parse("{\"\xE9\":1,\"\xC9\":2,\"z\":3,\"A\":4,\"\xC0\":5,\"\xE0\":6}");
            cJSONUtils_SortObject(s);
            show("sorted-ci", s);
            cJSON_Delete(s);
        }
        {
            cJSON *d = cJSON_Parse(ta);
            if (d != NULL && cJSON_IsObject(d) && d->child != NULL && d->child->string != NULL)
            {
                char *fl = flip_case(d->child->string);
                printf("input-key lookup ");
                put_str(fl);
                printf(" ci=%s\n", cJSON_GetObjectItem(d, fl) ? "found" : "NULL");
                free(fl);
            }
            cJSON_Delete(d);
        }
        cJSON_Delete(o);
        cJSON_Delete(a);
        cJSON_Delete(b);
        setlocale(LC_CTYPE, "C");
    }
}

/* ---- M7: fixed constructors ----------------------------------------------------------------------------- */

static void constructors(void)
{
    static const double nums[] = {0.0,  -0.0, 1.0,   -1.0,     0.5,       1.5,       0.1,       1e10,
                                  1e15, 1e16, 1e300, -1e300,   2147483647.0, 2147483648.0, -2147483648.0,
                                  -2147483649.0, 4294967296.0, 3.141592653589793, 1.0 / 3.0, 1e-5, 1e-7,
                                  123456789012345678.0, 9007199254740993.0, 5e-324, 1.7976931348623157e308};
    cJSON *n, *arr;
    double nan_v = NAN, inf_v = INFINITY;
    printf("== constructors\n");
    for (size_t i = 0; i < sizeof nums / sizeof nums[0]; i++)
    {
        n = cJSON_CreateNumber(nums[i]);
        printf("number[%zu] vi=%d vd=%.17g ", i, n->valueint, n->valuedouble);
        show("print", n);
        cJSON_Delete(n);
    }
    /* D6: (int)NaN is UB in C; both sides are expected to print 0 on this platform (measured, see RUN-NOTES) */
    n = cJSON_CreateNumber(nan_v);
    printf("NaN vi=%d ", n->valueint);
    show("print", n);
    printf("NaN getnumber isnan=%d\n", isnan(cJSON_GetNumberValue(n)));
    printf("SetNumberHelper(NaN)=%.17g vi=%d\n", cJSON_SetNumberHelper(n, nan_v), n->valueint);
    cJSON_Delete(n);
    n = cJSON_CreateNumber(inf_v);
    printf("Inf vi=%d ", n->valueint);
    show("print", n);
    cJSON_Delete(n);
    n = cJSON_CreateNumber(-inf_v);
    printf("-Inf vi=%d ", n->valueint);
    show("print", n);
    cJSON_Delete(n);
    printf("GetNumberValue(NULL) isnan=%d\n", isnan(cJSON_GetNumberValue(NULL)));

    arr = cJSON_CreateArray();
    cJSON_AddItemToArray(arr, cJSON_CreateString("a\"b\\c\n\t\x01\x1f/"));
    cJSON_AddItemToArray(arr, cJSON_CreateString("\xc3\xa9\xff"));
    cJSON_AddItemToArray(arr, cJSON_CreateString(""));
    cJSON_AddItemToArray(arr, cJSON_CreateTrue());
    cJSON_AddItemToArray(arr, cJSON_CreateFalse());
    cJSON_AddItemToArray(arr, cJSON_CreateBool(7));
    cJSON_AddItemToArray(arr, cJSON_CreateNull());
    cJSON_AddItemToArray(arr, cJSON_CreateRaw("{raw:1}"));
    cJSON_AddItemToArray(arr, cJSON_CreateStringReference("ref"));
    {
        const int ints[] = {1, -2, 2147483647};
        const float floats[] = {0.5f, 1.25f};
        const double doubles[] = {0.1, 1e100};
        const char *strs[] = {"x", "y\"z"};
        cJSON_AddItemToArray(arr, cJSON_CreateIntArray(ints, 3));
        cJSON_AddItemToArray(arr, cJSON_CreateFloatArray(floats, 2));
        cJSON_AddItemToArray(arr, cJSON_CreateDoubleArray(doubles, 2));
        cJSON_AddItemToArray(arr, cJSON_CreateStringArray(strs, 2));
    }
    show("built", arr);
    show_fmt("built-fmt", arr);
    printf("version=");
    put_str(cJSON_Version());
    putchar('\n');
    cJSON_Delete(arr);
    n = cJSON_CreateNull();
    printf("PrintBuffered(NULL item)=%s\n", cJSON_PrintBuffered(NULL, 0, 0) ? "ok" : "NULL");
    printf("Print(NULL)=%s\n", cJSON_Print(NULL) ? "ok" : "NULL");
    printf("AddItemToArray(NULL,NULL)=%d\n", (int)cJSON_AddItemToArray(NULL, NULL));
    printf("InsertItemInArray(self)=%d\n", (int)cJSON_InsertItemInArray(n, 0, n));
    cJSON_Delete(n);

    /* D21: writes through the char * that GetStringValue, SetValuestring and the print functions return */
    n = cJSON_CreateString("abcdef");
    {
        char *v = cJSON_GetStringValue(n);
        const unsigned long first = (unsigned long)v; /* an address value, compared only */
        char *out;
        v[0] = 'X';
        v[3] = '\0';
        put_line("view GetStringValue", cJSON_GetStringValue(n));
        show("view print", n);
        v = cJSON_SetValuestring(n, "xyz"); /* 3 <= strlen 3: copied in place */
        printf("view SetValuestring(xyz) same-buffer=%d\n", (unsigned long)v == first);
        v[1] = 'Y';
        put_line("view after write", cJSON_GetStringValue(n));
        v = cJSON_SetValuestring(n, "long1"); /* longer: new buffer, allocated before the old one is freed */
        printf("view SetValuestring(long1) same-buffer=%d\n", (unsigned long)v == first);
        out = cJSON_PrintUnformatted(n);
        out[1] = 'Q';
        out[4] = '\0';
        put_line("view printed", out);
        cJSON_free(out);
    }
    cJSON_Delete(n);
    /* Q3: valueint and valuedouble are separate fields; print uses %d only when they agree */
    n = cJSON_CreateNumber(7);
    n->valuedouble = 7.5;
    show("field vd=7.5", n);
    n->valueint = 3;
    show("field vi=3", n);
    n->valuedouble = 3.0;
    show("field vd=3", n);
    n->valueint = 100;
    show("field vi=100", n);
    printf("field getnumber=%.17g vi=%d\n", cJSON_GetNumberValue(n), n->valueint);
    cJSON_Delete(n);
}

/* ---- main ----------------------------------------------------------------------------------------------- */

static void run_document(const char *label, const char *text, size_t len)
{
    cJSON *root;
    parse_modes(label, text, len);
    root = cJSON_ParseWithLength(text, len);
    if (root != NULL)
    {
        printf("== document %s\n", label);
        print_modes(label, root);
        lookups(root);
        mutations(root);
        {
            cJSON *d1 = cJSON_Duplicate(root, 1);
            cJSON *d0 = cJSON_Duplicate(root, 0);
            printf("-- duplicate\n");
            show("  deep", d1);
            show("  shallow", d0);
            if (tree_depth(root) <= COMPARE_MAX_DEPTH)
                printf("  compare deep ci=%d cs=%d shallow ci=%d\n", (int)cJSON_Compare(root, d1, 0),
                       (int)cJSON_Compare(root, d1, 1), (int)cJSON_Compare(root, d0, 0));
            else
                printf("  compare skipped (depth > %d)\n", COMPARE_MAX_DEPTH);
            cJSON_Delete(d1);
            cJSON_Delete(d0);
        }
        cJSON_Delete(root);
    }
    minify_pass(text, len);
}

/* ---- Utils allocator section (patch-* and judge-* inputs): trace and fail-the-Nth for the Utils calls ------- */

static void utils_once(int op, const cJSON *A, const cJSON *B, long fail, const char *label, long *total)
{
    cJSON *x = cJSON_Duplicate(A, 1);
    cJSON *y = cJSON_Duplicate(B, 1);
    cJSON *r = NULL;
    char *ptr = NULL;
    char ptr_copy[256];
    int ptr_ok = 0;
    int st = 0;
    long live;
    trace_len = 0;
    trace_buf[0] = 0;
    hooks_on(fail);
    trace_on = 1;
    switch (op)
    {
    case 0:
        st = cJSONUtils_ApplyPatches(x, y);
        break;
    case 1:
        st = cJSONUtils_ApplyPatchesCaseSensitive(x, y);
        break;
    case 2:
        x = cJSONUtils_MergePatch(x, y);
        break;
    case 3:
        r = cJSONUtils_GenerateMergePatch(x, y);
        break;
    case 4:
        r = cJSONUtils_GeneratePatches(x, y);
        break;
    default:
        ptr = cJSONUtils_FindPointerFromObjectTo(x, x->child != NULL ? (x->child->child != NULL ? x->child->child : x->child) : x);
        break;
    }
    if (op == 5 && ptr != NULL)
    {
        /* the Rust adapter returns an owned Vec (the string is freed inside the call): free the C string here,
         * inside the counted window, so both drivers record the same trace */
        strncpy(ptr_copy, ptr, sizeof ptr_copy - 1);
        ptr_copy[sizeof ptr_copy - 1] = 0;
        cJSON_free(ptr);
        ptr_ok = 1;
    }
    trace_on = 0;
    live = live_now();
    if (total != NULL)
        *total = n_alloc;
    printf("utils-%s[%ld] st=%d live=%ld allocs=%ld trace=%s\n", label, fail, st, live, n_alloc, trace_buf);
    hooks_off();
    if (op == 5)
        put_line("  pointer", ptr_ok ? ptr_copy : NULL);
    else
    {
        show("  target", x);
        if (op >= 3)
        {
            if (r != NULL)
                show("  result", r);
            else
                printf("  result=NULL\n");
        }
    }
    cJSON_Delete(x);
    cJSON_Delete(y);
    cJSON_Delete(r);
}

static void utils_hooks_pass(const char *ta, size_t la, const char *tb, size_t lb)
{
    static const char *const names[] = {"apply-ci", "apply-cs", "merge", "genmerge", "genpatches", "findptr"};
    cJSON *A = cJSON_ParseWithLength(ta, la);
    cJSON *B = cJSON_ParseWithLength(tb, lb);
    int op;
    printf("== utils-hooks\n");
    if (A != NULL && B != NULL)
    {
        for (op = 0; op < 6; op++)
        {
            long total = 0, i, limit;
            utils_once(op, A, B, 0, names[op], &total);
            if (op >= 4)
                continue;
            limit = total < MAXFAIL ? total : MAXFAIL;
            for (i = 1; i <= limit; i++)
                utils_once(op, A, B, i, names[op], NULL);
        }
        if (utils_probe)
        {
            /* BLOCK 1: the size-1 strdup at cJSON_Utils.c:211; BLOCK 2: the path buffer of GeneratePatches([], [1]) */
            cJSON *e = cJSON_Parse("[]");
            cJSON *one = cJSON_Parse("[1]");
            cJSON *g;
            char *pp;
            cJSON *tgt = A->child->child;
            hooks_on(1);
            pp = cJSONUtils_FindPointerFromObjectTo(A, tgt);
            printf("probe findptr[fail 1] %s live=%ld\n", pp ? "ok" : "NULL", live_now());
            cJSON_free(pp);
            hooks_off();
            trace_len = 0;
            trace_buf[0] = 0;
            hooks_on(0);
            trace_on = 1;
            g = cJSONUtils_GeneratePatches(e, one);
            trace_on = 0;
            printf("probe genpatches trace=%s\n", trace_buf);
            hooks_off();
            cJSON_Delete(g);
            hooks_on(2);
            g = cJSONUtils_GeneratePatches(e, one);
            printf("probe genpatches[fail 2] %s live=%ld\n", g ? "ok" : "NULL", live_now());
            hooks_off();
            if (g != NULL)
                show("  result", g);
            else
                printf("  result=NULL\n");
            cJSON_Delete(g);
            cJSON_Delete(e);
            cJSON_Delete(one);
        }
    }
    cJSON_Delete(A);
    cJSON_Delete(B);
}

int main(int argc, char **argv)
{
    FILE *f;
    char *buf;
    size_t len, la, lb;
    char *a, *b;
    const char *sep;
    if (argc < 2)
        return 2;
    f = fopen(argv[argc - 1], "rb");
    if (!f)
        return 2;
    buf = malloc((1u << 20) + 1);
    len = fread(buf, 1, 1u << 20, f);
    fclose(f);
    buf[len] = 0;

    sep = len >= 5 ? memmem(buf, len, "\n===\n", 5) : NULL;
    if (sep != NULL)
    {
        la = (size_t)(sep - buf);
        a = malloc(la + 1);
        memcpy(a, buf, la);
        a[la] = 0;
        lb = len - la - 5;
        b = malloc(lb + 1);
        memcpy(b, sep + 5, lb);
        b[lb] = 0;
    }
    else
    {
        la = lb = len;
        a = malloc(la + 1);
        memcpy(a, buf, la + 1);
        b = malloc(lb + 1);
        memcpy(b, buf, lb + 1);
    }

    printf("input len=%zu a=%zu b=%zu separator=%d\n", len, la, lb, sep != NULL);
    run_document("A", a, la);
    if (sep != NULL)
        run_document("B", b, lb);
    hooks_pass(a, la);
    {
        const char *base = strrchr(argv[argc - 1], '/');
        base = base != NULL ? base + 1 : argv[argc - 1];
        utils_subset = strncmp(base, "patch-", 6) == 0 || strncmp(base, "judge-", 6) == 0;
        utils_probe = strncmp(base, "judge-utils-oom", 15) == 0;
    }
    if (utils_subset)
        utils_hooks_pass(a, la, b, lb);
    pair_ops(a, b);
    locale_pass(a);
    constructors();
    free(a);
    free(b);
    free(buf);
    return 0;
}
