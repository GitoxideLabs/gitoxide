#!/usr/bin/env bash
set -eu

# Distinct default and namespaced tips reveal accidental namespace loss. Both
# ordinary and physical-name hideRefs patterns have explicit visible exceptions.
git init -b main source
(
    cd source
    git commit --allow-empty -m default
    default_tip=$(git rev-parse HEAD)
    git commit --allow-empty -m namespaced
    namespace_tip=$(git rev-parse HEAD)
    git update-ref refs/heads/main "$default_tip"
    prefix=refs/namespaces/team/refs/namespaces/project
    for name in topic private/hidden private/open full/hidden full/open; do
        git update-ref "$prefix/refs/heads/$name" "$namespace_tip"
    done
    git symbolic-ref "$prefix/HEAD" "$prefix/refs/heads/topic"
    git config uploadpack.hideRefs refs/heads/private
    git config --add uploadpack.hideRefs '!refs/heads/private/open'
    git config transfer.hideRefs "^$prefix/refs/heads/full"
    git config --add transfer.hideRefs "!^$prefix/refs/heads/full/open"
    git pack-refs --all --prune
)
git init -b unborn empty
