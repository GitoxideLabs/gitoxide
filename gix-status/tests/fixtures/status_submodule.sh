#!/usr/bin/env bash
set -eu -o pipefail

git init -q module1
(cd module1
  touch this
  git add .
  git commit -q -m c1
  echo hello >> this
  git commit -q -am c2
)

git init no-change
(cd no-change
  git submodule add ../module1 m1
  git commit -m "add module 1"
)

cp -R no-change deleted-dir
(cd deleted-dir
  rm -Rf m1
)

cp -R no-change type-change
(cd type-change
  rm -Rf m1
  touch m1
)

# Gitlinks without a .gitmodules entry still track their paths after a type change.
for kind in file symlink
do
  git init -q "type-change-unconfigured-$kind"
  (cd "type-change-unconfigured-$kind"
    git commit --allow-empty -m init
    git update-index --add --cacheinfo "160000,$(git rev-parse HEAD),m1"
    git commit -m gitlink
    if test "$kind" = file
    then
      echo content > m1
    elif ! { ln -s ../module1 m1 2>/dev/null && test -L m1; }; then
      # MSYS may emulate symlinks, or the host may lack permission to create them.
      rm -rf m1
      touch .git/symlinks-unavailable
    fi
  )
done

for repo in type-change type-change-unconfigured-file type-change-unconfigured-symlink
do
  (cd "$repo"
    # Git status rejects symlinked gitlinks, but ls-files still excludes them from untracked paths.
    if test "$repo" != type-change-unconfigured-symlink
    then
      git status --porcelain=v1 > .git/status.baseline
    fi
    git ls-files --others > .git/untracked.baseline
  )
done

cp -R no-change empty-dir-no-change
(cd empty-dir-no-change
  rm -Rf m1
  mkdir m1
)

cp -R no-change conflict
(cd conflict
  (cd m1
    git checkout @~1
  )

  git commit -am "change submodule head"
  git checkout -b other @~1
  git rm -rf m1
  git commit -m "removed submodule"

  git merge main || :
)
