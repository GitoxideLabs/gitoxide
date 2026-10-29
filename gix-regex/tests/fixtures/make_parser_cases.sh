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

make_case chained-range xfuncname '[a-b-c]' '-'
make_case literal-escapes xfuncname '\a\d\n\r\t\0' adnrt0
make_case word-class xfuncname '\w+' _word1
make_case nonword-class xfuncname '\W+' '!@'
make_case word-boundary xfuncname '\b(word)\b' word
make_case inside-word xfuncname '\B(oo)\B' food
make_case word-start-requires-word xfuncname '\<(!)' '!'
make_case word-end-requires-word xfuncname '(!)\>' '!'
make_case outside-word xfuncname '\B(!)' '!'
make_case absolute-anchors xfuncname "\\\`(word)\\'" word
make_case basic-middle-start funcname 'a^b' 'a^b'
make_case basic-middle-end funcname 'a$b' 'a$b'
make_case basic-second-start funcname '^^a' '^a'
make_case basic-alternative-start funcname 'a\|^b' b
make_case basic-alternative-end funcname 'a$\|b' a
make_case basic-group-anchors funcname '\(^a$\)' a

make_case leading-star xfuncname '*a' a
make_case leading-plus xfuncname '+a' a
make_case leading-question xfuncname '?a' a
make_case leading-range xfuncname '{1}a' a
make_case excessive-lower-bound xfuncname 'a{32768}' a
make_case excessive-upper-bound xfuncname 'a{1,32768}' a
make_case incorrect-basic-delimiter funcname 'a\{1}' a
make_case incorrect-extended-delimiter xfuncname 'a{1\}' a
make_case empty-range xfuncname 'a{}' a
make_case descending-range xfuncname 'a{2,1}' a
make_case omitted-lower-bound xfuncname 'a{,1}' a
make_case omitted-basic-lower-bound funcname 'a\{,1\}' a
make_case omitted-upper-bound xfuncname 'a{1,}' a
make_case omitted-basic-upper-bound funcname 'a\{1,\}' a

make_case empty-alternative xfuncname '(a|)b' b
make_case leading-empty-alternative xfuncname '(|a)b' b
make_case zero-repetitions xfuncname '(a){0}' a
make_case required-empty-repetitions xfuncname '(a?){2}b' b
make_case repeated-long-alternative xfuncname '(aa|a)*' aa
make_case repeated-short-alternative xfuncname '(a|aa)*' aa
make_case incomplete-interval xfuncname 'a{' a
make_case textual-interval xfuncname 'a{foo}' a

make_case nested-interval xfuncname 'a{2}{3}' aaaaaaa
make_case repeated-pair xfuncname 'a{2}*' aaa
make_case nested-star-plus xfuncname 'a*+' aaa
make_case nested-plus-question xfuncname '(a+?)b' b
make_case basic-nested-plus-question funcname '\(a\+\?\)b' b
make_case basic-repeated-pair funcname 'a\{2\}\+' aaa
make_case repeated-anchor xfuncname '^*' a
make_case repeated-word-boundary xfuncname '\b+' a
make_case basic-nested-stars funcname 'a**' aaa
make_case basic-nested-interval funcname 'a*\{2\}' aaa
make_case basic-leading-interval funcname '\{1\}' a
make_case basic-star-after-anchor funcname '^*a' '*a'
