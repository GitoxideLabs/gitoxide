#!/usr/bin/env bash
set -eu -o pipefail

# Git's fsck discovers these blobs through tree names, including filesystem
# aliases. Isolated repositories prevent diagnostics from mixing across cases.
git init -q --bare template
empty_tree_id=$(git -C template hash-object -w -t tree --stdin </dev/null)

finish_case() {
    local case_name=$1 entry_name=$2 mode=$3 entry_id=$4
    printf '%s %s\0' "$mode" "$entry_name" >"$case_name/tree"
    printf '%b' "$(printf '%s' "$entry_id" | sed 's/../\\x&/g')" >>"$case_name/tree"
    git -C "$case_name/repo" hash-object -w --literally -t tree --stdin <"$case_name/tree" >"$case_name/tree-id"
    if git -C "$case_name/repo" fsck --strict --no-reflogs --no-dangling >"$case_name/report" 2>&1; then
        printf success >"$case_name/status"
    else
        printf rejected >"$case_name/status"
    fi
}
blob_case() {
    local case_name=$1 entry_name=${2:-.gitmodules} mode=${3:-100644}
    mkdir "$case_name"
    cp -a template "$case_name/repo"
    local blob_id
    blob_id=$(git -C "$case_name/repo" hash-object -w --stdin)
    printf '%s' "$blob_id" >"$case_name/blob-id"
    finish_case "$case_name" "$entry_name" "$mode" "$blob_id"
}
url_case() {
    # Git config doubles backslashes; the URL itself remains byte-for-byte intact.
    local quoted=${2//\\/\\\\}
    printf '[submodule "ok"]\nurl = "%s"\n' "$quoted" | blob_case "url-$1"
}
printf '[submodule "ok"]\npath = nested/repo\nurl = ./repo\n' | blob_case valid-modules
printf '[submodule ""]\n' | blob_case empty-section
printf '[submodule ""]\nbare\n' | blob_case empty-name
printf '[submodule "a/../b"]\npath = x\n' | blob_case parent-name
printf '[submodule "a\\\\..\\\\b"]\npath = x\n' | blob_case backslash-name
printf '[submodule "ok"]\npath = -repo\n' | blob_case option-path
printf '[submodule "ok"]\nupdate = !command\n' | blob_case command-update
printf '[SuBmOdUlE "ok"]\nURL = -bad\nurl = ./good\n' | blob_case duplicate-url
printf '[submodule "ok"]\nurl\npath\nupdate\n' | blob_case implicit-values
printf '[include]\npath = /does/not/exist\n' | blob_case no-includes
printf '[submodule "unterminated]\n' | blob_case parse-error
printf '[submodule "ok"]\nurl = -unsafe\n[broken' | blob_case parse-after-unsafe-url
printf '[submodule "ok"]\nupdate = !command\npath = "unfinished' | blob_case parse-after-command
url_case local ./bar/baz/foo.git
url_case https https://example.com/foo.git
url_case port http://example.com:80/deeper/foo.git
url_case option -a./foo
url_case root ../../..//test/foo.git
url_case helper-escape ../../../../../:localhost:8080/foo.git
url_case backslash-escape '..\../.\../:example.com/foo.git'
url_case relative-newline ./%0ahost=example.com/foo.git
url_case git-newline git://example.com/%0afoo
url_case query-newline 'https://one.example.com/evil?%0ahost=two.example.com'
url_case missing-host https:///example.com/foo.git
url_case bad-port http://example.com:test/foo.git
url_case helper-local https::example.com/foo.git
url_case helper-colon http:::example.com/foo.git
url_case helper-valid http::https://example.com/repo.git
url_case helper-file http::file:///tmp/repo
url_case root-dotdot https://example.com/../repo
url_case encoded-dotdot https://example.com/%2e%2e/repo
url_case normalized-newline https://example.com/%0a/../repo
url_case bad-percent https://example.com/%xx
url_case relative-percent ./repo%xx
url_case zero-port https://example.com:0/repo
url_case high-port https://example.com:65536/repo
url_case padded-port https://example.com:00000443/repo
url_case ipv6 'https://[::1]:443/repo'
url_case escaped-host https://ex%61mple.com/repo
url_case credentials-newline https://user%0a@example.com/repo
url_case ssh-newline ssh://example.com/%0arepo
url_case helper-scheme http::custom://example.com/repo

printf 'target\n' | blob_case modules-symlink .gitmodules 120000
printf 'target\n' | blob_case modules-ntfs 'GITMOD~1' 120000
printf 'target\n' | blob_case modules-hashed 'gi7eba~1' 120000
printf 'target\n' | blob_case modules-hfs $'.git\xe2\x80\x8cmodules' 120000
printf 'target\n' | blob_case modules-stream '.gitmodules::$DATA' 120000
printf 'target\n' | blob_case modules-backslash 'nested\.gitmodules' 120000
printf 'target\n' | blob_case attributes-symlink .gitattributes 120000
printf 'target\n' | blob_case attributes-ntfs 'GI7D29~1' 120000
printf 'target\n' | blob_case ignore-symlink .gitignore 120000
printf 'target\n' | blob_case mailmap-symlink .mailmap 120000
printf 'target\n' | blob_case ordinary-symlink unrelated 120000
printf 'target\n' | blob_case attributes-backslash 'nested\.gitattributes' 120000
printf '* text\n' | blob_case valid-attributes .gitattributes
printf '%2047s\n' ' ' | blob_case attributes-line-limit .gitattributes
printf '%2048s\n' ' ' | blob_case attributes-long-line .gitattributes
printf '%2048s' ' ' | blob_case attributes-long-final-line .gitattributes
# Every byte is LF, so the file boundary is independent of the line boundary.
head -c 104857600 /dev/zero | tr '\0' '\n' | blob_case attributes-size-limit .gitattributes
head -c 104857601 /dev/zero | tr '\0' '\n' | blob_case attributes-large .gitattributes

for special in gitmodules gitattributes; do
    mkdir "$special-tree" "$special-missing"
    cp -a template "$special-tree/repo"
    cp -a template "$special-missing/repo"
    finish_case "$special-tree" ".$special" 40000 "$empty_tree_id"
    missing_id=$(printf missing | git -C template hash-object --stdin)
    finish_case "$special-missing" ".$special" 100644 "$missing_id"
done

# One object can play both roles; discovering either name must not suppress the
# other validator when a blob is shared by differently named tree entries.
printf '[submodule "ok"]\nupdate = !%02050d\n' 0 | blob_case shared-special
shared_id=$(cat shared-special/blob-id)
{
    for name in .gitattributes .gitmodules; do
        printf '100644 %s\0' "$name"
        printf '%b' "$(printf '%s' "$shared_id" | sed 's/../\\x&/g')"
    done
} >shared-special/tree
git -C shared-special/repo hash-object -w -t tree --stdin <shared-special/tree >shared-special/tree-id
git -C shared-special/repo fsck --strict --no-reflogs --no-dangling >shared-special/report 2>&1 || :

# Skipping a tree suppresses its own diagnostics, but must not exempt unsafe
# special-file blobs discovered through that tree. Only skipping the blob does.
cp -a command-update skipped-tree-modules
cp skipped-tree-modules/tree-id skipped-tree-modules/skiplist
git -C skipped-tree-modules/repo -c fsck.skipList=../skiplist fsck --strict --no-reflogs --no-dangling >skipped-tree-modules/report 2>&1 || :
