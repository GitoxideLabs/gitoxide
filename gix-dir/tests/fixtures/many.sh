#!/usr/bin/env bash
set -eu -o pipefail

# These fixtures don't use symlinks. See `many-symlinks.sh` for some that do.

git init with-nested-dot-git
(cd with-nested-dot-git
  mkdir -p dir/.git/subdir
  touch dir/.git/config dir/.git/subdir/bar
)

git init with-nested-capitalized-dot-git
(cd with-nested-capitalized-dot-git
  mkdir -p dir/.GIT/subdir
  touch dir/.GIT/config dir/.GIT/subdir/bar
)

git init dir-with-file
(cd dir-with-file
  mkdir dir
  touch dir/file
)

git init dir-with-tracked-file
(cd dir-with-tracked-file
  mkdir dir
  touch dir/file
  git add .
  git commit -m "init"
)

git init repo-with-submodule
(cd repo-with-submodule
  git submodule add ../dir-with-tracked-file submodule
  git commit -m "add submodule"
  touch submodule/untracked
)

git init ignored-dir
(cd ignored-dir
  mkdir dir
  touch dir/file
  echo "dir/" > .gitignore
)

cp -R ignored-dir ignored-dir-with-nested-repository
(cd ignored-dir-with-nested-repository
  echo "*.o" >> .gitignore
  git add .
  mkdir dir/subdir objs
  (cd dir/subdir
    touch a
    git init nested
  )
  >objs/a.o
)

cp -R ignored-dir ignored-dir-with-nested-bare-repository
(cd ignored-dir-with-nested-bare-repository
  mkdir dir/subdir
  (cd dir/subdir
    git init --bare nested-bare
  )
  git init --bare bare
)

cp -R ignored-dir-with-nested-bare-repository ignored-dir-nested-minimal
(cd ignored-dir-nested-minimal
  (cd bare
    rm -Rf hooks config description
  )
  (cd dir/subdir/nested-bare
    rm -Rf refs hooks config description
  )
)

mkdir untracked-hidden-bare
(cd untracked-hidden-bare
  mkdir subdir
  git init --bare subdir/hidden-bare
  >subdir/file
)

git init tracked-is-ignored
(cd tracked-is-ignored
  mkdir dir
  touch dir/file
  echo "dir/" > .gitignore
  git add --force . && git commit -m "init"
)

git init nested-repository
(cd nested-repository
  touch file
  git add . && git commit -m "init"

  git init nested
  (cd nested
    touch file
    git add file && git commit -m "init"
  )
)

git clone dir-with-tracked-file with-submodule
(cd with-submodule
  git submodule add ../dir-with-tracked-file submodule
  git commit -m "submodule added"
)

git init nonstandard-worktree
(cd nonstandard-worktree
  mkdir dir-with-dot-git
  touch dir-with-dot-git/inside
  touch seemingly-outside
  git add dir-with-dot-git/inside seemingly-outside
  mv .git dir-with-dot-git
  git -C dir-with-dot-git config core.worktree "$PWD"
  git -C dir-with-dot-git commit -m "init"
)

git init nonstandard-worktree-untracked
(cd nonstandard-worktree-untracked
  mkdir dir-with-dot-git
  touch dir-with-dot-git/inside
  touch seemingly-outside
  git add dir-with-dot-git/inside seemingly-outside
  mv .git dir-with-dot-git
  git -C dir-with-dot-git config core.worktree "$PWD"
  git -C dir-with-dot-git commit -m "init"

  rm dir-with-dot-git/.git/index
)

git init partial-checkout-cone-mode
(cd partial-checkout-cone-mode
  touch a b
  mkdir c1
  (cd c1 && touch a b && mkdir c2 && cd c2 && touch a b)
  (cd c1 && mkdir c3 && cd c3 && touch a b)
  mkdir d
  (cd d && touch a b && mkdir c4 && cd c4 && touch a b c5)

  git add .
  git commit -m "init"

  git sparse-checkout set c1/c2 --sparse-index

  mkdir d && touch d/file-created-manually
)

git init partial-checkout-non-cone
(cd partial-checkout-non-cone
  touch a b
  mkdir c1
  (cd c1 && touch a b && mkdir c2 && cd c2 && touch a b)
  (cd c1 && mkdir c3 && cd c3 && touch a b)
  mkdir d
  (cd d && touch a b && mkdir c4 && cd c4 && touch a b c5)

  git add .
  git commit -m "init"

  git sparse-checkout set c1/c2 --no-cone
  mkdir d && touch d/file-created-manually
)

