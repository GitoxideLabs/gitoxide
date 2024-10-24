#!/usr/bin/env bash
set -eu -o pipefail

function tick () {
  if test -z "${tick+set}"
  then
    tick=1112911993
  else
    tick=$(($tick + 60))
  fi
  GIT_COMMITTER_DATE="$tick -0700"
  GIT_AUTHOR_DATE="$tick -0700"
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE
}

function write_lines () {
	printf "%s\n" "$@"
}

function baseline () (
  local dir=${1:?the directory to enter}
  local output_name=${2:?the basename of the output of the merge}
  local our_committish=${3:?our side from which a commit can be derived}
  local their_committish=${4:?Their side from which a commit can be derived}

  cd "$dir"
  local our_commit_id
  local their_commit_id

  our_commit_id="$(git rev-parse "$our_committish")"
  their_commit_id="$(git rev-parse "$their_committish")"

  local merge_info="${output_name}.merge-info"
  git merge-tree -z --write-tree "$our_committish" "$their_committish" > "$merge_info" || :
  echo "$dir" "$our_commit_id" "$our_committish" "$their_commit_id" "$their_committish" "$merge_info" >> ../baseline.cases

  local merge_info="${output_name}-reversed.merge-info"
  git merge-tree -z --write-tree "$their_committish" "$our_committish" > "$merge_info" || :
  echo "$dir" "$their_commit_id" "$their_committish" "$our_commit_id" "$our_committish" "$merge_info" >> ../baseline.cases
)

git init simple
(cd simple
  rm -Rf .git/hooks
  write_lines 1 2 3 4 5 >numbers
  echo hello >greeting
  echo foo >whatever
  git add numbers greeting whatever
  tick
  git commit -m initial

  git branch side1
  git branch side2
  git branch side3
  git branch side4

  git checkout side1
  write_lines 1 2 3 4 5 6 >numbers
  echo hi >greeting
  echo bar >whatever
  git add numbers greeting whatever
  tick
  git commit -m modify-stuff

  git checkout side2
  write_lines 0 1 2 3 4 5 >numbers
  echo yo >greeting
  git rm whatever
  mkdir whatever
  >whatever/empty
  git add numbers greeting whatever/empty
  tick
  git commit -m other-modifications

  git checkout side3
  git mv numbers sequence
  tick
  git commit -m rename-numbers

  git checkout side4
  write_lines 0 1 2 3 4 5 >numbers
  echo yo >greeting
  git add numbers greeting
  tick
  git commit -m other-content-modifications

  git switch --orphan unrelated
  >something-else
  git add something-else
  tick
  git commit -m first-commit
)

git init rename-delete
(cd rename-delete
  write_lines 1 2 3 4 5 >foo
  mkdir olddir
  for i in a b c; do echo $i >olddir/$i; done
  git add foo olddir
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >foo
  git add foo
  git mv olddir newdir
  git commit -m "Modify foo, rename olddir to newdir"

  git checkout B
  write_lines 1 2 3 4 5 six >foo
  git add foo
  git mv foo olddir/bar
  git commit -m "Modify foo & rename foo -> olddir/bar"
)

git init rename-add
(cd rename-add
		write_lines original 1 2 3 4 5 >foo
		git add foo
		git commit -m "original"

		git branch A
		git branch B

		git checkout A
		write_lines 1 2 3 4 5 >foo
		echo "different file" >bar
		git add foo bar
		git commit -m "Modify foo, add bar"

		git checkout B
		write_lines original 1 2 3 4 5 6 >foo
		git add foo
		git mv foo bar
		git commit -m "rename foo to bar"
)

git init rename-add-symlink
(cd rename-add-symlink
  write_lines original 1 2 3 4 5 >foo
  git add foo
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 >foo
  ln -s foo bar
  git add foo bar
  git commit -m "Modify foo, add symlink bar"

  git checkout B
  write_lines original 1 2 3 4 5 6 >foo
  git add foo
  git mv foo bar
  git commit -m "rename foo to bar"
)

git init rename-rename-plus-content
(cd rename-rename-plus-content
  write_lines 1 2 3 4 5 >foo
  git add foo
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 six >foo
  git add foo
  git mv foo bar
  git commit -m "Modify foo + rename to bar"

  git checkout B
  write_lines 1 2 3 4 5 6 >foo
  git add foo
  git mv foo baz
  git commit -m "Modify foo + rename to baz"
)
  
git init rename-add-delete
(
  cd rename-add-delete
  echo "original file" >foo
  git add foo
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  git rm foo
  echo "different file" >bar
  git add bar
  git commit -m "Remove foo, add bar"

  git checkout B
  git mv foo bar
  git commit -m "rename foo to bar"
)

baseline simple without-conflict side1 side3
baseline simple various-conflicts side1 side2
baseline simple single-content-conflict side1 side4
baseline rename-delete A-B A B
baseline rename-delete A-similar A A
baseline rename-delete B-similar B B
baseline rename-add A-B A B
baseline rename-add-symlink A-B A B
baseline rename-rename-plus-content A-B A B
baseline rename-add-delete A-B A B
