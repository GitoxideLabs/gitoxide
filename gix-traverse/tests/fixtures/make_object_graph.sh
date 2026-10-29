#!/usr/bin/env bash
set -eu -o pipefail

# A shared tree and blob across two commits, nested annotated tags, a blob tag,
# and an unavailable submodule commit exercise all Git object-graph edges.
git init -q
mkdir dir
printf 'shared\n' >dir/shared
printf 'first\n' >changed
git add .
git commit -qm first
printf 'second\n' >changed
git add changed
foreign=$(printf 'not stored\n' | git hash-object --stdin)
git update-index --add --cacheinfo "160000,$foreign,submodule"
git commit -qm second
git tag -am inner inner
git tag -am outer outer inner
git tag -am blob blob-tag "HEAD:dir/shared"
git for-each-ref --format='%(objectname)' >tips
git rev-list --objects --all --no-object-names >reachable
git rev-parse HEAD >tip
git rev-parse HEAD >.git/shallow
git rev-list --objects --no-object-names HEAD >shallow-reachable
rm .git/shallow

# Keep corrupt objects unreferenced so ordinary reachability is still valid.
# A tag claims a blob is a commit, and a tree is truncated before its object ID.
blob=$(git rev-parse HEAD:dir/shared)
printf '%s\n' "$blob" >blob
printf 'object %s\ntype commit\ntag wrong-kind\n\nmessage\n' "$blob" |
    git hash-object -w --literally -t tag --stdin >wrong-kind
printf '100644 file\0' | git hash-object -w --literally -t tree --stdin >malformed
