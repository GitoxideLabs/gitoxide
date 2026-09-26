#!/usr/bin/env bash
set -eu -o pipefail

# Prerequisites for worktree-add tests. Failure injection, platform-specific
# configuration, and comparisons with Git remain in Rust.
case "$1" in
  config-inheritance)
    # Preserve the basic history and the this file used for line-ending checks.
    # Rust installs the platform-specific filters after these attributes are committed.
    source "$(dirname "${BASH_SOURCE[0]}")/make_basic_repo.sh"
    printf "filtered filter=inherit\n" >.gitattributes
    printf "hello\n" >filtered
    git add .gitattributes filtered
    git commit -m "record filter attributes"
    ;;
  relative-config-lock)
    # Honor the fixture runner-selected hash; Rust holds the config lock during addition.
    git init
    git commit --allow-empty -m initial
    ;;
  relative-config-unknown-extension)
    # Only SHA-1 starts at format v0. The unknown extension must prevent upgrading
    # to v1 when enabling relative worktree links, even in SHA-256 fixture runs.
    git init --object-format=sha1
    git commit --allow-empty -m initial
    git config extensions.futureExtension true
    ;;
  separate-git-dir)
    # Use Creation::Execute: main/.git contains an absolute path to separate.git.
    git init --initial-branch=main --separate-git-dir separate.git main
    git -C main commit --allow-empty -m initial
    ;;
  *)
    echo "unknown worktree-add fixture scenario: $1" >&2
    exit 1
    ;;
esac
