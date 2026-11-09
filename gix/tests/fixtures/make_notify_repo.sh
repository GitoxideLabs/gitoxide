#!/usr/bin/env bash
set -eu -o pipefail

init_repo() {
  git init -q "$1"
  printf 'tracked\n' >"$1/this"
  git -C "$1" add this
  git -C "$1" commit -q -m 'notification fixture'
}

case "${1:-coverage}" in
  coverage)
    # Identical directories and index entries, but different ignore rules, make
    # registration counts prove that ignore-aware coverage skips the whole subtree.
    git init -q ignored
    git init -q unignored
    for repo in ignored unignored; do
      (
        cd "$repo"
        mkdir -p tracked ignored/deep
        echo tracked >tracked/file
        echo untracked >ignored/deep/file
        git add tracked/file
      )
    done
    printf 'ignored/\n' >ignored/.gitignore
    printf '# No ignored directories.\n' >unignored/.gitignore
    ;;
  malformed-index)
    # Recursive coverage must not read the index; ignore-aware coverage must.
    init_repo .
    printf 'not a Git index\n' >.git/index
    ;;
  index-symlink-loop)
    # Even resolving this index path fails. No mutation is needed to test that
    # unfiltered monitoring and raw streams leave the dependency alone.
    gix_testtools_require_symlinks
    init_repo .
    rm .git/index
    ln -s index .git/index
    ;;
  linked-in-common)
    # A common-directory ancestor must not classify the whole checkout as metadata.
    init_repo main
    git clone -q --bare --no-hardlinks main bare.git
    git -C bare.git worktree add -q --detach checkouts/root HEAD
    # Both registration directions stay inside writable copies of the fixture.
    printf 'gitdir: ../../worktrees/root\n' >bare.git/checkouts/root/.git
    printf '../../checkouts/root/.git\n' >bare.git/worktrees/root/gitdir
    ;;
  ignore-refresh)
    # Both subtrees exist before monitoring starts. Removing an ignore rule or
    # force-adding an ignored file must each expose the existing subtree recursively.
    init_repo .
    for directory in ignore-refresh/deep index-refresh/deep; do
      mkdir -p "$directory"
      printf 'initial\n' >"$directory/file"
    done
    printf 'ignore-refresh/\nindex-refresh/\n' >.gitignore
    ;;
  event-filter)
    # File-only exclusions leave directories watched. Callers receive ignored and
    # tracked file events alike, including forced entries absent at startup.
    init_repo .
    mkdir sub
    cat <<'EOF' >.gitignore
*.ignored
!keep.ignored
only-dir/
EOF
    cat <<'EOF' >sub/.gitignore
*.blocked
!keep.blocked
EOF
    for path in drop.ignored keep.ignored tracked.ignored missing.ignored only-dir sub/drop.blocked sub/keep.blocked; do
      printf 'initial\n' >"$path"
    done
    git add -f tracked.ignored missing.ignored
    rm missing.ignored
    ;;
  pattern-dependencies)
    # Optional configured files live below a missing administrative directory.
    # In-root XDG fallbacks initially hide an existing two-level subtree; creating
    # the configured files must switch sources and coverage without a rescan.
    init_repo .
    mkdir -p xdg/git ignored/deep
    printf 'initial\n' >ignored/deep/file
    printf 'ignored/\n' >xdg/git/ignore
    printf '* fallback\n' >xdg/git/attributes
    git config core.excludesFile ':(optional).git/patterns/nested/ignore'
    git config core.attributesFile ':(optional).git/patterns/nested/attributes'
    ;;
  raw-stream)
    # Raw coverage includes ignored files, nested repositories, and Git metadata.
    # Cookie directories are outside the worktree so only auxiliary watches cover them.
    init_repo main
    init_repo main/repo.git
    mkdir -p main/ignored/deep cookies replacement
    printf 'ignored\n' >main/ignored/deep/file
    printf 'ignored/\nthis\n' >main/.gitignore
    ;;
  *) echo "unsupported notification fixture: $1" >&2; exit 1 ;;
esac
