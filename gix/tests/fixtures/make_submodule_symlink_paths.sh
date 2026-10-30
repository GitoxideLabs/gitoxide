#!/usr/bin/env bash
set -eu -o pipefail

# Configured submodule paths must not traverse symlinks, even without index gitlinks.
# Cover old-form and gitdir-file repositories, dangling links, and lexical aliases.
git init -q other
git init -q superproject
(cd superproject
  git init -q actual/old
  git init -q --separate-git-dir "$PWD/modern.git" actual/modern
  ln -s .. parent
  ln -s ../other leaf
  ln -s actual inside
  ln -s missing dangling
  ln -s / absolute
  mkdir deps
  ln -s ../../other deps/nested

  case_number=0
  for path in parent/other leaf inside/old inside/modern dangling dangling/child \
              absolute/missing deps/nested parent//other parent/./other leaf/ leaf/. inside/../actual/old
  do
    case_number=$((case_number + 1))
    git config -f .gitmodules "submodule.case-$case_number.path" "$path"
    git config -f .gitmodules "submodule.case-$case_number.url" ../other
  done
  git add .gitmodules parent leaf inside dangling absolute deps/nested
  git commit -qm "configured paths through symlinks"
)
git clone -q --bare superproject bare.git
git init -q --bare bare.git/modules/case-1

# A regular file obstructing the checkout must still allow the modules-directory fallback.
git init -q obstructed
(cd obstructed
  git init -q --bare .git/modules/module
  git config -f .git/modules/module/config core.bare false
  git config -f .git/modules/module/config core.worktree ../../../blocked/child
  git config -f .gitmodules submodule.module.path blocked/child
  git config -f .gitmodules submodule.module.url ../other
  touch blocked
)
