#!/usr/bin/env bash
set -eu -o pipefail

# Main has two commits of its own, and its upstream topic has one.
git init -q unconfigured
(
  cd unconfigured
  git commit -q --allow-empty -m root
  git branch topic
  git commit -q --allow-empty -m ahead-one
  git commit -q --allow-empty -m ahead-two
  git switch -q topic
  git commit -q --allow-empty -m behind
  git switch -q main
  git update-ref refs/tracking/topic topic
  git update-ref refs/tracking/fallback main
)

for repo in local mapped local-mapped; do
  cp -R unconfigured "$repo"
done

git -C local config branch.main.remote .
git -C local config --add branch.main.merge refs/heads/topic
git -C local config --add branch.main.merge refs/heads/main
git -C local-mapped config branch.main.remote .
git -C local-mapped config branch.main.merge refs/heads/topic
git -C local-mapped config remote...fetch refs/heads/topic:refs/tracking/fallback

# Upstream lookup uses the first merge entry and the first matching fetch
# refspec. A literal object in a fetch spec cannot supply an unrelated upstream.
git -C mapped config branch.main.remote origin
git -C mapped config --add branch.main.merge refs/heads/topic
git -C mapped config --add branch.main.merge refs/heads/main
git -C mapped config --add remote.origin.fetch "$(git -C mapped rev-parse HEAD):refs/tracking/fallback"
git -C mapped config --add remote.origin.fetch '+refs/heads/*:refs/tracking/*'
git -C mapped config --add remote.origin.fetch '+refs/heads/topic:refs/tracking/fallback'
# This malformed push URL is irrelevant to resolving a local tracking ref.
git -C mapped config remote.origin.pushurl ssh://host

cp -R mapped excluded
git -C excluded config --add remote.origin.fetch '^refs/heads/topic'
cp -R mapped gone
git -C gone update-ref -d refs/tracking/topic

for repo in local mapped local-mapped; do
  git -C "$repo" rev-parse --symbolic-full-name --verify '@{upstream}' >"$repo/.git/upstream-baseline"
done
