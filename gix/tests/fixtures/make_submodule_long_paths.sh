#!/usr/bin/env bash
set -eu -o pipefail

# Keep the real paths short while making the submodule's joined gitdir exceed
# Windows MAX_PATH. Every module/.. pair refers back to the superproject.
git init -q source
(cd source
  echo original >file
  git add file
  git commit -qm original
)

git init -q parent
(cd parent
  git submodule add -q ../source module
  git commit -qm submodule
  echo changed >>module/file
  {
    printf 'gitdir: ../'
    for _ in {1..40}; do
      printf 'module/../'
    done
    printf '.git/modules/module\n'
  } >module/.git

  # Git must still recognize the modified submodule with this spelling.
  git status --porcelain=v1 >../git-status
)
