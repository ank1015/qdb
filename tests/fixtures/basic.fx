# The main fixture: a small source tree with the awkward cases in it
# (hidden files, an empty folder, an empty file, a file with no final
# newline, names with spaces, non-ASCII text).
F README.md
|# Sample project
|
|A small tree used by the tests.
|It has source files, docs and data.
|
|TODO: write more
F notes.txt
|first note
|second note
:last line has no newline
F with space.txt
|a file whose name has a space
F .hidden
|hidden at the top
F Makefile
|all: main
|
|main: src/main.c src/util.c
|	cc -o main src/main.c src/util.c
|
|clean:
|	rm -f main
F src/main.c
|#include <stdio.h>
|#include "util.h"
|
|/* TODO: parse arguments */
|int main(int argc, char **argv) {
|    int total = add(1, 2);
|    printf("total = %d\n", total);
|    return 0;
|}
F src/util.c
|#include "util.h"
|
|int add(int a, int b) {
|    return a + b;
|}
|
|int sub(int a, int b) {
|    return a - b;
|}
|
|// todo: multiply
F src/util.h
|#ifndef UTIL_H
|#define UTIL_H
|
|int add(int a, int b);
|int sub(int a, int b);
|
|#endif
F src/.config
|debug=1
|level=3
F src/lib/lex.c
|#include "lex.h"
|
|int next_token(const char *s) {
|    /* TODO */
|    return s[0];
|}
F src/lib/lex.h
|int next_token(const char *s);
F src/lib/parse.c
|#include "lex.h"
|
|int parse(const char *s) {
|    int t = next_token(s);
|    return t == 0 ? -1 : t;
|}
F docs/guide.md
|# Guide
|
|Run `make` to build.
|Run `./main` to start.
|
|## Options
|
|There are no options yet.
F docs/api.md
|# API
|
|- add(a, b): returns a + b
|- sub(a, b): returns a - b
F data/numbers.txt
|1
|2
|3
|4
|5
|6
|7
|8
|9
|10
|11
|12
|13
|14
|15
|16
|17
|18
|19
|20
F data/words.txt
|apple
|banana
|cherry
|Apple pie
|banana split
|
|grape
|PINEAPPLE
|foo bar baz
|foo.bar
|foo*bar
|a+b=c
|(parens) [brackets] {braces}
|back\slash
|tab	here
|  leading spaces
|trailing spaces  
|ab
|abab
|abcabc
F data/blank.txt
F data/one.txt
|only line
F data/unicode.txt
|café au lait
|naïve résumé
|日本語のテキスト
|plain ascii
|é
F data/long.txt
|line 1: the quick brown fox jumps over the lazy dog
|line 2: the quick brown fox jumps over the lazy dog
|line 3: the quick brown fox jumps over the lazy dog
|line 4: the quick brown fox jumps over the lazy dog
|line 5: the quick brown fox jumps over the lazy dog
|line 6: the quick brown fox jumps over the lazy dog
|line 7: the quick brown fox jumps over the lazy dog
|line 8: the quick brown fox jumps over the lazy dog
|line 9: the quick brown fox jumps over the lazy dog
|line 10: the quick brown fox jumps over the lazy dog
|line 11: the quick brown fox jumps over the lazy dog
|line 12: the quick brown fox jumps over the lazy dog
|line 13: the quick brown fox jumps over the lazy dog
|line 14: the quick brown fox jumps over the lazy dog
|line 15: the quick brown fox jumps over the lazy dog
|line 16: the quick brown fox jumps over the lazy dog
|line 17: the quick brown fox jumps over the lazy dog
|line 18: the quick brown fox jumps over the lazy dog
|line 19: the quick brown fox jumps over the lazy dog
|line 20: the quick brown fox jumps over the lazy dog
|line 21: the quick brown fox jumps over the lazy dog
|line 22: the quick brown fox jumps over the lazy dog
|line 23: the quick brown fox jumps over the lazy dog
|line 24: the quick brown fox jumps over the lazy dog
|line 25: the quick brown fox jumps over the lazy dog
|line 26: the quick brown fox jumps over the lazy dog
|line 27: the quick brown fox jumps over the lazy dog
|line 28: the quick brown fox jumps over the lazy dog
|line 29: the quick brown fox jumps over the lazy dog
|line 30: the quick brown fox jumps over the lazy dog
|line 31: the quick brown fox jumps over the lazy dog
|line 32: the quick brown fox jumps over the lazy dog
|line 33: the quick brown fox jumps over the lazy dog
|line 34: the quick brown fox jumps over the lazy dog
|line 35: the quick brown fox jumps over the lazy dog
|line 36: the quick brown fox jumps over the lazy dog
|line 37: the quick brown fox jumps over the lazy dog
|line 38: the quick brown fox jumps over the lazy dog
|line 39: the quick brown fox jumps over the lazy dog
|line 40: the quick brown fox jumps over the lazy dog
|line 41: the quick brown fox jumps over the lazy dog
|line 42: the quick brown fox jumps over the lazy dog
|line 43: the quick brown fox jumps over the lazy dog
|line 44: the quick brown fox jumps over the lazy dog
|line 45: the quick brown fox jumps over the lazy dog
|line 46: the quick brown fox jumps over the lazy dog
|line 47: the quick brown fox jumps over the lazy dog
|line 48: the quick brown fox jumps over the lazy dog
|line 49: the quick brown fox jumps over the lazy dog
|line 50: the quick brown fox jumps over the lazy dog
|line 51: the quick brown fox jumps over the lazy dog
|line 52: the quick brown fox jumps over the lazy dog
|line 53: the quick brown fox jumps over the lazy dog
|line 54: the quick brown fox jumps over the lazy dog
|line 55: the quick brown fox jumps over the lazy dog
|line 56: the quick brown fox jumps over the lazy dog
|line 57: the quick brown fox jumps over the lazy dog
|line 58: the quick brown fox jumps over the lazy dog
|line 59: the quick brown fox jumps over the lazy dog
|line 60: the quick brown fox jumps over the lazy dog
F deep/a/b/c/leaf.txt
|leaf
F deep/a/sibling.txt
|sibling
D empty
D deep/a/b/hollow
