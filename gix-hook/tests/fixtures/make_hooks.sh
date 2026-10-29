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
