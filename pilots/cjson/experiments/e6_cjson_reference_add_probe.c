/* E6: is the `child->prev == NULL` branch of add_item_to_array (cJSON.c:2050-2055) reachable through the public
 * API, or only through a hand-corrupted list (RULEBOOK D14)?
 *
 * Question: the Stage 2 proposals review (experiments/stage2_proposals_review.md, D14) says the branch is
 * unreachable without corruption and may be removed in Stage 2. A reference made by cJSON_CreateArrayReference(x)
 * has x as its first child; if x is in no list, x->prev is NULL, which is exactly that branch.
 *
 * Build and run against the pinned original (never from inside the pilot directory):
 *   cc -O0 -w -Ipilots/cjson/src pilots/cjson/experiments/e6_cjson_reference_add_probe.c pilots/cjson/src/cJSON.c \
 *      -lm -o <scratch>/e6 && <scratch>/e6
 *
 * Answer (run 2026-10-10, macOS aarch64, clang -O0):
 *   add=1 insert=1 size=1 print=[null] y.prev_null=1 z.prev_null=1
 * cJSON_AddItemToArray and cJSON_InsertItemInArray on such a reference return true without appending: the branch
 * is reachable publicly, so Stage 2 keeps it (RULEBOOK D14, narrowed). The upstream.rs test
 * extra_add_to_a_reference_of_a_node_in_no_list_succeeds_without_appending pins this in both Stages.
 */
#include <stdio.h>
#include "cJSON.h"

int main(void)
{
    cJSON *x = cJSON_CreateNull(), *y = cJSON_CreateTrue(), *z = cJSON_CreateFalse();
    cJSON *ref = cJSON_CreateArrayReference(x);
    int r = cJSON_AddItemToArray(ref, y);
    int r2 = cJSON_InsertItemInArray(ref, 5, z);
    char *s = cJSON_PrintUnformatted(ref);
    printf("add=%d insert=%d size=%d print=%s y.prev_null=%d z.prev_null=%d\n", r, r2, cJSON_GetArraySize(ref), s,
           y->prev == NULL, z->prev == NULL);
    cJSON_free(s);
    cJSON_Delete(ref);
    cJSON_Delete(x);
    cJSON_Delete(y);
    cJSON_Delete(z);
    return 0;
}
