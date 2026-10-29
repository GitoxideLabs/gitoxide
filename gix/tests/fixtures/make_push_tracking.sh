#!/usr/bin/env bash
set -eu -o pipefail

# Keep the advertised remote tip independent of the rewritten source tip.
# Deliberately dated reflogs distinguish the remote tip's presence, ancestry,
# the first older boundary entry, and an occurrence beyond that boundary.
git init -q --object-format="${1:?hash algorithm}" source
git init -q --bare --object-format="$1" destination
(
    cd source
    git config core.logAllRefUpdates true
    tree=$(git mktree </dev/null)
    base=$(printf 'base\n' | git commit-tree "$tree")
    remote=$(printf 'remote\n' | git commit-tree "$tree" -p "$base")
    descendant=$(printf 'descendant\n' | git commit-tree "$tree" -p "$remote")
    rewritten=$(printf 'rewritten\n' | git commit-tree "$tree" -p "$base")
    printf '%s\n' "$base" "$remote" "$descendant" "$rewritten" >../object-ids

    at() {
        GIT_COMMITTER_DATE="$1 +0000" git update-ref --create-reflog "$2" "$3"
    }
    at 1112912110 refs/heads/source "$remote"
    at 1112912120 refs/heads/source "$rewritten"
    for branch in included descendant boundary too-old not-included no-local; do
        git push -q ../destination "$remote:refs/heads/$branch"
        at 1112912100 "refs/remotes/origin/$branch" "$remote"
    done
    at 1112912110 refs/heads/included "$remote"
    at 1112912120 refs/heads/included "$rewritten"
    at 1112912110 refs/heads/descendant "$descendant"
    at 1112912120 refs/heads/descendant "$rewritten"
    at 1112912090 refs/heads/boundary "$remote"
    at 1112912120 refs/heads/boundary "$rewritten"
    at 1112912080 refs/heads/too-old "$remote"
    at 1112912090 refs/heads/too-old "$base"
    at 1112912120 refs/heads/too-old "$rewritten"
    at 1112912090 refs/heads/not-included "$base"
    at 1112912120 refs/heads/not-included "$rewritten"
    git remote add origin ../destination
)
