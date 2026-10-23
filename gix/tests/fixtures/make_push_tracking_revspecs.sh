#!/usr/bin/env bash
set -eu -o pipefail

for setup in unconfigured triangular; do
  for mode in current matching upstream simple refspec; do
    git init -q -b main "$setup-$mode"
    (
      cd "$setup-$mode"
      git commit -q --allow-empty -m one
      git remote add origin ../origin.git
      git update-ref refs/remotes/origin/main HEAD
      git update-ref refs/remotes/origin/other HEAD
      git commit -q --allow-empty -m two
      if test "$setup" = triangular; then
        # Fetch and push tracking refs intentionally point to different commits.
        git remote add up ../up.git
        git update-ref refs/remotes/up/main HEAD
        git branch -q -u up/main
        git config remote.pushDefault origin
      fi
      if test "$mode" = refspec; then
        # An explicit push refspec takes precedence over push.default=upstream.
        git config push.default upstream
        git config remote.origin.push refs/heads/main:refs/heads/other
      else
        git config push.default "$mode"
      fi
      for branch in '' HEAD main; do
        spec="$branch@{push}"
        printf '%s\n' "$spec" >>baseline.git
        git rev-parse -q --verify "$spec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
      done
      git rev-parse --verify --symbolic-full-name '@{push}' >tracking-ref 2>/dev/null || :
    )
  done
done
