/* Judge shim for the upstream cJSON tests (RULEBOOK §10, ticket 02).
 *
 * The upstream tests/common.h does `#include "../cJSON.c"`, so every test compiles the C library into
 * itself and could never exercise the Rust crate. This file replaces it: it includes only the public
 * headers, and re-declares the C library's *private* declarations that the 12 internals-using tests
 * call, as ordinary extern declarations. Those names are exported by the Rust crate under the cargo
 * feature `test-internals` (Stage 1 only). Everything below is copied from cJSON.c @ 6d9f244 except the
 * `extern` keywords; nothing here is invented behaviour.
 */
#ifndef CJSON_TESTS_COMMON_H
#define CJSON_TESTS_COMMON_H

/* what cJSON.c includes (tests rely on these transitively) */
#include <string.h>
#include <stdio.h>
#include <math.h>
#include <stdlib.h>
#include <limits.h>
#include <ctype.h>
#include <float.h>
#ifdef ENABLE_LOCALES
#include <locale.h>
#endif

#include "../cJSON.h"
#include "../cJSON_Utils.h"

#ifndef true
#define true ((cJSON_bool)1)
#endif
#ifndef false
#define false ((cJSON_bool)0)
#endif

/* cJSON.c:156-161, 289-296, 478-487 */
typedef struct internal_hooks
{
    void *(CJSON_CDECL *allocate)(size_t size);
    void (CJSON_CDECL *deallocate)(void *pointer);
    void *(CJSON_CDECL *reallocate)(void *pointer, size_t size);
} internal_hooks;

typedef struct
{
    const unsigned char *content;
    size_t length;
    size_t offset;
    size_t depth;
    internal_hooks hooks;
} parse_buffer;

typedef struct
{
    unsigned char *buffer;
    size_t length;
    size_t offset;
    size_t depth;
    cJSON_bool noalloc;
    cJSON_bool format;
    internal_hooks hooks;
} printbuffer;

#define can_read(buffer, size) ((buffer != NULL) && (((buffer)->offset + size) <= (buffer)->length))
#define can_access_at_index(buffer, index) ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
#define cannot_access_at_index(buffer, index) (!can_access_at_index(buffer, index))
#define buffer_at_offset(buffer) ((buffer)->content + (buffer)->offset)
#define static_strlen(string_literal) (sizeof(string_literal) - sizeof(""))

/* the C library's private state and functions, exported by the Rust crate with `test-internals` */
extern internal_hooks global_hooks;
extern cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
extern unsigned parse_hex4(const unsigned char * const input);
extern cJSON_bool parse_string(cJSON * const item, parse_buffer * const input_buffer);
extern cJSON_bool parse_value(cJSON * const item, parse_buffer * const input_buffer);
extern cJSON_bool parse_array(cJSON * const item, parse_buffer * const input_buffer);
extern cJSON_bool parse_object(cJSON * const item, parse_buffer * const input_buffer);
extern cJSON_bool print_number(const cJSON * const item, printbuffer * const output_buffer);
extern cJSON_bool print_string_ptr(const unsigned char * const input, printbuffer * const output_buffer);
extern cJSON_bool print_value(const cJSON * const item, printbuffer * const output_buffer);
extern cJSON_bool print_array(const cJSON * const item, printbuffer * const output_buffer);
extern cJSON_bool print_object(const cJSON * const item, printbuffer * const output_buffer);
extern unsigned char* ensure(printbuffer * const p, size_t needed);
extern parse_buffer *skip_utf8_bom(parse_buffer * const buffer);
extern parse_buffer *buffer_skip_whitespace(parse_buffer * const buffer);
extern cJSON_bool add_item_to_array(cJSON *array, cJSON *item);
extern unsigned char* cJSON_strdup(const unsigned char* string, const internal_hooks * const hooks);
extern cJSON_bool compare_double(double a, double b);
/* `print` is used by the print_* tests as a name only through cJSON_Print*; declared for completeness */
extern unsigned char *print(const cJSON * const item, cJSON_bool format, const internal_hooks * const hooks);

/* copied from upstream tests/common.h (everything below the include) */
void reset(cJSON *item);
void reset(cJSON *item) {
    if ((item != NULL) && (item->child != NULL))
    {
        cJSON_Delete(item->child);
    }
    if ((item->valuestring != NULL) && !(item->type & cJSON_IsReference))
    {
        global_hooks.deallocate(item->valuestring);
    }
    if ((item->string != NULL) && !(item->type & cJSON_StringIsConst))
    {
        global_hooks.deallocate(item->string);
    }

    memset(item, 0, sizeof(cJSON));
}

char* read_file(const char *filename);
char* read_file(const char *filename) {
    FILE *file = NULL;
    long length = 0;
    char *content = NULL;
    size_t read_chars = 0;

    file = fopen(filename, "rb");
    if (file == NULL)
    {
        goto cleanup;
    }
    if (fseek(file, 0, SEEK_END) != 0)
    {
        goto cleanup;
    }
    length = ftell(file);
    if (length < 0)
    {
        goto cleanup;
    }
    if (fseek(file, 0, SEEK_SET) != 0)
    {
        goto cleanup;
    }
    content = (char*)malloc((size_t)length + sizeof(""));
    if (content == NULL)
    {
        goto cleanup;
    }
    read_chars = fread(content, sizeof(char), (size_t)length, file);
    if ((long)read_chars != length)
    {
        free(content);
        content = NULL;
        goto cleanup;
    }
    content[read_chars] = '\0';

cleanup:
    if (file != NULL)
    {
        fclose(file);
    }

    return content;
}

#define assert_has_type(item, item_type) TEST_ASSERT_BITS_MESSAGE(0xFF, item_type, item->type, "Item doesn't have expected type.")
#define assert_has_no_reference(item) TEST_ASSERT_BITS_MESSAGE(cJSON_IsReference, 0, item->type, "Item should not have a string as reference.")
#define assert_has_no_const_string(item) TEST_ASSERT_BITS_MESSAGE(cJSON_StringIsConst, 0, item->type, "Item should not have a const string.")
#define assert_has_valuestring(item) TEST_ASSERT_NOT_NULL_MESSAGE(item->valuestring, "Valuestring is NULL.")
#define assert_has_no_valuestring(item) TEST_ASSERT_NULL_MESSAGE(item->valuestring, "Valuestring is not NULL.")
#define assert_has_string(item) TEST_ASSERT_NOT_NULL_MESSAGE(item->string, "String is NULL")
#define assert_has_no_string(item) TEST_ASSERT_NULL_MESSAGE(item->string, "String is not NULL.")
#define assert_not_in_list(item) \
	TEST_ASSERT_NULL_MESSAGE(item->next, "Linked list next pointer is not NULL.");\
	TEST_ASSERT_NULL_MESSAGE(item->prev, "Linked list previous pointer is not NULL.")
#define assert_has_child(item) TEST_ASSERT_NOT_NULL_MESSAGE(item->child, "Item doesn't have a child.")
#define assert_has_no_child(item) TEST_ASSERT_NULL_MESSAGE(item->child, "Item has a child.")
#define assert_is_invalid(item) \
	assert_has_type(item, cJSON_Invalid);\
	assert_not_in_list(item);\
	assert_has_no_child(item);\
	assert_has_no_string(item);\
	assert_has_no_valuestring(item)

#endif
