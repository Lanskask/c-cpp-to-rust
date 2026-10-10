/* Differential driver for jsmn: reads a file, prints a canonical dump of three parses.
 * Built twice from this one source: against the original jsmn.h (implementation included)
 * and against the Rust staticlib (-DJSMN_HEADER). Build flags select JSMN_STRICT / JSMN_PARENT_LINKS. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "jsmn.h"

static void dump(const char *label, int r, const jsmntok_t *t, int n) {
  printf("%s r=%d\n", label, r);
  for (int i = 0; i < n; i++) {
#ifdef JSMN_PARENT_LINKS
    printf("  %d type=%d start=%d end=%d size=%d parent=%d\n", i, (int)t[i].type, t[i].start, t[i].end, t[i].size,
           t[i].parent);
#else
    printf("  %d type=%d start=%d end=%d size=%d\n", i, (int)t[i].type, t[i].start, t[i].end, t[i].size);
#endif
  }
}

int main(int argc, char **argv) {
  if (argc < 2) return 2;
  FILE *f = fopen(argv[argc - 1], "rb");
  if (!f) return 2;
  char *buf = malloc(1 << 20);
  size_t len = fread(buf, 1, (1 << 20) - 1, f);
  fclose(f);
  buf[len] = 0;

  jsmn_parser p;
  /* 1. count only (tokens == NULL) */
  jsmn_init(&p);
  int r = jsmn_parse(&p, buf, len, NULL, 0);
  printf("count r=%d pos=%u toknext=%u toksuper=%d\n", r, p.pos, p.toknext, p.toksuper);

  /* 2. full parse into a roomy buffer */
  enum { N = 512 };
  jsmntok_t *tok = calloc(N, sizeof *tok);
  jsmn_init(&p);
  r = jsmn_parse(&p, buf, len, tok, N);
  printf("full pos=%u toknext=%u toksuper=%d\n", p.pos, p.toknext, p.toksuper);
  dump("full", r, tok, r > 0 ? r : (int)p.toknext);

  /* 3. deliberately small buffer (NOMEM paths) */
  memset(tok, 0, N * sizeof *tok);
  jsmn_init(&p);
  r = jsmn_parse(&p, buf, len, tok, 3);
  printf("small pos=%u toknext=%u toksuper=%d\n", p.pos, p.toknext, p.toksuper);
  dump("small", r, tok, (int)p.toknext);

  /* 4. byte-at-a-time feeding (resumable parser; PART path) */
  memset(tok, 0, N * sizeof *tok);
  jsmn_init(&p);
  for (size_t cut = 1; cut <= len; cut++) {
    r = jsmn_parse(&p, buf, cut, tok, N);
    if (r != JSMN_ERROR_PART) break;
  }
  printf("chunked pos=%u toknext=%u toksuper=%d\n", p.pos, p.toknext, p.toksuper);
  dump("chunked", r, tok, r > 0 ? r : (int)p.toknext);

  /* 5. dirty buffer: allocation does not reset the type (Q2) */
  for (int i = 0; i < N; i++) tok[i].type = JSMN_PRIMITIVE; /* other fields as phase 4 left them */
  jsmn_init(&p);
  r = jsmn_parse(&p, buf, len, tok, N);
  printf("dirty pos=%u toknext=%u toksuper=%d\n", p.pos, p.toknext, p.toksuper);
  dump("dirty", r, tok, r > 0 ? r : (int)p.toknext);

  /* 6. reused parser, num_tokens below toknext (Q9, RP14) */
  memset(tok, 0, N * sizeof *tok);
  jsmn_init(&p);
  size_t half = len / 2;
  r = jsmn_parse(&p, buf, half, tok, N);
  unsigned k = p.toknext / 2;
  printf("shrink1 r=%d pos=%u toknext=%u toksuper=%d k=%u\n", r, p.pos, p.toknext, p.toksuper, k);
  r = jsmn_parse(&p, buf, len, tok, k);
  printf("shrink pos=%u toknext=%u toksuper=%d\n", p.pos, p.toknext, p.toksuper);
  dump("shrink", r, tok, r > 0 ? r : (int)p.toknext);
  free(tok);
  free(buf);
  return 0;
}
