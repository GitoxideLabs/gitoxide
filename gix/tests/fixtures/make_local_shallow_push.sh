#!/usr/bin/env bash
set -eu -o pipefail

# Two source views share the same history: one stops at the newest existing
# commit, while the other retains every parent. Receivers cover complete,
# shallow, and empty histories, with Git recording each receive outcome.
git init -q -b main full
(
    cd full
    for value in one two three; do
        printf '%s\n' "$value" >file
        git add file
        git commit -qm "$value"
    done
)
git clone -q --bare full complete-template
git clone -q --bare --depth=2 "file://$PWD/full" shallow-template
git init -q --bare empty-template
git clone -q --depth=1 "file://$PWD/full" shallow-source
for source in full shallow-source; do
    (
        cd "$source"
        printf 'four\n' >file
        git add file
        git commit -qm four
    )
done

make_case() {
    local name=$1 source=$2 template=$3 allow=$4
    cp -a "$template" "native-$name"
    cp -a "$template" "git-$name"
    for kind in native git; do
        git -C "$kind-$name" config receive.shallowUpdate "$allow"
    done
    printf '%s' "$source" >"$name-source-name"
    if git -C "$source" push "$PWD/git-$name" main:refs/heads/topic >"$name-output" 2>&1; then
        printf success >"$name-status"
    else
        printf rejected >"$name-status"
    fi
    git -C "git-$name" fsck --strict --no-reflogs >"$name-fsck" 2>&1
}
make_case complete shallow-source complete-template false
make_case shallow shallow-source shallow-template false
make_case denied shallow-source empty-template false
make_case allowed shallow-source empty-template true
make_case full-to-shallow full shallow-template false

# Two incoming branches need different boundaries. Hooks inspect their graphs
# while still quarantined, then reject either one update or the whole receive.
(
    cd full
    git checkout -q --orphan other
    printf 'other root\n' >file
    git commit -qam 'other root'
    printf 'other base\n' >file
    git commit -qam 'other base'
    git checkout -q main
)
git clone -q --depth=1 --no-single-branch "file://$PWD/full" multi-source
(
    cd multi-source
    printf 'five\n' >file
    git commit -qam five
    git checkout -qb other origin/other
    printf 'other\n' >file
    git commit -qam other
)
for mode in partial atomic prereject; do
    for kind in native git; do
        target="$kind-hook-$mode"
        cp -a empty-template "$target"
        git -C "$target" config receive.shallowUpdate true
        cat >"$target/hooks/pre-receive" <<'HOOK'
#!/bin/sh
while read old new ref; do
    count=$(git rev-list --count "$new") || exit 1
    printf '%s %s\n' "$ref" "$count" >>hook-log
done
HOOK
        if test "$mode" = prereject; then
            printf 'exit 1\n' >>"$target/hooks/pre-receive"
        fi
        cat >"$target/hooks/update" <<'HOOK'
#!/bin/sh
if test "$1" = refs/heads/reject; then
    exit 1
fi
git rev-list --count "$3" >>update-log
HOOK
        chmod +x "$target/hooks/pre-receive" "$target/hooks/update"
    done
    args=("$PWD/git-hook-$mode")
    if test "$mode" = atomic; then
        args=(--atomic "${args[@]}")
    fi
    # Git 2.53's per-command shallow connectivity can traverse another
    # incoming branch before its boundary is installed. Independent pushes
    # provide the reference outcome for independent, non-atomic commands.
    refs=(main:refs/heads/accept other:refs/heads/reject)
    if test "$mode" = partial; then
        git -C multi-source push "$PWD/git-hook-$mode" main:refs/heads/accept >"hook-$mode-first-output" 2>&1
        refs=(other:refs/heads/reject)
    fi
    if git -C multi-source push "${args[@]}" "${refs[@]}" >"hook-$mode-output" 2>&1; then
        echo 'the hook must reject at least one update' >&2
        exit 1
    fi
    git -C "git-hook-$mode" fsck --strict --no-reflogs >"hook-$mode-fsck" 2>&1
 done
