#!/usr/bin/env bash
set -eu -o pipefail

# A remote tip is beyond the actual boundary needed by local history. Another
# remote is disjoint but shares an object, and reflog-only objects have no tip.
git init -q -b main --object-format="${1:-sha1}" repo
printf 'shared\n' >repo/shared
git -C repo add shared
git -C repo commit -qm base
git -C repo rev-parse HEAD >base-id
git -C repo rev-parse HEAD^{tree} >base-tree
git -C repo rev-parse HEAD:shared >shared-blob
git -C repo tag -m 'old tag' old
printf 'remote base\n' >repo/base
git -C repo add base
git -C repo commit -qm boundary
git -C repo rev-parse HEAD >boundary-id
printf 'remote only\n' >repo/remote
git -C repo add remote
git -C repo commit -qm remote-tip
git -C repo update-ref refs/remotes/origin/main HEAD
git -C repo reset --hard -q HEAD^
printf 'local\n' >repo/local
git -C repo add local
git -C repo commit -qm local
git -C repo tag -m 'new tag' new
git -C repo tag -m 'nested tag' nested new

base_tree=$(cat base-tree)
independent=$(printf 'independent\n' | git -C repo commit-tree "$base_tree")
git -C repo update-ref refs/heads/independent "$independent"
git -C repo update-ref refs/data/tree "$base_tree"
git -C repo update-ref refs/data/blob "$(cat shared-blob)"
printf 'unreferenced\n' | git -C repo commit-tree "$base_tree" >unreferenced-id
git -C repo checkout -q --detach main

# Disable deltas so the oracle's payload object set can be inspected without
# adding external delta bases while fixing a thin pack.
bundle_case () {
    local name=$1
    shift
    if git -C repo -c pack.window=0 bundle create "../$name.bundle" "$@" >"$name.stdout" 2>"$name.stderr"; then
        sed '/^$/q' "$name.bundle" >"$name.header"
        local header_length
        header_length=$(wc -c <"$name.header")
        tail -c +$((header_length + 1)) "$name.bundle" >"$name.pack"
        git -C repo index-pack -o "../$name.idx" "../$name.pack" >"$name.checksum"
        git -C repo verify-pack -v "../$name.idx" | awk '$2 == "commit" || $2 == "tag" || $2 == "tree" || $2 == "blob" { print $1 }' | sort >"$name.objects"
        git -C repo bundle list-heads "../$name.bundle" >"$name.refs"
    else
        touch "$name.failed"
    fi
}

bundle_case full --all HEAD
bundle_case incremental --all HEAD --not --remotes
bundle_case tag-excluded refs/tags/old ^refs/remotes/origin/main
bundle_case tree-excluded refs/data/tree ^refs/remotes/origin/main
bundle_case blob-excluded refs/data/blob ^refs/remotes/origin/main
bundle_case disjoint-shared refs/heads/independent ^refs/remotes/origin/main
bundle_case exclude-blob main ^refs/data/blob

bundle_case main-only main

# A merge crosses two remote boundaries with a shared ancestor; neither remote
# tip itself is needed by the receiver.
side=$(printf 'side boundary\n' | git -C repo commit-tree "$base_tree" -p "$(cat base-id)")
remote_side=$(printf 'remote side\n' | git -C repo commit-tree "$base_tree" -p "$side")
git -C repo update-ref refs/remotes/origin/side "$remote_side"
merge=$(printf 'local merge\n' | git -C repo commit-tree "main^{tree}" -p main -p "$side")
git -C repo update-ref refs/heads/merge "$merge"
bundle_case merge refs/heads/merge ^refs/remotes/origin/main ^refs/remotes/origin/side

# Ancestor exclusion must remain exact even when commit dates run backwards.
skew_old=$(printf 'future ancestor\n' | GIT_COMMITTER_DATE='2000000000 +0000' git -C repo commit-tree "$base_tree" -p "$(cat base-id)")
skew_new=$(printf 'past descendant\n' | GIT_COMMITTER_DATE='1000000000 +0000' git -C repo commit-tree "$base_tree" -p "$skew_old")
git -C repo update-ref refs/heads/skew-old "$skew_old"
git -C repo update-ref refs/remotes/origin/skew "$skew_new"

# Replacement refs must not change exported identity or hide original parents.
git -C repo replace main "$independent"

# A local shallow graft with available parents must not hide history from an
# unfiltered bundle. Actual missing parents are covered by an in-memory test.
git -C repo rev-parse main >repo/.git/shallow
