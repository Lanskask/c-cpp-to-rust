"""Question: are RULEBOOK quirks Q15 (cJSON_Minify mishandles a backslash-backslash string) and Q20
(decode_pointer_inplace mis-decodes ~1 unless it is last) real behaviour of the pinned cJSON when the C library is
compiled and run?

Answer (run 2026-10-10 on cJSON @ 6d9f244): both are real. Q15: '["\\", "x y"]' minifies to '["\\", "xy"]'.
Q20: decode_pointer_inplace turns "a~1" into "a~" and "a~1b" into "a~/"; patches with ~1 fail with status 13.
q20_direct.c (same directory) calls the static decoder directly and shows each decoded string.

Usage: python3 -I cjson_q15_q20.py <repo-root> <build-dir>
"""
import subprocess
import sys
from pathlib import Path

DRIVER = r"""
#include <stdio.h>
#include <string.h>
#include <stdlib.h>
#include "cJSON.h"
#include "cJSON_Utils.h"

int main(void) {
    char a[] = "[\"\\\\\", \"x y\"]";
    printf("Q15 in      : %s\n", a);
    cJSON_Minify(a);
    printf("Q15 out     : %s\n", a);
    printf("Q15 expected: [\"\\\\\",\"x y\"]\n");

    cJSON *o = cJSON_Parse("{\"a/b\":1,\"a~b\":2,\"x/\":3,\"ab/cd\":4}");
    const char *ptrs[] = {"/a~1b", "/a~0b", "/x~1", "/ab~1cd"};
    for (int i = 0; i < 4; i++) {
        cJSON *r = cJSONUtils_GetPointer(o, ptrs[i]);
        printf("Q20 pointer %-8s -> %s\n", ptrs[i], r ? cJSON_PrintUnformatted(r) : "NULL (not found)");
    }
    const char *docs[] = {"{\"a/b\":1}", "{\"ab/cd\":1}", "{\"x/\":1}", "{\"a~b\":1}"};
    const char *pats[] = {"[{\"op\":\"replace\",\"path\":\"/a~1b\",\"value\":9}]",
                          "[{\"op\":\"replace\",\"path\":\"/ab~1cd\",\"value\":9}]",
                          "[{\"op\":\"replace\",\"path\":\"/x~1\",\"value\":9}]",
                          "[{\"op\":\"replace\",\"path\":\"/a~0b\",\"value\":9}]"};
    for (int i = 0; i < 4; i++) {
        cJSON *d = cJSON_Parse(docs[i]);
        cJSON *pa = cJSON_Parse(pats[i]);
        int rc = cJSONUtils_ApplyPatches(d, pa);
        printf("Q20 doc %-14s patch path %-9s rc=%d -> %s\n", docs[i], cJSON_GetObjectItem(pa->child, "path")->valuestring, rc, cJSON_PrintUnformatted(d));
        cJSON_Delete(d); cJSON_Delete(pa);
    }
    cJSON_Delete(o);
    return 0;
}
"""


def main() -> int:
    root, build = Path(sys.argv[1]), Path(sys.argv[2])
    src = root / "pilots" / "cjson" / "src"
    build.mkdir(parents=True, exist_ok=True)
    (build / "driver.c").write_text(DRIVER)
    exe = build / "driver"
    subprocess.run(["cc", "-O0", "-w", f"-I{src}", str(build / "driver.c"), str(src / "cJSON.c"),
                    str(src / "cJSON_Utils.c"), "-lm", "-o", str(exe)], check=True, cwd=build)
    print(subprocess.run([str(exe)], check=True, capture_output=True, text=True, cwd=build).stdout, end="")
    return 0


sys.exit(main())
