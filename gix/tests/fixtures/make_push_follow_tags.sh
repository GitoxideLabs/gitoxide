#!/usr/bin/env bash
set -eu -o pipefail

# Follow-tags considers proposed tips and unchanged advertised tips, including
# the old tip of a deleted ref. Only missing annotated tags of commits qualify.
git init -q --object-format="$1" -b main source
git -C source commit --allow-empty -qm root
root=$(git -C source rev-parse HEAD)
git -C source tag -a root-tag -m root
git -C source tag -a existing -m old
git -C source commit --allow-empty -qm main
git -C source tag -a main-tag -m main
git -C source tag -a nested-tag main-tag -m nested
git -C source tag lightweight
blob=$(printf 'blob' | git -C source hash-object -w --stdin)
git -C source tag -a blob-tag "$blob" -m blob
git -C source checkout -qb side "$root"
git -C source commit --allow-empty -qm side
git -C source tag -a side-tag -m side
git -C source checkout -q --orphan remove
git -C source commit --allow-empty -qm remove
git -C source tag -a removed-tip-tag -m remove
git -C source checkout -q --orphan unreachable
git -C source commit --allow-empty -qm unreachable
git -C source tag -a unreachable-tag -m unreachable
git -C source checkout -q main

git init -q --bare --object-format="$1" destination
git -C source push -q ../destination "$root:refs/heads/main" refs/heads/side refs/heads/remove refs/tags/existing
git -C source tag -fa existing -m new
git -C source config push.followTags true
for name in expected no-follow dry-run connection connection-follow explicit-follow
do
    cp -R destination "$name"
done
git -C source push -q ../expected refs/heads/main:refs/heads/main :refs/heads/remove
git -C expected for-each-ref --format='%(refname) %(objectname)' >expected-refs
