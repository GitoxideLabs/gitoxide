#!/usr/bin/env bash
set -eu -o pipefail

# Create one isolated diff per matcher case. The source declaration precedes an
# indented change, so Git's zero-context hunk header reveals exactly the selected
# declaration. Control characters are intentional: Git preserves VT and FF but
# trims spaces, tabs, CR and LF from the selected header.
mkdir cases
make_case() {
    local name=$1 line=$2
    mkdir "cases/$name"
    (
        cd "cases/$name"
        git init -q
        printf '%s\n' 'file diff=header' >.gitattributes
        if [[ $# -eq 3 ]]; then
            printf '%s' "$3" >pattern
            git config diff.header.xfuncname "$3"
        fi
        printf '%s\n' "$line" >source
        printf '%s\n    same\n    before\n' "$line" >file
        git add file
        printf '%s\n    same\n    after\n' "$line" >file
        git diff --no-ext-diff --no-color --unified=0 -- file >baseline.diff
    )
}
make_case default 'function() {'
make_case trailing-tab $'_private() {\t '
make_case vertical-tab $'trailing_vertical_tab\x0b'
make_case form-feed $'trailing_form_feed\x0c'
make_case indented '    not_a_header'
make_case capture $'prefix declaration\t' '^prefix (.*)$'
make_case capture-vertical-tab $'prefix vertical_tab\x0b' '^prefix (.*)$'
make_case capture-form-feed $'prefix form_feed\x0c' '^prefix (.*)$'
make_case negative 'skip declaration' '!^skip.*
^prefix (.*)$'
make_case crlf $'other declaration\r' '^other.*'
