#!/usr/bin/env bash
set -eu -o pipefail

for setup in unconfigured triangular mirror; do
  modes=(current matching upstream simple refspec)
  if test "$setup" = mirror; then
    modes=(upstream nothing refspec no-fetch)
  fi
  for mode in "${modes[@]}"; do
    git init -q -b main "$setup-$mode"
    (
      cd "$setup-$mode"
      git commit -q --allow-empty -m one
      git remote add origin ../origin.git
      git update-ref refs/remotes/origin/main HEAD
      git update-ref refs/remotes/origin/other HEAD
      git commit -q --allow-empty -m two
      push_remote=origin
      if test "$setup" = mirror; then
        # The branch's mirror push remote overrides the global default and tracks a different commit.
        git branch -q -u origin/main
        git remote add mirror ../mirror.git
        git update-ref refs/remotes/mirror/main HEAD
        git update-ref refs/remotes/mirror/other refs/remotes/origin/main
        git config remote.pushDefault origin
        git config branch.main.pushRemote mirror
        git config remote.mirror.mirror true
        push_remote=mirror
      elif test "$setup" = triangular; then
        # Fetch and push tracking refs intentionally point to different commits.
        git remote add up ../up.git
        git update-ref refs/remotes/up/main HEAD
        git branch -q -u up/main
        git config remote.pushDefault origin
      fi
      case "$mode" in
        refspec)
          # An explicit push refspec takes precedence over mirror mode and push.default=upstream.
          git config push.default upstream
          git config "remote.$push_remote.push" refs/heads/main:refs/heads/other
          ;;
        nothing)
          git config --unset branch.main.merge
          git config push.default nothing
          ;;
        no-fetch)
          git config push.default upstream
          git config remote.mirror.fetch +refs/heads/other:refs/remotes/mirror/other
          ;;
        *)
          git config push.default "$mode"
          ;;
      esac
      for branch in '' HEAD main; do
        spec="$branch@{push}"
        printf '%s\n' "$spec" >>baseline.git
        git rev-parse -q --verify "$spec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
      done
      git rev-parse --verify --symbolic-full-name '@{push}' >tracking-ref 2>/dev/null || :
    )
  done
done
