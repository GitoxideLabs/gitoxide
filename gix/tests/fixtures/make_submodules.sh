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

git clone with-submodules not-a-submodule
(cd not-a-submodule
  git submodule update --init
  cp .gitmodules modules.bak
  git rm m1
  echo fake > m1
  mv modules.bak .gitmodules
  git add m1 && git commit -m "no submodule in index and commit, but in configuration"
)

git init unborn
