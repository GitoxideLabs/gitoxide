#!/usr/bin/env bash
set -eu -o pipefail

git init -q module1
(cd module1
  touch this
  mkdir subdir
  touch subdir/that
  git add .
  git commit -q -m c1
  echo hello >> this
  git commit -q -am c2
  touch untracked
)

git init submodule-head-changed
(cd submodule-head-changed
  git submodule add ../module1 m1
  git commit -m "add submodule"

  cd m1 && git checkout @~1
)

git init submodule-index-changed
(cd submodule-index-changed
  git submodule add ../module1 m1
  git commit -m "add submodule"

  (cd m1
    git mv subdir  subdir-renamed
    git mv this that
  )
)

git init submodule-head-changed-no-worktree
(cd submodule-head-changed-no-worktree
  git submodule add ../module1 m1
  git commit -m "add submodule"

  (cd m1 && git checkout @~1)
  rm -Rf m1 && mkdir m1
)

git init modified-and-untracked
(cd modified-and-untracked
  git submodule add ../module1 m1
  git commit -m "add submodule"

  (cd m1
    echo change >> this
    touch new
  )
)

git init submodule-head-changed-and-modified
(cd submodule-head-changed-and-modified
  git submodule add ../module1 m1
  git commit -m "add submodule"

  (cd m1
    git checkout @~1
    echo change >> this
  )
)

git init submodule-head-changed-and-modified-with-untracked-in-nested-dir
(cd submodule-head-changed-and-modified-with-untracked-in-nested-dir
  git submodule add ../module1 m1
  git commit -m "add submodule"

  mkdir -p a/b
  touch a/b/untracked

  (cd m1
    git checkout @~1
    echo change >> this
  )
)

ln -s submodule-head-changed-and-modified-with-untracked-in-nested-dir/a/b link-to-dir-in-changed-parent-repo

git init modified-untracked-and-submodule-head-changed-and-modified
(cd modified-untracked-and-submodule-head-changed-and-modified
  git submodule add ../module1 m1
  git commit -m "add submodule"

  (cd m1
    git checkout @~1
    echo change >> this
  )

  touch this
  git add this && git commit -m "this"
  echo change >> this
  touch untracked
)

cp -Rv modified-untracked-and-submodule-head-changed-and-modified git-mv-and-untracked-and-submodule-head-changed-and-modified
(cd git-mv-and-untracked-and-submodule-head-changed-and-modified
  git checkout this
  git mv this that
)

cp -Rv git-mv-and-untracked-and-submodule-head-changed-and-modified git-mv-and-untracked-and-submodule-head-changed-and-modified-ignore-all
(cd git-mv-and-untracked-and-submodule-head-changed-and-modified-ignore-all
  echo $'\tignore = all' >>.gitmodules
  git add .gitmodules && git commit -m "ignore all submodule changes"
)

git init with-submodules
(cd with-submodules
  mkdir dir
  touch dir/file
  git add dir
  git commit -m "init"

  git submodule add ../module1 m1
  git commit -m "add module 1"

  git submodule add ../module1 dir/m1
)

# Git permits absolute-looking submodule names because it appends them textually below
# `.git/modules/` instead of joining them as paths. Keep committed gitlinks for both Unix and
# Windows spellings so consumers can verify that these are ordinary submodules made by Git.
git init absolute-looking-submodule-names
(cd absolute-looking-submodule-names
  # Use portable names while cloning, then change only the committed configuration. Passing a
  # leading slash through Git Bash on Windows would be translated to its installation directory.
  git submodule add ../module1 unix
  git submodule add ../module1 windows
  git config -f .gitmodules --rename-section submodule.unix submodule./absolute/unix
  git config -f .gitmodules --rename-section submodule.windows 'submodule.\absolute\windows'
  git commit -m "add submodules with absolute-looking names"

  # A backslash is a filename character on Unix but a separator on Windows. The test only needs
  # Git's committed configuration and gitlinks, so omit the cloned repositories from the fixture
  # to keep its generated representation portable.
  rm -rf .git/modules
)

