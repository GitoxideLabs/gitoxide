#!/usr/bin/env bash
set -eu

# A linear history, peeled and lightweight tags, aliases, and hidden refs exercise
# native local fetch advertisement and depth changes against upload-pack.
git init -b main source
(
    cd source
    for i in 1 2 3 4; do
        echo "$i" >file
        git add file
        git commit -m "commit $i"
    done
    git tag -m version annotated HEAD~1
    git tag light
    git branch private/hidden HEAD~2
    git branch private/visible
    git config uploadpack.hideRefs refs/heads/private
    git config --add uploadpack.hideRefs '!refs/heads/private/visible'
    git symbolic-ref refs/heads/alias refs/heads/main
    git symbolic-ref refs/heads/dangling refs/heads/missing
    # Object transfer must use the original object graph, not this local view.
    git replace HEAD HEAD~1
)
git init -b unborn empty

# Upload-pack suppresses replacement refs and honors this truncated source boundary.
git clone --depth 2 "file://$(pwd)/source" shallow