git init precious-nested-repository
(cd precious-nested-repository
  echo '$precious*/' > .gitignore
  git init precious-repo
  git add .gitignore && git commit -m "init"
)

git init only-untracked
(cd only-untracked
  >a
  >b
  mkdir d
  >d/a
  >d/b
  mkdir d/d
  >d/d/a
  >c
)

git init untracked-cache-changed-parent-ignore
(cd untracked-cache-changed-parent-ignore
  mkdir child
  echo ignored >.gitignore
  >child/ignored
  git add .gitignore
  git commit -m "track inherited ignore"
)

git init ignored-with-empty
(cd ignored-with-empty
  echo "/target/" >> .gitignore
  git add .gitignore && git commit -m "init"
  mkdir -p target/empty target/debug target/release
  touch target/debug/a target/release/b
)

git init ignored-with-only-empty-dirs
(cd ignored-with-only-empty-dirs
  echo "/target/" >> .gitignore
  git add .gitignore && git commit -m "init"
  mkdir -p target/empty target/nested/empty
)

cp -R only-untracked subdir-untracked
(cd subdir-untracked
  git add .
  git rm --cached d/d/a
  git commit -m "init"
)

cp -R subdir-untracked subdir-untracked-and-ignored
(cd subdir-untracked-and-ignored
  >a.o
  >b.o
  >d/a.o
  >d/b.o
  >d/d/a.o
  >d/d/b.o
  >c.o
  mkdir generated d/generated d/d/generated
  touch generated/a generated/a.o d/generated/b d/d/generated/b
  mkdir -p objs/sub
  touch objs/a.o objs/b.o objs/sub/other.o

  echo "*.o" > .gitignore
  echo "generated/" >> .gitignore
)

mkdir untracked-and-ignored-for-collapse
(cd untracked-and-ignored-for-collapse
  echo "ignored/" >> .gitignore
  echo "*.o" >> .gitignore

  mkdir -p untracked ignored/empty mixed ignored-inside
  touch untracked/a ignored/b mixed/c mixed/c.o ignored-inside/d.o
)

git init untracked-and-precious
(cd untracked-and-precious
  echo '*.o' >> .gitignore
  echo '$*.precious' >> .gitignore

  mkdir -p d/d
  touch d/a d/b && git add .

  touch a.o d/a.o d/b.o
  touch d/d/new d/d/a.precious

  git commit -m "init"
)

git init expendable-and-precious
(cd expendable-and-precious
  echo "*.o" >> .gitignore
  echo '$precious' >> .gitignore
  echo '$/mixed/precious' >> .gitignore
  echo '$/all-precious/' >> .gitignore
  echo "/all-expendable/" >> .gitignore
  echo '$*.precious' >> .gitignore

  git add .gitignore

  touch a.o
  touch precious
  mkdir mixed
  touch mixed/precious mixed/b.o

  (mkdir some-expendable && cd some-expendable
    touch file.o file new && git add file
  )

  (mkdir some-precious && cd some-precious
    touch file.precious file new && git add file
  )

  mkdir all-precious all-expendable all-precious-by-filematch all-expendable-by-filematch
  touch all-precious/a all-precious/b all-expendable/c all-expendable/d
  (cd all-precious-by-filematch
    touch a.precious b.precious
  )
  (cd all-expendable-by-filematch
    touch e.o f.o
  )

  git commit -m "init"
)

