#!/usr/bin/env bash
set -eu -o pipefail

# Exercise POSIX byte semantics independently of the developer locale.
export LC_ALL=C

# Each repository gives Git one unchanged candidate line followed by a changed
# line. A zero-context diff exposes the selected capture in the hunk header.
# Invalid patterns fail while generating the diff; preserve that status as well.
mkdir cases
make_case() {
    local name=$1 syntax=$2 pattern=$3 source=$4
    mkdir "cases/$name"
    (
        cd "cases/$name"
        git init -q
        printf '%s\n' 'file diff=header' >.gitattributes
        printf '%s' "$pattern" >pattern
        printf '%s' "$source" >source
        printf '%s' "$syntax" >syntax
        git config "diff.header.$syntax" "$pattern"
        printf '%s\n    before\n' "$source" >file
        git add file
        printf '%s\n    after\n' "$source" >file
        if git diff --no-ext-diff --no-color --unified=0 -- file >baseline.diff 2>baseline.err; then
            printf '%s' valid >status
        else
            printf '%s' invalid >status
        fi
    )
}
make_case unclosed-ere xfuncname '(unclosed' unclosed
make_case unclosed-bre funcname '\(unclosed' unclosed
make_case unmatched-close funcname 'closed\)' closed
make_case close-is-literal xfuncname 'closed)literal' 'closed)literal'
make_case reversed-range xfuncname '[z-a]' a
make_case class-range-start xfuncname '[[:alpha:]-z]' a
make_case class-range-end xfuncname '[z-[:alpha:]]' z
make_case literal-bracket xfuncname '[[]' '['
make_case mixed-literal-bracket funcname '[a[]' '['
# 0xff is deliberate: a byte-range endpoint must not overflow at 255.
# Capture ASCII because Git truncates invalid UTF-8 when rendering hunk headers.
make_case maximum-byte xfuncname $'[\xff-\xff] (suffix)' $'\xff suffix'
make_case self-reference xfuncname '(\1)' a
make_case absent-reference xfuncname '(a)\2' aa
make_case first-digit-ere xfuncname '(a)\10' aa0
make_case first-digit-bre funcname '\(a\)\10' aa0
make_case nested-captures xfuncname '((ab)*)*' abab
