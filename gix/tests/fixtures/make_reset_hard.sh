#!/usr/bin/env bash
set -eu -o pipefail

# Two complete trees exercise restored deletions, removed additions, and both
# directions of a file/directory transition. Tests layer dirty and staged states
# onto the second commit, then reset to the first with Git and gitoxide.
git init -q -b main
mkdir -p file-to-dir
printf 'target\n' >changed
printf 'same\n' >same
printf 'restored\n' >recreated
printf 'target-only\n' >target-only
printf 'target child\n' >file-to-dir/child
printf 'target file\n' >dir-to-file
git add .
git commit -qm target
git tag target

rm -r file-to-dir
rm dir-to-file target-only
mkdir -p dir-to-file obsolete-dir
printf 'old file\n' >file-to-dir
printf 'old child\n' >dir-to-file/child
printf 'old-only\n' >old-only
printf 'old leaf\n' >obsolete-dir/leaf
printf 'old\n' >changed
git add -A
git commit -qm previous

case "${1:-plain}" in
    filtered|filter-failure)
        # Install checkout filters only after committing so fixture creation
        # does not depend on the filter succeeding.
        printf '*.txt filter=reset\ncrlf.txt -filter text eol=crlf\n' >.gitattributes
        printf 'filtered content\n' >filtered.txt
        printf 'line one\nline two\n' >crlf.txt
        git add .gitattributes filtered.txt crlf.txt
        git commit -qm filtered
        git tag -f target
        git reset --soft HEAD^
        if test "$1" = filtered; then
            git config filter.reset.smudge 'tr a-z A-Z'
        else
            git config filter.reset.smudge false
        fi
        git config filter.reset.required true
        printf 'dirty\n' >filtered.txt
        printf 'dirty\n' >crlf.txt
        ;;
esac
