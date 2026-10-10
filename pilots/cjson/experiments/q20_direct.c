
#include <stdio.h>
#include <string.h>
#include "cJSON_Utils.c"
static void show(const char *in) {
    unsigned char buf[64];
    strcpy((char *)buf, in);
    decode_pointer_inplace(buf);
    printf("decode(\"%s\") -> \"%s\"\n", in, (char *)buf);
}
static void patch(const char *doc, const char *op_path) {
    char p[160];
    snprintf(p, sizeof p, "[{\"op\":\"replace\",\"path\":\"%s\",\"value\":9}]", op_path);
    cJSON *d = cJSON_Parse(doc), *pa = cJSON_Parse(p);
    int rc = cJSONUtils_ApplyPatches(d, pa);
    char *out = cJSON_PrintUnformatted(d);
    printf("apply replace %-8s on %-12s rc=%d -> %s\n", op_path, doc, rc, out);
    cJSON_free(out); cJSON_Delete(d); cJSON_Delete(pa);
}
int main(void) {
    show("a"); show("~0"); show("a~0"); show("a~1"); show("a~0b"); show("a~1b"); show("~0~0");
    patch("{\"a\":1}", "/a");
    patch("{\"a/b\":1}", "/a~1b");
    patch("{\"a~b\":1}", "/a~0b");
    patch("{\"x/\":1}", "/x~1");
    patch("{\"x~\":1}", "/x~0");
    return 0;
}
