#!/usr/bin/env bash
set -eu -o pipefail

# updateInstead delegates to push-to-checkout when present. The hook receives
# the owning worktree's Git directory and root, even when receive-pack opened
# the main repository and the updated branch is checked out in a linked one.
git init -q -b main source
(
    cd source
    printf 'old\n' >file
    git add file
    git commit -qm old
)
for mode in main-accept main-reject linked-accept linked-reject \
    main-configured-accept main-configured-reject linked-configured-accept linked-configured-reject missing; do
    for kind in native git; do
        target="$kind-$mode"
        git clone -q source "$target"
        git -C "$target" config receive.denyCurrentBranch updateInstead
        worktree="$target"
        if [[ "$mode" = linked-* ]]; then
            worktree="$target-worktree"
            git -C "$target" worktree add -qb deploy "$PWD/$worktree"
        fi
        printf '%s' "$worktree" >"$target-worktree-name"
        mkdir -p "$target/.git/custom-hooks"
        git -C "$target" config core.hooksPath custom-hooks
        cat >"$target/.git/custom-hooks/update" <<'HOOK'
#!/bin/sh
printf '%s\n' "$1" >update-called
HOOK
        chmod +x "$target/.git/custom-hooks/update"
        if test "$mode" != missing; then
            cat >"$target/.git/custom-hooks/push-to-checkout" <<'HOOK'
#!/bin/sh
# Receive hooks still run from the receiver's Git directory. Only the worktree
# environment changes, and the ordinary update hook has already approved it.
test -f update-called || exit 2
printf '%s\n' "$1" >"$GIT_WORK_TREE/checkout-id"
printf '%s\n' "$GIT_DIR" >"$GIT_WORK_TREE/checkout-git-dir"
HOOK
            if [[ "$mode" = *-reject ]]; then
                printf 'exit 1\n' >>"$target/.git/custom-hooks/push-to-checkout"
            else
                cat >>"$target/.git/custom-hooks/push-to-checkout" <<'HOOK'
# A hook can intentionally replace dirty tracked contents; receive-pack does
# not apply its built-in clean-worktree policy when the hook is present.
git read-tree --reset -u "$1"
HOOK
            fi
            chmod +x "$target/.git/custom-hooks/push-to-checkout"
            if [[ "$mode" = *-configured-* ]]; then
                # Configured commands use the receive directory too, and must count
                # as an installed deployment hook when no traditional hook exists.
                mv "$target/.git/custom-hooks/push-to-checkout" "$target/.git/custom-hooks/deploy"
                git -C "$target" config hook.deploy.command ./custom-hooks/deploy
                git -C "$target" config hook.deploy.event push-to-checkout
            fi
            printf 'dirty\n' >"$worktree/file"
        fi
    done
done
(
    cd source
    printf 'new\n' >file
    git commit -qam new
)
for mode in main-accept main-reject linked-accept linked-reject \
    main-configured-accept main-configured-reject linked-configured-accept linked-configured-reject; do
    branch=main
    if [[ "$mode" = linked-* ]]; then
        branch=deploy
    fi
    if git -C source push "$PWD/git-$mode" "main:refs/heads/$branch" >"$mode-output" 2>&1; then
        printf success >"$mode-status"
    else
        printf rejected >"$mode-status"
    fi
done
