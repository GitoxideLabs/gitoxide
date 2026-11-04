#!/usr/bin/env bash
set -eu -o pipefail

# Each object has its own repository so Git's full scan cannot mix unrelated
# diagnostics. Valid dependencies make the results about object syntax only.
git init -q --bare template
tree_id=$(git -C template hash-object -w -t tree --stdin </dev/null)
blob_id=$(printf content | git -C template hash-object -w --stdin)
ident='A U Thor <author@example.com> 1 +0000'

make_case() {
    local name=$1 kind=$2
    mkdir "$name"
    cp -a template "$name/repo"
    cat >"$name/object"
    printf '%s' "$kind" >"$name/kind"
    git -C "$name/repo" hash-object -w --literally -t "$kind" --stdin <"$name/object" >"$name/id"
    if git -C "$name/repo" fsck --strict --no-reflogs --no-dangling >"$name/report" 2>&1; then
        printf success >"$name/status"
    else
        printf rejected >"$name/status"
    fi
}
commit_with_ident() {
    printf 'tree %s\nauthor %s\ncommitter %s\n\nmessage\n' "$tree_id" "$1" "$ident"
}
commit_with_ident "$ident" | make_case valid-commit commit
commit_with_ident 'A U Thor<author@example.com> 1 +0000' | make_case missing-space-email commit
commit_with_ident '<author@example.com> 1 +0000' | make_case missing-name commit
commit_with_ident 'A U Thor 1 +0000' | make_case missing-email commit
commit_with_ident 'A > Thor <author@example.com> 1 +0000' | make_case bad-name commit
commit_with_ident 'A U Thor <author<example.com> 1 +0000' | make_case bad-email commit
commit_with_ident 'A U Thor <author@example.com>1 +0000' | make_case missing-space-date commit
commit_with_ident 'A U Thor <author@example.com> -1 +0000' | make_case bad-date commit
commit_with_ident 'A U Thor <author@example.com> 01 +0000' | make_case padded-date commit
# Git's unsigned timestamp parser also checks conversion to signed system time_t.
commit_with_ident 'A U Thor <author@example.com> 9223372036854775807 +0000' | make_case maximum-date commit
commit_with_ident 'A U Thor <author@example.com> 9223372036854775808 +0000' | make_case signed-overflow-date commit
commit_with_ident 'A U Thor <author@example.com> 18446744073709551615 +0000' | make_case unsigned-maximum-date commit
commit_with_ident 'A U Thor <author@example.com> 18446744073709551616 +0000' | make_case overflow-date commit
commit_with_ident 'A U Thor <author@example.com> 1 +00' | make_case bad-timezone commit
printf 'tree %s\nauthor %s\nauthor %s\ncommitter %s\n\nmessage\n' "$tree_id" "$ident" "$ident" "$ident" | make_case duplicate-author commit
printf 'tree %s\ncommitter %s\n\nmessage\n' "$tree_id" "$ident" | make_case missing-author commit
printf 'tree %s\nauthor %s\n\nmessage\n' "$tree_id" "$ident" | make_case missing-committer commit
printf 'author %s\ncommitter %s\n\nmessage\n' "$ident" "$ident" | make_case missing-tree commit
printf 'tree %s\nauthor %s\ncommitter %s\n' "$tree_id" "$ident" "$ident" | make_case no-message-separator commit
printf 'tree %s\nauthor %s\ncommitter %s' "$tree_id" "$ident" "$ident" | make_case unterminated commit
printf 'tree %s\nauthor A\0 B\n\nmessage\n' "$tree_id" | make_case nul-header commit
printf 'tree %s\nauthor %s\ncommitter %s\n\nmessage\0tail\n' "$tree_id" "$ident" "$ident" | make_case nul-message commit
printf 'object %s\ntype blob\ntag v1\ntagger %s\n\nmessage\n' "$blob_id" "$ident" | make_case valid-tag tag
printf 'object %s\ntype blob\ntag v1\n\nmessage\n' "$blob_id" | make_case missing-tagger tag
printf 'object %s\ntype blob\ntag bad..name\ntagger %s\n\nmessage\n' "$blob_id" "$ident" | make_case bad-tag-name tag
printf 'object %s\ntype strange\ntag v1\ntagger %s\n\nmessage\n' "$blob_id" "$ident" | make_case bad-tag-type tag
printf 'object %s\ntype blob\ntag v1\ntagger %s\nextra data\n\nmessage\n' "$blob_id" "$ident" | make_case extra-tag-header tag

tree_entry() {
    printf '%s %s\0' "$1" "$2"
    printf '%b' "$(printf '%s' "${3:-$blob_id}" | sed 's/../\\x&/g')"
}
tree_entry 100644 file | make_case valid-tree tree
tree_entry 0100644 file | make_case padded-mode tree
tree_entry 100664 file | make_case historical-mode tree
tree_entry 100644 a/b | make_case full-path tree
tree_entry 100644 '' | make_case empty-name tree
tree_entry 100644 . | make_case dot-name tree
tree_entry 100644 .. | make_case dotdot-name tree
tree_entry 100644 .git | make_case dotgit-name tree
{ tree_entry 100644 z; tree_entry 100644 a; } | make_case unsorted-tree tree
{ tree_entry 100644 a; tree_entry 40000 a "$tree_id"; } | make_case duplicate-tree tree
