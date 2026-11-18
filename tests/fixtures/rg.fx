# A tree for rg: a repository with ignore files of every kind, hidden
# files, files of several types and text that is not ASCII.
F .git/HEAD
|ref: refs/heads/main
F .git/info/exclude
|excluded.txt
|scratch/
F .gitignore
|# build output
|target/
|*.log
|!keep.log
|/top-only.txt
|docs/draft-*.md
|**/gen/*.rs
F .ignore
|vendor/
|*.min.js
F .rgignore
|!vendor/keep.js
|notes/private.txt
F README.md
|# Project needle
|
|A needle in a haystack. TODO: find it.
|Second line with Needle and NEEDLE.
F top-only.txt
|needle at the top only
F excluded.txt
|needle excluded by .git/info/exclude
F main.rs
|fn main() {
|    let needle = find("needle");
|    println!("{}", needle);
|}
F lib.c
|#include "lib.h"
|int needle(void) { return 42; }
F lib.h
|int needle(void);
F app.js
|const needle = require('./needle');
|// TODO needle twice needle
F app.min.js
|var n="needle";
F build.log
|needle in the log
F keep.log
|needle kept in a log
F .hidden.txt
|hidden needle
F .config/settings.toml
|needle = true
F src/a.py
|def needle():
|    return "needle"
F src/b.ts
|export const needle: string = "x";
F src/c.go
|func needle() {}
F src/style.css
|.needle { color: red; }
F src/page.html
|<p>needle</p>
F src/data.json
|{"needle": 1}
F src/conf.yaml
|needle: yes
F src/run.sh
|echo needle
F src/Cargo.lock
|needle = "1.0"
F src/top-only.txt
|needle below the top
F src/gen/out.rs
|// generated needle
F src/gen/out.txt
|generated text needle
F src/deep/.gitignore
|*.tmp
|!important.tmp
|local/
F src/deep/x.tmp
|needle tmp
F src/deep/important.tmp
|needle important
F src/deep/local/l.txt
|needle local
F src/deep/keep.txt
|needle deep
F target/debug/out.txt
|needle in target
F vendor/lib.js
|needle vendored
F vendor/keep.js
|needle vendored and kept
F scratch/s.txt
|needle scratch
F notes/private.txt
|needle private
F notes/public.txt
|needle public
|line two
|line three
|another needle here
|line five
|line six
|line seven
|last needle
F docs/draft-one.md
|needle draft
F docs/final.md
|needle final
|context a
|context b
F docs/unicode.txt
|naïve needle café
|Ünïcödé NEEDLE straße
|日本語 needle テキスト
|plain ascii line
|needleécole
|éneedle
|x_needle needle_y
F sub/.git/HEAD
|ref: refs/heads/other
F sub/.gitignore
|ignored-in-sub.txt
F sub/ignored-in-sub.txt
|needle ignored in sub
F sub/seen.txt
|needle seen in sub
F nogit/.gitignore
|never.txt
F nogit/never.txt
|needle never
D emptydir
