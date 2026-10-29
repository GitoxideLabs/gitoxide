#!/usr/bin/env bash
set -eu -o pipefail

# Keep Git's expected objects separate so hashing can prove it stores nothing.
# The original index contains CRLF content, exercising text=auto's index fallback.
git init -q --object-format="${1:?hash algorithm}" repo
mkdir outside expected-objects
printf 'outside contents\n' >outside/file
(
    cd repo
    git config core.autocrlf false
    git config filter.upper.clean 'tr a-z A-Z'
    git config filter.upper.required true
    git config filter.fail.clean 'exit 1'
    git config filter.fail.required true
    printf 'old\r\n' >keep-crlf
    git add keep-crlf
    cat >.gitattributes <<'ATTRS'
filtered filter=upper
crlf text
keep-crlf text=auto
encoded working-tree-encoding=ISO-8859-1
binary text=auto
link filter=upper text
failure filter=fail
ATTRS
    git add .gitattributes
    printf 'plain contents\n' >plain
    printf 'filtered contents\n' >filtered
    printf 'normalized\r\n' >crlf
    printf 'preserved\r\n' >keep-crlf
    printf 'caf\351\n' >encoded
    printf 'binary\000bytes\377\r\n' >binary
    printf 'executable\n' >executable
    chmod +x executable
    printf 'failure\n' >failure
    args=(plain filtered crlf keep-crlf encoded binary executable)
    if ln -s plain link 2>/dev/null; then
        args+=(link)
        ln -s ../outside parent-link
    fi
    cp .git/index ../expected-index
    export GIT_INDEX_FILE="$PWD/../expected-index"
    export GIT_OBJECT_DIRECTORY="$PWD/../expected-objects"
    export GIT_ALTERNATE_OBJECT_DIRECTORIES="$PWD/.git/objects"
    git add -- "${args[@]}"
    git ls-files --stage -z -- "${args[@]}" >../expected-entries
)
