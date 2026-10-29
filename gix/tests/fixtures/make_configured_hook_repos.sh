#!/usr/bin/env bash
set -eu -o pipefail

# Ordinary events run configured commands first and the traditional hook last.
git init -q repo
cat >repo/.git/hooks/post-checkout <<'HOOK'
#!/bin/sh
printf 'traditional\n'
HOOK
chmod +x repo/.git/hooks/post-checkout
cat >repo/configured-hook <<'HOOK'
#!/bin/sh
printf '%s\n' "$HOOK_VALUE" "$@"
cat
HOOK
chmod +x repo/configured-hook
printf 'hook input\n' >input

# A non-bare receive operation uses its Git directory both for execution and
# relative hooksPath lookup. A worktree hook makes the wrong context visible.
git init -q receiver
git -C receiver config core.hooksPath custom-hooks
mkdir -p receiver/.git/custom-hooks receiver/custom-hooks
cat >receiver/.git/custom-hooks/pre-receive <<'HOOK'
#!/bin/sh
test -f config && printf 'gitdir\n'
HOOK
cat >receiver/custom-hooks/pre-receive <<'HOOK'
#!/bin/sh
printf 'worktree\n'
HOOK
chmod +x receiver/.git/custom-hooks/pre-receive receiver/custom-hooks/pre-receive
git init -q sender
git -C sender commit -qm initial --allow-empty
git -C sender push -q ../receiver HEAD:refs/heads/incoming >receive-stdout 2>receive-output
