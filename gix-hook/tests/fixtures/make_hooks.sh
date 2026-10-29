#!/usr/bin/env bash
set -eu -o pipefail

# A single repository gives Git an oracle hook directory. Rust tests consume the
# hook files directly, with explicit command context and no repository handle.
git init -q -b main repo
mkdir -p 'repo/hook $@ space' repo/hook-interpreter repo/.git/hooks/directory
cat >repo/.git/hooks/pre-commit <<'HOOK'
#!/bin/sh
printf '%s\n' "$HOOK_VALUE" "$@"
printf 'hook-stderr\n' >&2
cat
HOOK
chmod +x repo/.git/hooks/pre-commit
cp repo/.git/hooks/pre-commit repo/.git/hooks/post-commit
chmod -x repo/.git/hooks/post-commit
cp repo/.git/hooks/pre-commit repo/.git/hooks/applypatch-msg
chmod 610 repo/.git/hooks/applypatch-msg
if git -C repo hook run applypatch-msg >mode-output 2>mode-error; then
  touch mode-hook-executable
fi

# The OS, not a heuristic, decides whether Unix shell fallback is needed.
sed '1d' repo/.git/hooks/pre-commit >'repo/hook $@ space/pre-rebase'
chmod +x 'repo/hook $@ space/pre-rebase'
if HOOK_VALUE='literal $value' git -C repo -c 'core.hooksPath=hook $@ space' hook run pre-rebase -- 'one argument' '$(touch unexpected)' >no-shebang-stdout 2>no-shebang-output; then
  touch no-shebang-supported
fi
cat >repo/.git/hooks/pre-receive <<'HOOK'
#!/gitoxide-missing-hook-interpreter
printf 'must not run\n'
HOOK
chmod +x repo/.git/hooks/pre-receive

# More than pipe capacity is written before input is read.
cat >repo/.git/hooks/pre-merge-commit <<'HOOK'
#!/bin/sh
head -c 1048576 /dev/zero
head -c 1048576 /dev/zero >&2
cat
HOOK
chmod +x repo/.git/hooks/pre-merge-commit
cat >repo/.git/hooks/pre-applypatch <<'HOOK'
#!/bin/sh
exec 0<&-
printf 'early stdout\n'
printf 'early stderr\n' >&2
exit 23
HOOK
chmod +x repo/.git/hooks/pre-applypatch
cat >repo/.git/hooks/post-checkout <<'HOOK'
#!/bin/sh
printf 'traditional\n'
HOOK
chmod +x repo/.git/hooks/post-checkout
cat >repo/.git/hooks/pre-auto-gc <<'HOOK'
#!/bin/sh
printf '%s\n' "$GIT_DIR" "$GIT_COMMON_DIR" "${GIT_WORK_TREE-unset}"
HOOK
chmod +x repo/.git/hooks/pre-auto-gc

# Relative interpreter PATH lookup can inspect a placeholder without running it.
printf 'interpreter placeholder\n' >repo/hook-interpreter/sh.exe
printf 'hook input\n' >input