git init expendable-and-precious-nested-in-ignored-dir
(cd expendable-and-precious-nested-in-ignored-dir
  echo 'ignored/' > .gitignore
  git add .gitignore && git commit -m "init"
  mkdir -p ignored/other
  cp -R ../expendable-and-precious ignored/d
  rm -Rf ignored/d/*-by-filematch ignored/d/some-*
  mkdir -p other/ignored && >other/ignored/a
)


mkdir empty-and-untracked-dir
(cd empty-and-untracked-dir
  mkdir empty untracked
  >untracked/file
)


mkdir complex-empty
(cd complex-empty
  mkdir empty-toplevel
  mkdir -p only-dirs/sub/subsub only-dirs/other
  mkdir -p dirs-and-files/sub dirs-and-files/dir
  touch dirs-and-files/dir/file
)

git init type-mismatch
(cd type-mismatch
  mkdir dir-is-file && >dir-is-file/a
  >file-is-dir
  git add .
  rm -Rf dir-is-file
  >dir-is-file
  rm file-is-dir && mkdir file-is-dir && >file-is-dir/b
)

git init type-mismatch-icase
(cd type-mismatch-icase
  mkdir dir-is-file && >dir-is-file/a
  >file-is-dir
  git add .
  rm -Rf dir-is-file
  >Dir-is-File
  rm file-is-dir && mkdir File-is-Dir && >File-is-Dir/b
)

git init untracked-cache-icase
(cd untracked-cache-icase
  mkdir Dir && >Dir/tracked
  git add . && git commit -m "tracked directory"
  mv Dir dir
  >dir/untracked
  git config core.ignoreCase true
)

git init type-mismatch-icase-clash-dir-is-file
(cd type-mismatch-icase-clash-dir-is-file
  empty_oid=$(git hash-object -w --stdin </dev/null)
  git update-index --index-info <<EOF
100644 $empty_oid	D/a
100644 $empty_oid	d
EOF
  >d
)

cp -R type-mismatch-icase-clash-dir-is-file type-mismatch-icase-clash-file-is-dir
(cd type-mismatch-icase-clash-file-is-dir
  rm d
  mkdir D && >D/a
)
mkdir empty
touch just-a-file

git init submodule
(cd submodule
  touch empty && git add empty
  git commit -m upstream
)

git clone submodule multiple-submodules
(cd multiple-submodules
  git submodule add ../submodule submodule
  git submodule add ../submodule a/b
  git commit -m "add modules"
)

git clone submodule one-ignored-submodule
(cd one-ignored-submodule
  git submodule add ../submodule submodule
  echo '/submodule/' > .gitignore
  echo '*' > submodule/.gitignore
  git commit -m "add seemingly ignored submodule"
)

git init slash-in-root-and-negated
(cd slash-in-root-and-negated
  cat <<'EOF' >.gitignore
/
!file
!*.md
!.github
!.github/**
EOF
  touch file readme.md
  mkdir .github
  touch .github/workflow.yml
  git add .github readme.md .gitignore
  git commit -m "init"
)

git init star-in-root-and-negated
(cd star-in-root-and-negated
  cat <<'EOF' >.gitignore
*
!file
!.gitignore
!*.md
!.github
!.github/**
EOF
  touch file readme.md
  mkdir .github
  touch .github/workflow.yml
  git add .github readme.md .gitignore
  git commit -m "init"
)

git init slash-in-subdir-and-negated
(cd slash-in-subdir-and-negated
  mkdir sub
  (cd sub
    cat <<'EOF' >.gitignore
/
!file
!*.md
!.github
!.github/**
EOF
    touch file readme.md
    mkdir .github
    touch .github/workflow.yml
    git add .github readme.md .gitignore
    git commit -m "init"
  )
)

git init star-in-subdir-and-negated
(cd star-in-subdir-and-negated
  mkdir sub
  (cd sub
    cat <<'EOF' >.gitignore
*
!file
!.gitignore
!*.md
!.github
!.github/**
EOF
    touch file readme.md
    mkdir .github
    touch .github/workflow.yml
    git add .github readme.md .gitignore
    git commit -m "init"
  )
)

git init with-sub-repo
(cd with-sub-repo
  echo '*' > .gitignore
  git add -f .gitignore
  git clone ../dir-with-file sub-repo
)

git clone dir-with-tracked-file in-repo-worktree
(cd in-repo-worktree
  git worktree add worktree
  git worktree add -b other-worktree dir/worktree
)

git clone dir-with-tracked-file in-repo-hidden-worktree
(cd in-repo-hidden-worktree
  echo '/hidden/' > .gitignore
  mkdir -p hidden/subdir
  touch hidden/file
  git worktree add -b worktree-branch hidden/subdir/worktree
)

# Populate UNTR caches so every directory-walk test exercises cache eligibility.
find . -name .git -prune | while read -r git_dir
do
  repo=${git_dir%/.git}
  git -C "$repo" config core.untrackedCache true
  git -C "$repo" config core.excludesFile .git/no-global-excludes
  git -C "$repo" update-index --untracked-cache --index-version 2
  git -C "$repo" status --porcelain --ignore-submodules=all >/dev/null
done

# Make the root cache stale while leaving its child's stat and local excludes unchanged.
>untracked-cache-changed-parent-ignore/.gitignore
