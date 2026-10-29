#!/usr/bin/env bash
set -eu -o pipefail

# Traditional hooks are shared by linked worktrees; configured relative paths
# instead start at the checkout from which the hook is run.
git init -q -b main repo
git -C repo commit -qm initial --allow-empty
git -C repo worktree add --detach ../linked HEAD
printf 'gitdir: ../repo/.git/worktrees/linked\n' >linked/.git
printf '../../../../linked/.git\n' >repo/.git/worktrees/linked/gitdir
git init -q --bare bare.git

for directory in repo/.git/hooks repo/custom-hooks linked/custom-hooks bare.git/hooks; do
  mkdir -p "$directory"
  cat >"$directory/pre-commit" <<'HOOK'
#!/bin/sh
printf '%s\n' "$1" "$2"
HOOK
  chmod +x "$directory/pre-commit"
done
cp repo/.git/hooks/pre-commit repo/pre-commit
chmod +x repo/pre-commit
cp repo/.git/hooks/pre-commit repo/.git/hooks/post-commit
chmod -x repo/.git/hooks/post-commit
mkdir repo/.git/hooks/directory

# Keep the reference command's output outside the inspected repositories.
git -C repo hook run pre-commit -- 'one argument' 'two' 2>expected-output
git -C linked hook run pre-commit -- 'one argument' 'two' 2>expected-linked-output
git -C bare.git hook run pre-commit -- 'one argument' 'two' 2>expected-bare-output

# Group execute permission does not let the owner execute a hook.
cp repo/.git/hooks/pre-commit repo/.git/hooks/applypatch-msg
chmod 610 repo/.git/hooks/applypatch-msg
if git -C repo hook run applypatch-msg -- one two >mode-output 2>mode-error; then
  touch mode-hook-executable
fi

# A prepared command must not inherit repository paths from a different handle.
for directory in repo/.git/hooks bare.git/hooks; do
  cat >"$directory/pre-auto-gc" <<'HOOK'
#!/bin/sh
printf '%s\n' "$GIT_COMMON_DIR" "${GIT_WORK_TREE-unset}"
HOOK
  chmod +x "$directory/pre-auto-gc"
done
