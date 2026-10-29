#!/usr/bin/env bash
set -eu -o pipefail

# Configure both POSIX dialects in differing key order. Invalid regular
# expressions must stay dormant until their driver is actually selected.
git init -q
git config --add diff.basic-last.xfuncname '^wrong (.*)$'
git config --add diff.basic-last.funcname '^prefix \(.*\)$'
git config --add diff.extended-last.funcname '^wrong \(.*\)$'
git config --add diff.extended-last.xfuncname '^prefix (.*)$'
git config diff.special.xfuncname '^prefix (.*)$'
git config diff.invalid.xfuncname '('
git config diff.rust.binary auto
git config diff.binary.binary true

mkdir nested
cat >.gitattributes <<'ATTR'
*.rs diff=rust
*.custom diff=special
*.invalid diff=invalid
*.unknown diff=unconfigured
*.binary -diff
*.text diff
*.unset !diff
ATTR
printf '%s\n' '*.rs diff=special' >nested/.gitattributes
mkdir -p .git/info
printf '%s\n' 'override.rs diff=special' >.git/info/attributes
git add .gitattributes nested/.gitattributes

# A named binary driver differs from an explicit -diff attribute. Text conversion
# permits a header for the former, once the caller has converted its content.
git config diff.converted.binary true
git config diff.converted.textconv cat
cat >>.gitattributes <<'ATTR'
*.driverbinary diff=binary
*.converted diff=converted
ATTR
git add .gitattributes

# Alternate roots use their own attributes while preserving repository-level
# info/attributes and the supplied index as a fallback.
mkdir alternate
printf '%s\n' '*.rs diff=special' >alternate/.gitattributes