git init submodule-with-divergent-gitlink
(cd submodule-with-divergent-gitlink
  git submodule add ../module1 outer/inner
  git commit -m "add nested submodule"

  mv .git/modules/outer/inner .git/modules/inner
  rmdir .git/modules/outer
  git config --file .git/modules/inner/config core.worktree ../../../outer/inner
  printf 'gitdir: ../../.git/modules/inner\n' >outer/inner/.git

  git clone --bare ../module1 .git/modules/outer/inner
  git -C .git/modules/outer/inner update-ref HEAD "$(git -C ../module1 rev-parse @~1)"
)

mkdir linked-git-dir-detached-worktree
(cd linked-git-dir-detached-worktree
  mkdir -p home store
  git init store/dots
  ln -s ../store/dots/.git home/.git
  git -C store/dots config core.worktree ../../../home
  git -C home submodule add ../../module1 .config/awesome/lain
  git -C home commit -m "add detached-worktree submodule"
  echo "Git resolves the symlinked top-level .git to the real git dir before applying relative core.worktree." \
    >baseline.note
  git -C home status --porcelain=v1 >status.baseline
  git -C home/.config/awesome/lain rev-parse --show-toplevel >submodule-worktree.baseline
)

git init submodule-with-missing-gitlink-target
(cd submodule-with-missing-gitlink-target
  git submodule add ../module1 m1
  git commit -m "add submodule"
  printf 'gitdir: ../missing\n' >m1/.git
)

git init submodule-with-malformed-gitlink
(cd submodule-with-malformed-gitlink
  git submodule add ../module1 m1
  git commit -m "add submodule"
  printf 'bogus\n' >m1/.git
)

cp -R with-submodules with-submodules-in-index
(cd with-submodules-in-index
  git add .
  rm .gitmodules
)

cp -R with-submodules with-submodules-in-tree
(cd with-submodules-in-tree
  rm .gitmodules
  rm .git/index
)

git init old-form-invalid-worktree-path
(cd old-form-invalid-worktree-path
  mkdir dir
  git submodule add --name old ../module1 dir/old-form
  rm dir/old-form/.git
  # the config file contains a worktree path that points to the wrong place now
  mv .git/modules/old dir/old-form/.git
)

cp -R old-form-invalid-worktree-path old-form
(cd old-form
  cd dir/old-form
  git config --unset core.worktree
)

git clone with-submodules with-submodules-after-clone
(cd with-submodules-after-clone
  git submodule init m1
)
git clone --bare with-submodules with-submodules-after-clone.git
(cd with-submodules-after-clone.git
  # manually create a clone and see if we can handle it despite being bare
  git clone --bare ../module1 modules/m1
)

git init with-submodule-uninitialized-checkout
(cd with-submodule-uninitialized-checkout
  git submodule add ../module1 m1
  git submodule add ../module1 m2
  git commit -m "add modules"
  # The module clones in `.git/modules` remain, but the checkouts don't exist yet,
  # like just before a fresh worktree checkout.
  rm m1/.git
  rm -rf m2
)

git clone with-submodules not-a-submodule
(cd not-a-submodule
  git submodule update --init
  cp .gitmodules modules.bak
  git rm m1
  echo fake > m1
  mv modules.bak .gitmodules
  git add m1 && git commit -m "no submodule in index and commit, but in configuration"
)

# Path discovery must skip an absent field, but reject a present invalid one.
# The unrelated section comes first so a later valid path cannot hide its error.
for case in missing implicit empty absolute escaping; do
  root="path-$case"
  git init "$root"
  git -C "$root" read-tree --empty
  case "$case" in
    missing) field='url = ./unused' ;;
    implicit) field='path' ;;
    empty) field='path =' ;;
    absolute) field='path = /outside' ;;
    escaping) field='path = ../outside' ;;
  esac
  printf '[submodule "unrelated"]\n    %s\n[submodule "module"]\n    path = module\n' "$field" >"$root/.gitmodules"
done

git init unborn

# Opening repositories through this symlink exercises preservation of the
# caller's path namespace when only an ancestor of the Git directory is linked.
ln -s . symlinked-ancestor
