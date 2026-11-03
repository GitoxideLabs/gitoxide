#!/usr/bin/env bash
set -eu -o pipefail

# Prime Git's FSMN bitmap, then change both reported and deliberately unreported
# files. The latter must be skipped, proving that status actually uses the monitor.
git init -q
mkdir dir
for path in reported unreported dirty dir/a dir/b directory; do
  echo original >"$path"
done
git add .
git commit -qm initial
cat >.git/fsmonitor-hook <<'EOF'
#!/bin/sh
case "$1" in
  1) cat .git/fsmonitor-paths ;;
  2) printf 'next-token\000'; cat .git/fsmonitor-paths ;;
  *) exit 1 ;;
esac
EOF
cat >.git/fsmonitor-v1 <<'EOF'
#!/bin/sh
test "$1" = 1 || exit 1
cat .git/fsmonitor-paths
EOF
cat >.git/fsmonitor-fail <<'EOF'
#!/bin/sh
exit 1
EOF
cat >.git/fsmonitor-malformed <<'EOF'
#!/bin/sh
printf '\000reported\000'
EOF
chmod +x .git/fsmonitor-*
git config core.fsmonitor .git/fsmonitor-hook
git config core.untrackedCache false
: >.git/fsmonitor-paths
echo 'already dirty before the monitor was primed' >dirty
git status --porcelain >.git/initial-status
for path in reported unreported dir/a directory; do
  echo 'changed after the monitor was primed' >"$path"
done
echo untracked >untracked
printf 'reported\000dir\000' >.git/fsmonitor-paths
# Keep the original bitmap: asking Git for a baseline must not refresh it.
GIT_OPTIONAL_LOCKS=0 git status --porcelain >.git/expected-status
GIT_OPTIONAL_LOCKS=0 git -c core.fsmonitor=false status --porcelain >.git/expected-full-status
