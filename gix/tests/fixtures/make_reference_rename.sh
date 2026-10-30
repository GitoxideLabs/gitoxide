#!/usr/bin/env bash
set -eu -o pipefail

# Each scenario caches one small repository, including Git's complete ref,
# reflog, HEAD, and configuration state. Writable tests only copy `setup`.
scenario=$1
operation=$2
source=topic
destination=renamed
force=false
invocation=main
case "$scenario" in
  same) destination=$source ;;
  descend) destination=topic/child ;;
  ascend) source=topic/child; destination=topic ;;
  non-utf8) source=$'topic-\xff'; destination=$'renamed-\xfe' ;;
esac

# APFS rejects invalid UTF-8 filenames even though Unix paths are byte-oriented.
if test "$scenario" = non-utf8; then
  if ! (: >"$source") 2>/dev/null; then
    printf 'The fixture filesystem rejects non-UTF-8 filenames.\n' >unsupported
    exit 0
  fi
  rm "$source"
fi

git init -q -b "$source" main
cd main
git commit -q --allow-empty -m initial
initial_oid=$(git rev-parse HEAD)
git commit -q --allow-empty -m second
source_oid=$(git rev-parse HEAD)
git branch other "$initial_oid"

add_linked() {
  git worktree add -q --force "../$1" "$2"
}

case "$scenario" in
  plain|same|descend|ascend|config|config-lock|tag|remote|bare|symbolic-head|non-utf8) ;;
  force-destination|reject-destination|destination-used-main|destination-used-linked|rebase-destination|bisect-destination|pending-destination|bare-destination)
    git branch "$destination" "$initial_oid"
    if test "$scenario" != reject-destination; then force=true; fi
    ;;
  symbolic-destination)
    git symbolic-ref -m destination "refs/heads/$destination" refs/heads/other
    ;;
  broken-destination)
    git symbolic-ref "refs/heads/$destination" refs/heads/missing
    ;;
  cyclic-destination)
    git symbolic-ref "refs/heads/$destination" "refs/heads/$destination"
    ;;
  malformed-target-destination)
    printf 'broken\n' >.git/BROKEN_REF
    git symbolic-ref "refs/heads/$destination" BROKEN_REF
    ;;
  worktrees|linked|head-lock|unborn-worktrees|unborn|unborn-existing|unborn-prefix|rebase-source|bisect-source|apply-source) ;;
  *) echo "unknown rename scenario: $scenario" >&2; exit 1 ;;
esac

case "$scenario" in
  worktrees|linked|head-lock)
    if test "$scenario" = head-lock; then git checkout -q other; fi
    add_linked one "$source"
    add_linked two "$source"
    if test "$scenario" = linked; then invocation=one; fi
    ;;
  destination-used-main) git checkout -q "$destination" ;;
  destination-used-linked) add_linked one "$destination" ;;
  bare) git config core.bare true ;;
  bare-destination)
    git symbolic-ref HEAD "refs/heads/$destination"
    git config core.bare true
    ;;
  symbolic-head)
    git symbolic-ref -m alias refs/heads/alias "refs/heads/$source"
    git symbolic-ref HEAD refs/heads/alias
    ;;
  unborn|unborn-existing|unborn-prefix|unborn-worktrees)
    source=unborn
    if test "$scenario" = unborn-existing; then
      git branch "$destination" "$initial_oid"
      force=true
    elif test "$scenario" = unborn-prefix; then
      destination=other/child
    fi
    git symbolic-ref HEAD "refs/heads/$source"
    if test "$scenario" = unborn-worktrees; then
      git worktree add -q --detach ../one "$source_oid"
      git -C ../one symbolic-ref HEAD "refs/heads/$source"
    fi
    ;;
  rebase-source|bisect-source|apply-source|rebase-destination|bisect-destination|pending-destination)
    git worktree add -q --detach ../one "$source_oid"
    reserved=$source
    case "$scenario" in *destination) reserved=$destination ;; esac
    # These are the branch reservation files left by interrupted operations.
    case "$scenario" in
      bisect-*)
        printf 'bisect started\n' >.git/worktrees/one/BISECT_LOG
        printf '%s\n' "$reserved" >.git/worktrees/one/BISECT_START
        ;;
      apply-source)
        mkdir .git/worktrees/one/rebase-apply
        touch .git/worktrees/one/rebase-apply/applying
        printf 'refs/heads/%s\n' "$reserved" >.git/worktrees/one/rebase-apply/head-name
        ;;
      *)
        mkdir .git/worktrees/one/rebase-merge
        printf 'refs/heads/%s\n' "$reserved" >.git/worktrees/one/rebase-merge/head-name
        if test "$scenario" = pending-destination; then
          printf 'refs/heads/unrelated\n' >.git/worktrees/one/rebase-merge/head-name
          printf 'refs/heads/%s\n%s\n%s\n' "$destination" "$initial_oid" "$initial_oid" >.git/worktrees/one/rebase-merge/update-refs
        fi
        ;;
    esac
    ;;
esac

# Duplicate source sections must be renamed in place, while destination
# sections, included files, per-worktree config, and upstream values survive.
cat >>.git/config <<EOF
[branch "$source"]
    # First source section.
    remote = origin
    marker = first
[branch "$destination"]
    marker = destination
    keep = yes
[branch "$source"]
    # Second source section.
    merge = refs/heads/upstream
    marker = last
[branch "other"]
    remote = .
    merge = refs/heads/$source
[include]
    path = included.cfg
[extensions]
    worktreeConfig = true
EOF
printf '[branch "%s"]\n\tincluded = unchanged\n' "$source" >.git/included.cfg
printf '[branch "%s"]\n\tprivate = unchanged\n' "$source" >.git/config.worktree
if test -d .git/worktrees; then
  for private in .git/worktrees/*; do
    printf '[branch "%s"]\n\tprivate = unchanged\n' "$source" >"$private/config.worktree"
  done
fi

source_ref="refs/heads/$source"
destination_ref="refs/heads/$destination"
case "$scenario" in
  tag)
    source_ref="refs/tags/$source"
    destination_ref="refs/tags/$destination"
    git tag -a -m annotated "$source" "$source_oid"
    ;;
  remote)
    source_ref="refs/remotes/origin/$source"
    destination_ref="refs/remotes/origin/$destination"
    git update-ref -m initial "$source_ref" "$initial_oid"
    git update-ref -m second "$source_ref" "$source_oid"
    ;;
  config-lock) touch .git/config.lock ;;
  head-lock) touch .git/worktrees/one/HEAD.lock ;;
esac
printf '%s\n' "$source_ref" "$destination_ref" "$force" "$invocation" >../input

status=0
case "$operation" in
  setup) ;;
  git)
    flag=-m
    if test "$force" = true; then flag=-M; fi
    git -C "../$invocation" branch "$flag" "$source" "$destination" >../stdout 2>../stderr || status=$?
    ;;
  *) echo "unknown rename operation: $operation" >&2; exit 1 ;;
esac
printf '%s\n' "$status" >../status

# Git generates absolute worktree links. Relative links make cached writable
# copies independent of their original fixture, without any Rust-side repair.
if test -d .git/worktrees; then
  for private in .git/worktrees/*; do
    name=${private##*/}
    printf '../../../../%s/.git\n' "$name" >"$private/gitdir"
    printf 'gitdir: ../main/.git/worktrees/%s\n' "$name" >"../$name/.git"
  done
fi
