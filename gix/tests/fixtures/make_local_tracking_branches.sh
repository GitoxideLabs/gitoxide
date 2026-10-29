#!/usr/bin/env bash
set -eu -o pipefail

# Upstream relationships are configured names, independent of local branch
# spelling and whether a remote-tracking ref has been fetched yet.
git init -q -b main
git commit --allow-empty -qm initial
git remote add origin .
git config remote.origin.fetch '+refs/heads/*:refs/remotes/cache/*'
git branch renamed
git branch second
git branch unconfigured
git config branch.main.remote origin
git config branch.main.merge refs/heads/elsewhere
for name in renamed second missing-local
do
    git config "branch.$name.remote" origin
    git config "branch.$name.merge" refs/heads/topic
done
git for-each-ref --format='%(refname) %(upstream)' refs/heads/ >branch-upstreams
