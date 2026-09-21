#!/usr/bin/env bash
set -eu -o pipefail

git init -q

# Negating a nested directory cannot re-include descendants of the ignored tld.
mkdir -p tld/sd/nested
cat <<EOF >.gitignore
# directory exclude
tld/

!tld/file
EOF

cat <<EOF >tld/.gitignore
sd/
!sd/

!file
EOF

git check-ignore -vn --stdin 2>&1 <<EOF >git-check-ignore.baseline || :
tld
tld/
tld/file
tld/sd
tld/sd/
tld/sd/file
tld/sd/nested/file
EOF
