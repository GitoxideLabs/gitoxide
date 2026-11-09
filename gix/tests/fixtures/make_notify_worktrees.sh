#!/usr/bin/env bash
set -eu -o pipefail

# Sibling checkouts deliberately lie outside each other's watched roots. Every
# checkout has the same tracked path, so routing must use its own relative scope.
ref_format=${1:-files}
case "$ref_format" in
  files) git init -q main ;;
  reftable) git init -q --ref-format=reftable main ;;
  *) echo "unsupported reference format: $ref_format" >&2; exit 1 ;;
esac
(
  cd main
  git symbolic-ref HEAD refs/heads/main
  mkdir tracked
  printf 'tracked\n' >tracked/file
  printf 'ignored/\n' >.gitignore
  git add .gitignore tracked/file
  git commit -q -m 'notification worktree fixture'
)
git -C main worktree add -q --detach ../linked-a HEAD
git -C main worktree add -q --detach ../linked-b HEAD

# Keep both directions of each registration relative so relocating this fixture
# cannot leave links into the original directory.
# commondir is already relative. These paths also work without --relative-paths
# support in the Git version generating the fixture.
printf 'gitdir: ../main/.git/worktrees/linked-a\n' >linked-a/.git
printf 'gitdir: ../main/.git/worktrees/linked-b\n' >linked-b/.git
printf '../../../../linked-a/.git\n' >main/.git/worktrees/linked-a/gitdir
printf '../../../../linked-b/.git\n' >main/.git/worktrees/linked-b/gitdir

# A bare main has no checkout of its own, but its linked checkouts still need
# independent worktree coverage and startup baselines.
# Cloning does not inherit the source's reference backend; request it explicitly.
case "$ref_format" in
  files) git clone -q --bare --no-hardlinks main bare.git ;;
  reftable) git clone -q --bare --no-hardlinks --ref-format=reftable main bare.git ;;
esac
git -C bare.git worktree add -q --detach ../bare-a HEAD
git -C bare.git worktree add -q --detach ../bare-b HEAD
printf 'gitdir: ../bare.git/worktrees/bare-a\n' >bare-a/.git
printf 'gitdir: ../bare.git/worktrees/bare-b\n' >bare-b/.git
printf '../../../bare-a/.git\n' >bare.git/worktrees/bare-a/gitdir
printf '../../../bare-b/.git\n' >bare.git/worktrees/bare-b/gitdir

# Two ignored directory levels make inventory growth observable (+2 watches)
# when a single checkout is unignored, without timing the absence of events.
mkdir -p {main,linked-a,linked-b,bare-a,bare-b}/ignored/deep
printf 'ignored\n' | tee {main,linked-a,linked-b,bare-a,bare-b}/ignored/deep/file >/dev/null

# Private-HEAD notification tests switch each checkout to this existing branch;
# creating it is setup, not one of the reference events under test.
if [ "$ref_format" = reftable ]; then
  git -C main branch notify-head HEAD
fi
