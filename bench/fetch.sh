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
sha256sum -c - <<SUMS
763bf3005bcbf77c1d8145016a9627e45219665f99cf5a736bc73ec70faf0c8e  redis_8.0.6.orig.tar.gz
4f200ca23dfb120ff9838f13ce06014aad1d3c432d16ee9f93ab2000c0eeef7b  postgresql-16_16.15.orig.tar.gz
26512115972bdf017a4ac826cc7d3e9b0ba397d4f85cd330e4e4ff54c78061c8  linux_6.8.0.orig.tar.gz
457fdb04dc8728e007d4688695e6912e6f680727920f2a40bf11eacc17505357  git_2.55.0.orig.tar.xz
SUMS
