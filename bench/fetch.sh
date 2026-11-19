#!/bin/sh
# Downloads and unpacks the real benchmark datasets listed in
# bench/datasets.txt. They are upstream release tarballs as carried by the
# Ubuntu archive. Usage: bench/fetch.sh [DIR]   (default ../qdb-data)
set -eu
dir=${1:-../qdb-data}
base=https://archive.ubuntu.com/ubuntu/pool
mkdir -p "$dir/dl" "$dir/src"
cd "$dir/dl"
for u in \
    universe/r/redis/redis_8.0.6.orig.tar.gz \
    main/p/postgresql-16/postgresql-16_16.15.orig.tar.gz \
    main/l/linux/linux_6.8.0.orig.tar.gz \
    main/g/git/git_2.55.0.orig.tar.xz
do
    f=${u##*/}
    [ -f "$f" ] || curl -fsS -O "$base/$u"
    tar -xf "$f" -C ../src
done
# GNU awk, the reference for awk: Ubuntu's own awk is mawk. The package
# and the one library it needs are unpacked, not installed.
mkdir -p ../gawk/bin
for u in main/g/gawk/gawk_5.2.1-2build3_amd64.deb main/libs/libsigsegv/libsigsegv2_2.14-1ubuntu2_amd64.deb
do
    f=${u##*/}
    [ -f "$f" ] || curl -fsS -O "$base/$u"
    dpkg -x "$f" ../gawk/root
done
ln -sf ../root/usr/bin/gawk ../gawk/bin/awk
# The RFC tarball has no folder of its own.
f=doc-rfc_20170121.orig.tar.gz
[ -f "$f" ] || curl -fsS -O "$base/multiverse/d/doc-rfc/$f"
mkdir -p ../src/rfc-20170121
tar -xf "$f" -C ../src/rfc-20170121
sha256sum -c - <<SUMS
763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e  redis_8.0.6.orig.tar.gz
4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b  postgresql-16_16.15.orig.tar.gz
26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8  linux_6.8.0.orig.tar.gz
457fdb04dc8728e007d4688695e6912e6f680727920f2a40bf11eacc17505357  git_2.55.0.orig.tar.xz
7d0d6d76ade0a75cd87635b4794a407e1f845ad9fe0c849b660fcce9196855b2  doc-rfc_20170121.orig.tar.gz
4b662062e90f62ece26ca7adfcd3a33207c7862f3dc77b8df515b5680ee00ca9  gawk_5.2.1-2build3_amd64.deb
889f338bea4ebfd999939d946da4ac0a45668264a5aae78e023feeb0529505b8  libsigsegv2_2.14-1ubuntu2_amd64.deb
SUMS
