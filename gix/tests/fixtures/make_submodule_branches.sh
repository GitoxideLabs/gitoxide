#!/usr/bin/env bash
set -eu -o pipefail

# Give the remote HEAD, the inherited branch, and the explicit branch different tips.
# Keep the submodule itself on main before Git resolves the update target.
git init -q -b main source
git -C source commit -q --allow-empty -m main
git -C source checkout -q -b topic/feature
git -C source commit -q --allow-empty -m feature
git -C source branch topic/unborn
git -C source checkout -q -b explicit
git -C source commit -q --allow-empty -m explicit
git -C source checkout -q main

git init -q -b topic/feature inherited
git -C inherited submodule add -q ../source sm
git -C inherited config -f .gitmodules submodule.sm.branch .
git -C inherited add .gitmodules
git -C inherited commit -q -m submodule

cp -R inherited override-dot
git -C override-dot config -f .gitmodules submodule.sm.branch explicit
git -C override-dot config submodule.sm.branch .

cp -R inherited unborn
git -C unborn symbolic-ref HEAD refs/heads/topic/unborn

cp -R inherited detached
git -C detached checkout -q --detach
cp -R detached override-name
git -C override-name config submodule.sm.branch explicit
cp -R detached unset
git -C unset config -f .gitmodules --unset submodule.sm.branch

# The resulting child HEAD is the Git baseline. Detached superprojects only
# fail when the effective configuration asks to inherit the branch.
for scenario in inherited override-dot unborn override-name unset; do
  git -C "$scenario" submodule update --remote --no-fetch
done
if git -C detached submodule update --remote --no-fetch; then
  echo 'Git unexpectedly inherited a branch from a detached superproject' >&2
  exit 1
fi
git -C detached submodule update --no-fetch
