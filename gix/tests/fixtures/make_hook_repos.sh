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

# Git retries executable scripts without a shebang on Unix, but not on Windows.
# Shell metacharacters in a hook path or argument must stay literal.
mkdir -p 'repo/hook $@ space'
cat >'repo/hook $@ space/pre-rebase' <<'HOOK'
printf '%s\n' "$HOOK_VALUE" "$@"
printf 'hook-stderr\n' >&2
cat
HOOK
chmod +x 'repo/hook $@ space/pre-rebase'
if HOOK_VALUE='literal $value' git -C repo -c 'core.hooksPath=hook $@ space' hook run pre-rebase -- 'one argument' '$(touch unexpected)' >no-shebang-stdout 2>no-shebang-output; then
  touch no-shebang-supported
fi

# Produce more than pipe capacity before reading stdin, so a synchronous writer deadlocks.
cat >repo/.git/hooks/pre-merge-commit <<'HOOK'
#!/bin/sh
head -c 1048576 /dev/zero
head -c 1048576 /dev/zero >&2
cat
HOOK
chmod +x repo/.git/hooks/pre-merge-commit

# The hook's exit status remains authoritative if it closes stdin early.
cat >repo/.git/hooks/pre-applypatch <<'HOOK'
#!/bin/sh
exec 0<&-
printf 'early stdout\n'
printf 'early stderr\n' >&2
exit 23
HOOK
chmod +x repo/.git/hooks/pre-applypatch

# A missing interpreter is ENOENT, and must not trigger shell fallback.
cat >repo/.git/hooks/pre-receive <<'HOOK'
#!/gitoxide-missing-hook-interpreter
printf 'must not run\n'
HOOK
chmod +x repo/.git/hooks/pre-receive
