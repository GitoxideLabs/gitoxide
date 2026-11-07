#!/usr/bin/env bash
set -eu -o pipefail

git init -q

echo a > a
echo b > b
echo c > c
echo d > d
echo e > e-no-attr
echo unset > unset

cat <<EOF >.gitattributes
a diff=a
b diff=b
c diff=c
d diff=d
missing diff=missing
unset -diff
EOF

git add . && git commit -m "init"

# Present, unhashed empty files must remain distinct from missing rewrite resources.
: > empty-old
: > empty-new
