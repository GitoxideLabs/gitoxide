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

# Translate a historical SHA-1 object id (as hard-coded in the `git update-index
# --index-info` blocks below) to the equivalent id under the repository's
# current hash algorithm, so these baselines work under both SHA-1 and SHA-256.
function oid () {
  case "${GIT_DEFAULT_HASH:-sha1}" in
  sha1)
    printf '%s' "$1"
    return
    ;;
  sha256)
    ;;
  *)
    echo "oid(): unsupported object hash '${GIT_DEFAULT_HASH}', expected 'sha1' or 'sha256'" >&2
    return 1
    ;;
  esac
  case "$1" in
    092bfb9bdf74dd8cfd22e812151281ee9aa6f01a) echo d246bf89e9d0f356301a75184ab79e68cfcb0aac1a7acbf3d4de81400b5ee231 ;;
    09c277aa66897c58157f57a374eacc63a407dcab) echo cf8aedf347c947abf6c6e2c4ecaa2e086ea61ce81f52491c453b81c6455f305e ;;
    0a6a0ba83635bc00e7c79a4b5b6e50381385c1af) echo be45c0023fb9c5e05387235ef07be7e7e2034838216cd24cca06840cc01bc771 ;;
    0cde534c2fca6c92c07b3e7a696665e844b9b933) echo cc0d90a1291b62ffbf9288fad4e290e3a85160cef42c708be3b8a90ac13d501b ;;
    19102815663d23f8b75a47e7a01965dcdc96468c) echo a65ca4376b51e98097fec3a009b7e5adce3255aecaa1a6da7b1cdc92b90e025e ;;
    1a2664a9924754c698e323f756f9f87f3f2fb337) echo cff0bb6fba5633bd804f0a15fed6329ede0a9dfc4a7f53e31581f3d1a2005e0f ;;
    257cc5642cb1a054f08cc83f2d943e56fd3ebe99) echo 47d6aca82756ff2e61e53520bfdf1faa6c86d933be4854eb34840c57d12e0c85 ;;
    4178ea6795c4c3e07b4e17e6a04aa49584b07ecd) echo 60becdf11d1fcac5bdafbebf1f3e5c7dec1463f8381bcc8bfb533262ec3b1fa8 ;;
    44065282f89b9bd6439ed2e4674721383fd987eb) echo 6632209c4f376d0d893dcb49cd85f0abf9fb1bb0f79c2c6c998adef4d95cac95 ;;
    45b983be36b73c0788dc9cbcb76cbb80fc7bb057) echo 96c18f0297e38d01f4b2dacddea4259aea6b2961eb0822bd2c0c3f6029030045 ;;
    4b5599c7c2ed4390417d9699bec86144a386873d) echo 54b65f98d2bdee38cacc335639121482f497d71b6c9874002b637173f2cc212b ;;
    542802a799ded74fa01c47ba2f8925e284a369e2) echo 863a67a88f3ae07c9c4784fbed16d4f8eabea50aa40923804f7435c45c4bcb1e ;;
    5716ca5987cbf97d6bb54920bea6adde242d87e6) echo a52e146ac2ab2d0efbb768ab8ebd1e98a6055764c81fe424fbae4522f5b4cb92 ;;
    61780798228d17af2d34fce4cfbdf35556832472) echo 9b69d308c97f2c5933fdd0e8ce04acce91c09cb969e36a1f86756fc5a5d3323a ;;
    64012489f118cb4011c8902b4a635f70dcb0c0ca) echo 24bba3b983b179b8ceb5b7af0995c745dbe07f36b75e593b28e000c7d26aa1e5 ;;
    65bc6a1e238f4bf05b28fd05240636e2cfb657e0) echo 315347e306b9d8222324e6f48a1f585074712267eae89d34ef607663046cb3e3 ;;
    78981922613b2afb6025042ff6bd878ac1994e85) echo f8625e43f9e04f24291f77cdbe4c71b3c2a3b0003f60419b3ed06a058d766c8b ;;
    8a1218a1024a212bb3db30becd860315f9f3ac52) echo 6841122c240e69074e82e38506c1fcf806c3ee2469673392a9ed7650d7a6d51d ;;
    9dc97bdc2426e68423360e3e5299280b2cf6b8ff) echo 7108d070fd9fde47d46504102c6935d9251cce816fdbdcc6c1d56710f11790d1 ;;
    a4ae6e4709228b5da6001cb9d1cfa7736851e2a6) echo c3ae6188533b9bea9ad1049892c6b5488fffe5cfc518b30ba21b64a124bc7afe ;;
    b414108e81e5091fe0974a1858b4d0d22b107f70) echo f5f52958fe19d6073227d52208826e0d189bfdb9d0c428984244075155e57c71 ;;
    d0549c3d3c96a464289f3b820b7d96aedc58924b) echo 7960eaf2c5f3ac192aa208323a78bf4b0f2fb21d79813af79a78674698689442 ;;
    d5f7fc3f74f7dec08280f370a975b112e8f60818) echo ffc23e0956239fed93c95c4cf3d3152a887825f756309251d71bb16f261afabd ;;
    df967b96a579e45a18b8251732d16804b2e56a55) echo abed979e3cd3667c5a295c2641f8319f950860c65cc168eebd1571c51bb4f6fc ;;
    e29fa63dae4ccf0788897a7025da868083178fdf) echo 577603f537e595c5324b5987646cb9073d159a4f0c9e73f820d2ee83c8926acf ;;
    e33f5e94470d3b5fa0220ff6a9cabb78a3f72fa3) echo 72a7259eef0664ef8dae2039c11c61ec4dd039f94eee87a21c6ee1c637b7cca0 ;;
    e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) echo 473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813 ;;
    ea28dcd7f627a2a7bbd09daa679c452180617c9f) echo efd6b0b052df02752b7f371021660541dcb8f2b0665c19b672b031af7c086588 ;;
    f286e5cdd97ac6895438ea4548638bb98ac9bd6b) echo 33ba40b57f32a1383518de4f91d9eaf8bf5eb55e318f47215af5c367f9778b9b ;;
    f2ad6c76f0115a6ba5b00456a849810e7ec0af20) echo 2abe107e3b1b618efafa0df5e5f1118e5bf86694eb8c185741e67795ae314aa4 ;;
    f801a62deed900f8a80ff35e3339474ad6352a93) echo 970d16592cccbb8ce3846be7a74ecb94bc3bf8d01bec1eeb15a57380aae5d5a3 ;;
    f89a08d1e226b9a319210641b63b07dcf0bd705f) echo 43d8308eb726995b8b6a3f826252dd21dc35d01386931b8eea4125723e36cc83 ;;
    fa49b077972391ad58037050f2a75f74e3671e92) echo 6d5fd291bb0f67444e99ab492f1bf1fcdf5dca09dab24cf331e05111b4cfc1a3 ;;
    # gitlinks: submodule 'sub' commits (sub-root, sub-a, sub-b), deterministic from the fixed tick dates
    e835c0c403c8e494c0ca98f3d25d0b8464c18d38) echo ed2d65b7fe3c3f869a21cfec7f54867db6c8eb2cc9b0bf671522d87599704d70 ;;
    64466ebdff775ad618d9cc993cf52840e0af528c) echo df5d889015388598b724a741b53437e4dee9d67a5dbe0fa9f361fedd4c9ac6e8 ;;
    ea6eb701e03c2497915c25a851f3da8f8e362ca0) echo c79d38d6c8161bf2aec16032f82127c7ac237eeab69e1490422c93b5c314ce9c ;;
    *) echo "oid(): no SHA-256 mapping for '$1'" >&2; return 1 ;;
  esac
}

function write_lines () {
	printf "%s\n" "$@"
}

function seq () {
	case $# in
	1)	set 1 "$@" ;;
	2)	;;
	*)	{ echo "need 1 or 2 parameters: <end> or <start> <end>" 1>&2 && exit 2; } ;;
	esac
	local seq_counter=$1
	while test "$seq_counter" -le "$2"
	do
		echo "$seq_counter"
		seq_counter=$(( seq_counter + 1 ))
	done
}

function make_conflict_index() {
  local identifier=${1:?The first argument is the name of the parent directory along with the output name}
  cp .git/index .git/"${identifier}".index
}

function make_resolve_tree() {
  local resolve=${1:?Their 'ancestor' or 'ours'}
  local our_side=${2:-}
  local their_side=${3:-}

  local filename="resolve-${our_side}-${their_side}-with-${resolve}"
  git write-tree > ".git/${filename}.tree"
}

function baseline () (
  local dir=${1:?the directory to enter}
  local output_name=${2:?the basename of the output of the merge}
  local our_committish=${3:?our side from which a commit can be derived}
  local their_committish=${4:?Their side from which a commit can be derived}
  local opt_deviation_message=${5:-}
  local one_side=${6:-}

  cd "$dir"
  local our_commit_id
  local their_commit_id

  local conflict_style="merge"
   if [[ "$output_name" == *-merge ]]; then
       conflict_style="merge"
   elif [[ "$output_name" == *-diff3 ]]; then
       conflict_style="diff3"
   fi

  our_commit_id="$(git rev-parse "$our_committish")"
  their_commit_id="$(git rev-parse "$their_committish")"
  local maybe_expected_tree="$(git rev-parse expected^{tree})"
  local maybe_expected_reversed_tree="$(git rev-parse expected-reversed^{tree})"
  if [ "$maybe_expected_reversed_tree" == "expected-reversed^{tree}" ]; then
     maybe_expected_reversed_tree="$(git rev-parse expected^{tree} || :)"
  fi
  if [ -z "$opt_deviation_message" ]; then
    maybe_expected_tree="expected^{tree}"
    maybe_expected_reversed_tree="expected^{tree}"
  fi

  local merge_info="${output_name}.merge-info"
  git -c merge.conflictStyle=$conflict_style merge-tree -z --write-tree --allow-unrelated-histories "$our_committish" "$their_committish" > "$merge_info" || :
  echo "$dir" "$conflict_style" "$our_commit_id" "$our_committish" "$their_commit_id" "$their_committish" "$merge_info" "$maybe_expected_tree" "$opt_deviation_message" >> ../baseline.cases

  if [[ "$one_side" != "no-reverse" ]]; then
    local merge_info="${output_name}-reversed.merge-info"
    git -c merge.conflictStyle=$conflict_style merge-tree -z --write-tree --allow-unrelated-histories "$their_committish" "$our_committish" > "$merge_info" || :
    echo "$dir" "$conflict_style" "$their_commit_id" "$their_committish" "$our_commit_id" "$our_committish" "$merge_info" "$maybe_expected_reversed_tree" "$opt_deviation_message" >> ../baseline.cases
  fi
)


git init non-tree-to-tree
(cd non-tree-to-tree
  write_lines original 1 2 3 4 5 >a
  git add a && git commit -m "init"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >a
  git commit -am "'A' changes 'a'"

  git checkout B
  rm a
  mkdir -p a/sub
  touch a/sub/b a/sub/c a/d a/e
  git add a && git commit -m "mv 'a' to 'a/sub/b', populate 'a/' with empty files"
)

git init deleted-file-added-dir
(cd deleted-file-added-dir
  echo original >to-be-deleted
  git add to-be-deleted && git commit -m "init"

  git branch A
  git branch B

  git checkout A
  git rm to-be-deleted
  git commit -m "delete file"

  git checkout B
  git rm to-be-deleted
  mkdir to-be-deleted
  touch to-be-deleted/a
  git add to-be-deleted/a && git commit -m "replace file with directory"
)

git init deleted-file-added-gitlink-directory
(cd deleted-file-added-gitlink-directory
  write_lines original >a
  git add a
  git commit -m "file base"
  base=$(git rev-parse HEAD)

  git branch A
  git branch B

  # Both sides delete `a`; B additionally replaces it with a directory containing
  # a gitlink. The shared deletion and descendant addition are compatible even
  # though the gitlink takes a different structural merge path than a blob.
  git checkout A
  git rm a
  git commit -m "delete a"

  git checkout B
  git rm a
  git update-index --add --cacheinfo 160000,$base,a/a
  git commit -m "replace a with a gitlink directory"
)

git init tree-to-non-tree
(cd tree-to-non-tree
  mkdir -p a/sub
  write_lines original 1 2 3 4 5 >a/sub/b
  touch a/sub/c a/d a/e
  git add a && git commit -m "init"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >a/sub/b
  git commit -am "'A' changes 'a/sub/b'"

  git checkout B
  rm -Rf a
  echo "new file" > a
  git add a && git commit -m "rm -Rf a/ && add non-empty 'a'"
)

git init non-tree-to-tree-with-rename
(cd non-tree-to-tree-with-rename
  write_lines original 1 2 3 4 5 >a
  git add a && git commit -m "init"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >a
  git commit -am "'A' changes 'a'"

  git checkout B
  mv a tmp
  mkdir -p a/sub
  mv tmp a/sub/b
  touch a/sub/c a/d a/e
  git add a && git commit -m "mv 'a' to 'a/sub/b', populate 'a/' with empty files"
)

git init tree-to-non-tree-with-rename
(cd tree-to-non-tree-with-rename
  mkdir -p a/sub
  write_lines original 1 2 3 4 5 >a/sub/b
  touch a/sub/c a/d a/e
  git add a && git commit -m "init"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >a/sub/b
  git commit -am "'A' changes 'a/sub/b'"

  git checkout B
  rm -Rf a
  touch a
  git add a && git commit -m "rm -Rf a/ && add empty 'a' (which is like a rename from an empty deleted file)"
  # And because it's so thrown off, it gets a completely different result if reversed.
  git branch expected-reversed

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/b
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a/sub/b
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a~B
EOF
  make_conflict_index tree-to-non-tree-with-rename-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/b
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a/sub/b
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a~B
EOF
  make_conflict_index tree-to-non-tree-with-rename-A-B-reversed
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

  git checkout -b tweak1 side1
  write_lines zero 1 2 3 4 5 6 >numbers
  git add numbers
  git mv numbers "Αυτά μου φαίνονται κινέζικα"
  git commit -m "Renamed numbers"
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

git init rename-change-matrix
(cd rename-change-matrix
  # Each top-level directory is one independent rename interaction:
  #
  # * modify-source:
  #   A renames `file` to `renamed`; B modifies `file`.
  #   The modification follows the rename and the result is clean at `renamed`.
  # * delete-source:
  #   A renames `file` to `renamed`; B deletes `file`.
  #   This is a rename/delete conflict, with A's renamed file retained.
  # * add-destination:
  #   A renames `file` to `target`; B adds an unrelated `target`.
  #   Both additions occupy the rename destination, producing an add/add conflict.
  # * modify-destination:
  #   A replaces the existing `target` by renaming `source` onto it; B modifies the old `target`.
  #   The merge remains tied to the destination path: the base is the old `target`, A replaces all
  #   of it with `source`, and B appends to the old `target`. Those edits conflict, preserving the
  #   complete A and B versions inside conflict markers.
  # * delete-destination:
  #   A replaces the existing `target` by renaming `source` onto it; B deletes the old `target`.
  #   The deletion conflicts with A's replacement, which remains at `target`.
  # * different-renames:
  #   A and B both modify `file`, then rename it to different destinations.
  #   Both destinations contain the one merged result, with a content conflict for the two appends.
  #   Its configured marker size is 10; the rename/rename conflict adds one, yielding 11.
  # * directory-old:
  #   A renames the directory; B adds a child below its old name.
  #   The child follows the directory rename and the result is clean below `directory-renamed`.
  #
  # Distinct repeated words keep rename similarity pairing local to each case.
  mkdir -p modify-source delete-source add-destination modify-destination delete-destination \
    different-renames directory-old
  write_lines alpha alpha alpha alpha alpha >modify-source/file
  write_lines bravo bravo bravo bravo bravo >delete-source/file
  write_lines charlie charlie charlie charlie charlie >add-destination/file
  write_lines delta-source delta-source delta-source delta-source delta-source >modify-destination/source
  write_lines delta-target delta-target delta-target delta-target delta-target >modify-destination/target
  write_lines echo-source echo-source echo-source echo-source echo-source >delete-destination/source
  write_lines echo-target echo-target echo-target echo-target echo-target >delete-destination/target
  write_lines golf golf golf golf golf >different-renames/file
  write_lines juliet juliet juliet juliet juliet >directory-old/file
  echo 'different-renames/* conflict-marker-size=10' >.gitattributes
  git add . && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git mv modify-source/file modify-source/renamed
  git mv delete-source/file delete-source/renamed
  git mv add-destination/file add-destination/target
  git rm modify-destination/target
  git mv modify-destination/source modify-destination/target
  git rm delete-destination/target
  git mv delete-destination/source delete-destination/target
  echo changed-by-A >>different-renames/file
  git mv different-renames/file different-renames/ours
  git mv directory-old directory-renamed
  git commit -am "rename one side of each matrix entry"

  git checkout B
  echo changed-by-B >>modify-source/file
  git rm delete-source/file
  echo added-by-B >add-destination/target
  echo changed-by-B >>modify-destination/target
  git rm delete-destination/target
  echo changed-by-B >>different-renames/file
  git mv different-renames/file different-renames/theirs
  echo added-by-B >directory-old/added
  git add . && git commit -m "apply the other change in each matrix entry"

)

git init same-rename-with-content
(cd same-rename-with-content
  # Both sides modify the same source and rename it to the same destination.
  # The two appends conflict, but the rename itself agrees. The result should therefore be
  # exactly one content merge at `target`, never a second merge of an already-merged blob.
  write_lines foxtrot foxtrot foxtrot foxtrot foxtrot >file
  git add file && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  echo changed-by-A >>file
  git mv file target
  git commit -am "A renames and changes file"

  git checkout B
  echo changed-by-B >>file
  git mv file target
  git commit -am "B renames and changes file"

  # Both sides agree on the rename, so this is an ordinary content conflict using the
  # configured marker size. These branches provide the merged blob for gix's index shape.
  git checkout -b expected main
  git rm file
  write_lines \
    foxtrot foxtrot foxtrot foxtrot foxtrot \
    '<<<<<<< A' \
    changed-by-A \
    ======= \
    changed-by-B \
    '>>>>>>> B' >target
  git add target && git commit -m "single content merge at shared rename destination"

  git checkout -b expected-reversed main
  git rm file
  write_lines \
    foxtrot foxtrot foxtrot foxtrot foxtrot \
    '<<<<<<< B' \
    changed-by-B \
    ======= \
    changed-by-A \
    '>>>>>>> A' >target
  git add target && git commit -m "single reversed content merge at shared rename destination"
)

git init same-rename-and-file-to-directory
(cd same-rename-and-file-to-directory
  # Both sides perform the same compound operation:
  #
  # * rename `source` to `moved`;
  # * replace the old `source` path with a directory;
  # * add different content at `source/child`.
  #
  # The identical rewrite must be applied only once. Applying it again after merging the
  # children would recursively remove `source` and silently discard both payloads.
  write_lines source source source source source >source
  git add source && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git mv source moved
  mkdir source
  echo changed-by-A >source/child
  git add . && git commit -m "A renames source and adds a child at its old path"

  git checkout B
  git mv source moved
  mkdir source
  echo changed-by-B >source/child
  git add . && git commit -m "B renames source and adds a child at its old path"
)

git init renames-to-same-destination
(cd renames-to-same-destination
  # The sides rename different source files onto the same destination:
  #
  # * A: `one` -> `target`
  # * B: `two` -> `target`
  #
  # Neither rename should silently win. A normal merge removes both sources and records the
  # two destination contents as an add/add conflict. Forced ancestor resolution keeps `one`
  # and `two`; forced ours applies only the rename belonging to the selected first side.
  write_lines one one one one one >one
  write_lines two two two two two >two
  git add . && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git mv one target
  git commit -m "rename one to target"

  git checkout B
  git mv two target
  git commit -m "rename two to target"
)

git init identical-renames-to-same-destination
(cd identical-renames-to-same-destination
  write_lines same >one
  cp one two
  git add .
  git commit -m "two identical files"

  git branch A
  git branch B

  # Both sides rename a different source to `target`, but the entries have the
  # same mode and object ID. There is nothing to content-merge and the two
  # operations collapse cleanly to the shared destination in either direction.
  git checkout A
  git mv one target
  git commit -m "rename one to target"

  git checkout B
  git mv two target
  git commit -m "rename two to target"
)

git init identical-renames-to-same-destination-with-mode-change
(cd identical-renames-to-same-destination-with-mode-change
  write_lines same >one
  cp one two
  git add .
  git commit -m "two identical files"

  git branch A
  git branch B

  # The destinations have the same blob ID but differ in executable mode.
  # Content merging must see those original modes even though the final mode
  # has already been selected.
  git checkout A
  git mv one target
  chmod +x target
  # For this to work on windows, we need explicit executable bit handling.
  git update-index --chmod=+x target
  git commit -m "rename one to executable target"

  git checkout B
  git mv two target
  git commit -m "rename two to target"

  # gix treats the identical content as clean and carries the executable mode
  # selected from A to the shared destination in both merge directions.
  git checkout -b expected A
  git rm two
  git commit -m "expected gix merge"
)

git init deleted-file-added-dir-with-rename
(cd deleted-file-added-dir-with-rename
  # Regression for a deletion that is processed but not applied:
  #
  # * A deletes the file `x`.
  # * B renames that file to `renamed`, then creates the directory entry `x/a`.
  #
  # With ancestor conflict resolution, A's deletion must not count as applied merely because
  # the rename/delete conflict was handled. The ancestor file `x` remains, so `x/a` cannot
  # turn it into a directory.
  echo base >x
  git add x
  git commit -m "add x"

  git branch A
  git branch B

  git checkout A
  git rm x
  git commit -m "delete x"

  git checkout B
  git mv x renamed
  mkdir x
  echo added >x/a
  git add x/a
  git commit -m "rename x and add x/a"
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

git init rename-add-exe-bit-conflict
(cd rename-add-exe-bit-conflict
		touch a b
		chmod +x a
    git add --chmod=+x a
		git add b
		git commit -m "original"

		git branch A
		git branch B

		git checkout A
		chmod -x a
    git update-index --chmod=-x a
		git commit -m "-x a"

		git checkout B
		git mv --force b a
		chmod +x a
    git update-index --chmod=+x a
		git commit -m "mv b a; chmod +x a"
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

git init rename-add-same-symlink
(cd rename-add-same-symlink
  touch target
  ln -s target link
  git add .
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  git mv link link-new
  git commit -m "rename link to link-new"

  git checkout B
  ln -s target link-new
  git add link-new
  git commit -m "create link-new"
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

git init rename-rename-delete-delete
(
  cd rename-rename-delete-delete
  echo foo >foo
  echo bar >bar
  git add foo bar
  git commit -m O

  git branch A
  git branch B

  git checkout A
  git mv foo baz
  git rm bar
  git commit -m "Rename foo, remove bar"

  git checkout B
  git mv bar baz
  git rm foo
  git commit -m "Rename bar, remove foo"
)

git init super-1
(cd super-1
  seq 11 19 >one
  seq 31 39 >three
  seq 51 59 >five
  git add .
  tick
  git commit -m "O"

  git branch A
  git branch B

  git checkout A
  seq 10 19 >one
  echo 40        >>three
  git add one three
  git mv  one   two
  git mv  three four
  git mv  five  six
  tick
  git commit -m "A"

  git checkout B
  echo 20    >>one
  echo forty >>three
  echo 60    >>five
  git add one three five
  git mv  one   six
  git mv  three two
  git mv  five  four
  tick
  git commit -m "B"
)

git init super-2
(cd super-2
  write_lines 1 2 3 4 5 >foo
  mkdir olddir
  for i in a b c; do echo $i >olddir/$i || exit 1; done
  git add foo olddir
  git commit -m "original"

  git branch A
  git branch B

  git checkout A
  git rm foo
  git mv olddir newdir
  mkdir newdir/bar
  >newdir/bar/file
  git add newdir/bar/file
  git commit -m "rm foo, olddir/ -> newdir/, + newdir/bar/file"

  git checkout B
  write_lines 1 2 3 4 5 6 >foo
  git add foo
  git mv foo olddir/bar
  git commit -m "Modify foo & rename foo -> olddir/bar"

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 78981922613b2afb6025042ff6bd878ac1994e85) 0	newdir/a
100644 $(oid 61780798228d17af2d34fce4cfbdf35556832472) 0	newdir/b
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	newdir/bar/file
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	newdir/bar~B
100644 $(oid f2ad6c76f0115a6ba5b00456a849810e7ec0af20) 0	newdir/c
EOF
  # Git also has
  # 100644 b414108e81e5091fe0974a1858b4d0d22b107f70 1	newdir/bar~B
  # which then looks like "deleted by us: newdir/bar-B`
  # Our index here doesn't manage to track the base across so many renames, but it ends up looking like
  # `added by them: newdir/bar~B` which to my mind is more helpful, in a situation where the index simply
  # cannot properly show what happened.
  make_conflict_index super-2-A-B
  make_conflict_index super-2-A-B-diff3

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 78981922613b2afb6025042ff6bd878ac1994e85) 0	newdir/a
100644 $(oid 61780798228d17af2d34fce4cfbdf35556832472) 0	newdir/b
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	newdir/bar/file
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	newdir/bar~B
100644 $(oid f2ad6c76f0115a6ba5b00456a849810e7ec0af20) 0	newdir/c
EOF
  make_conflict_index super-2-A-B-reversed
  make_conflict_index super-2-A-B-diff3-reversed
)

git init rename-within-rename
(cd rename-within-rename
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  mkdir a/sub && write_lines original 1 2 3 4 5 >a/sub/y.f
  touch a/w a/sub/z
  git add . && git commit -m "original"

  git branch A
  git branch B
  git branch expected

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  write_lines 1 2 3 4 5 >a/sub/y.f
  git mv a a-renamed
  git commit -am "changed all content, renamed a -> a-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  write_lines original 1 2 3 4 5 6 >a/sub/y.f
  git mv a/sub a/sub-renamed
  git commit -am "changed all content, renamed a/sub -> a/sub-renamed"

  git checkout expected
  write_lines 1 2 3 4 5 6 >a/x.f
  write_lines 1 2 3 4 5 6 >a/sub/y.f
  cp -Rv a/sub a/sub-renamed
  git add .
  git mv a a-renamed
  git commit -am "we also have duplication just like Git, but we are consistent independently of the side, hence the expectation"

  # We have duplication just like Git, but our index is definitely more complex. This one seems more plausible.
  # The problem is that renames can't be indicated correctly in the index.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-renamed/sub/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/sub-renamed/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a/sub-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
EOF
  make_conflict_index rename-within-rename-A-B-deviates
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-renamed/sub/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/sub-renamed/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a/sub-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
EOF
  make_conflict_index rename-within-rename-A-B-deviates-reversed
)

git init rename-within-rename-2
(cd rename-within-rename-2
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  mkdir a/sub && write_lines original 1 2 3 4 5 >a/sub/y.f
  touch a/w a/sub/z
  git add . && git commit -m "original"

  git branch A
  git branch B
  git branch expected

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  write_lines 1 2 3 4 5 >a/sub/y.f
  git mv a/sub a/sub-renamed
  git mv a a-renamed
  git commit -am "changed all content, renamed a -> a-renamed, a/sub -> a/sub-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  write_lines original 1 2 3 4 5 6 >a/sub/y.f
  git mv a/sub a/sub-renamed
  git commit -am "changed all content, renamed a/sub -> a/sub-renamed"

  git checkout expected
  write_lines 1 2 3 4 5 6 >a/x.f
  write_lines 1 2 3 4 5 6 >a/sub/y.f
  git mv a/sub a/sub-renamed
  git mv a a-renamed
  git commit -am "tracked both renames, applied all modifications by merge"
  # Both merge directions yield this tree, so `expected` serves the reversed case as well.


  # Both directions produce the same cleanly merged state, and the index is the same as well.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/sub-renamed/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
EOF
  make_conflict_index rename-within-rename-2-A-B-deviates
  make_conflict_index rename-within-rename-2-A-B-deviates-reversed
)

git init conflicting-rename
(cd conflicting-rename
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  mkdir a/sub && write_lines original 1 2 3 4 5 >a/sub/y.f
  touch a/w a/sub/z
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  write_lines 1 2 3 4 5 >a/sub/y.f
  git mv a a-renamed
  git commit -am "changed all content, renamed a -> a-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  write_lines original 1 2 3 4 5 6 >a/sub/y.f
  git mv a a-different
  git commit -am "changed all content, renamed a -> a-different"

# Git only sees the files with content changes as conflicting, and somehow misses to add the
# bases of the files without content changes. After all, these also have been renamed into
# different places which must be a conflict just as much.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-different/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-different/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-different/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-different/x.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-renamed/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-renamed/x.f
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/x.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/w
EOF
  make_conflict_index conflicting-rename-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-different/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-different/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-different/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-different/x.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-renamed/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-renamed/x.f
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/x.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/w
EOF
  make_conflict_index conflicting-rename-A-B-reversed
)

git init conflicting-rename-2
(cd conflicting-rename-2
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  mkdir a/sub && write_lines original 1 2 3 4 5 >a/sub/y.f
  touch a/w a/sub/z
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  write_lines 1 2 3 4 5 >a/sub/y.f
  git mv a/sub a/sub-renamed
  git commit -am "changed all content, renamed a/sub -> a/sub-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  write_lines original 1 2 3 4 5 6 >a/sub/y.f
  git mv a/sub a/sub-different
  git commit -am "changed all content, renamed a/sub -> a/sub-different"

# Here it's the same as above, i.e. Git doesn't list files as conflicting if
# they didn't change, even though they have a conflicting rename.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a/sub-different/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a/sub-different/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a/sub-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a/x.f
EOF
  make_conflict_index conflicting-rename-2-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a/sub-different/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a/sub-different/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a/sub-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 1	a/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a/x.f
EOF
  make_conflict_index conflicting-rename-2-A-B-reversed
)

git init conflicting-rename-complex
(cd conflicting-rename-complex
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  mkdir a/sub && write_lines original 1 2 3 4 5 >a/sub/y.f
  touch a/w a/sub/z
  git add . && git commit -m "original"

  git branch A
  git branch B
  git branch expected

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  write_lines 1 2 3 4 5 >a/sub/y.f
  git mv a a-renamed
  git commit -am "changed all content, renamed a -> a-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/sub/y.f
  git mv a/sub tmp
  git rm -r a
  git mv tmp a
  git commit -am "change something in subdirectory, then overwrite directory with subdirectory"

  git checkout expected
  rm .git/index
  rm -Rf ./a
  mkdir -p a-renamed/sub
  write_lines 1 2 3 4 5 6 >a-renamed/sub/y.f
  write_lines 1 2 3 4 5 >a-renamed/x.f
  write_lines 1 2 3 4 5 6 >a-renamed/y.f
  touch a-renamed/z a-renamed/w a-renamed/sub/z
  git add .
  git commit -m "Close to what Git has, but different due to rename tracking. Contents follow their true renames thanks to filename-aware matching, while Git also keeps rename/rename destinations that we compose."


  # Since the whole state is very different, the expected index is as well, but at least it should make sense for what it is.
  # Files pair up by matching filename like in Git, so `y.f` and `z` follow their true renames,
  # while `a/x.f` and `a/w` which `B` deleted conflict with `A`'s directory rename and keep our side.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-renamed/sub/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/y.f
100644 $(oid 8a1218a1024a212bb3db30becd860315f9f3ac52) 2	a-renamed/x.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 2	a-renamed/w
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a/y.f
EOF
  make_conflict_index conflicting-rename-complex-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 3	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-renamed/sub/z
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/y.f
100644 $(oid 8a1218a1024a212bb3db30becd860315f9f3ac52) 3	a-renamed/x.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 3	a-renamed/w
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/z
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 1	a/sub/y.f
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 2	a/y.f
EOF
  make_conflict_index conflicting-rename-complex-A-B-reversed
)

git init same-rename-different-mode
(cd same-rename-different-mode
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  touch a/w
  git add . && git commit -m "original"

  git branch A
  git branch B
  git branch expected

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  chmod +x a/x.f a/w
  git update-index --chmod=+x a/x.f a/w
  git mv a a-renamed
  git commit -am "changed a/xf, add +x everywhere, renamed a -> a-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  git mv a a-renamed
  git commit -am "changed all content, renamed a -> a-renamed"

  git checkout expected
  chmod +x a/x.f a/w
  git update-index --chmod=+x a/x.f a/w
  write_lines 1 2 3 4 5 6 >a/x.f
  git mv a a-renamed
  git commit -am "Git, when branches are reversed, doesn't keep the +x flag on a/w so we specify our own expectation"
  # Git sets +x and adds it as conflict, even though the merge is perfect, i.e. one side adds +x on top, perfectly additive.
  make_conflict_index same-rename-different-mode-A-B
  make_conflict_index same-rename-different-mode-A-B-reversed
)

git init remove-executable-mode
(cd remove-executable-mode
  touch w
  chmod +x w
  git add --chmod=+x w
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  chmod -x w
  git update-index --chmod=-x w
  git commit -am "remove executable bit from w"

  git checkout B
  write_lines 1 2 3 4 5  >w
  git commit -am "unrelated change to w"
)

git init renamed-symlink-with-conflict
(cd renamed-symlink-with-conflict
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  ln -s a/x.f link
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 >a/x.f
  git mv link link-renamed
  git commit -am "changed a/x.f, renamed link -> link-renamed"

  git checkout B
  write_lines original 1 2 3 4 5 6 >a/x.f
  git mv link link-different
  git commit -am "change content, renamed link -> link-different"
)

git init added-file-changed-content-and-mode
(cd added-file-changed-content-and-mode
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  git add . && git commit -m "original"

  git branch A
  git branch B
  git branch expected

  git checkout A
  write_lines 1 2 3 4 5 >new
  git add .
  git commit -m "add 'new' with content A"

  git checkout B
  write_lines original 1 2 3 4 5 6 >new
  chmod +x new
  git add --chmod=+x new
  git commit -m "add new with content B and +x"

  git checkout expected
  echo -n $'<<<<<<< A\n1\n2\n3\n4\n5\n=======\noriginal\n1\n2\n3\n4\n5\n6\n>>>>>>> B\n' >new
  chmod +x new
  git add --chmod=+x new
  git commit -m "Git has a better merge here, but that's due to better hunk handling/hunk splitting. We, however, consistently use +x"
)

git init type-change-and-renamed
(cd type-change-and-renamed
  mkdir a && >a/x.f
  ln -s a/x.f link
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  rm link && echo not-link > link
  git commit -am "link type-changed, file changed"

  git checkout B
  git mv link link-renamed
  git commit -am "just renamed the link"
)

git init change-and-delete
(cd change-and-delete
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  ln -s a/x.f link
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines 1 2 3 4 5 6 >a/x.f
  rm link && echo not-link > link
  git commit -am "link type-changed, file changed"

  git checkout B
  git rm link a/x.f
  git commit -am "delete everything"
)

git init submodule-both-modify
(cd submodule-both-modify
	mkdir sub
	(cd sub
	 git init
	 echo original > file
	 git add file
	 tick
	 git commit -m sub-root
	)
	git add sub
	tick
	git commit -m root

	git branch expected

	git checkout -b A main
	(cd sub
	 echo A > file
	 git add file
	 tick
	 git commit -m sub-a
	)
	git add sub
	tick
	git commit -m a
	git branch -f expected

	git checkout -b B main
	(cd sub
	 echo B > file
	 git add file
	 tick
	 git commit -m sub-b
	)
	git add sub
	tick
	git commit -m b
	git branch expected-reversed

	# The tree merger retains Git's conflict stages and provisional ours entry without inspecting submodule history.
	rm .git/index
	git update-index --index-info <<EOF
160000 $(oid e835c0c403c8e494c0ca98f3d25d0b8464c18d38) 1	sub
160000 $(oid 64466ebdff775ad618d9cc993cf52840e0af528c) 2	sub
160000 $(oid ea6eb701e03c2497915c25a851f3da8f8e362ca0) 3	sub
EOF
  make_conflict_index submodule-both-modify-A-B

	rm .git/index
	git update-index --index-info <<EOF
160000 $(oid e835c0c403c8e494c0ca98f3d25d0b8464c18d38) 1	sub
160000 $(oid ea6eb701e03c2497915c25a851f3da8f8e362ca0) 2	sub
160000 $(oid 64466ebdff775ad618d9cc993cf52840e0af528c) 3	sub
EOF
  make_conflict_index submodule-both-modify-A-B-reversed
)

git init gitlink-replaced-by-files
(cd gitlink-replaced-by-files
  git commit --allow-empty -m "seed commit"
  seed=$(git rev-parse HEAD)
  git update-index --add --cacheinfo 160000,$seed,item
  git commit -m "gitlink base"

  git branch A
  git branch B

  # Both sides replace the gitlink with regular files. Their contents must be
  # merged as an add/add pair with an empty blob ancestor; the commit named by
  # the base entry is not a valid blob-merge resource.
  git checkout A
  git rm item
  write_lines changed-by-A >item
  git add item
  git commit -m "replace gitlink with A's file"

  git checkout B
  git rm item
  write_lines changed-by-B >item
  git add item
  git commit -m "replace gitlink with B's file"
)

git init both-modify-union-attr
(cd both-modify-union-attr
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  echo "a/* merge=union" >.gitattributes
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  write_lines A 1 2 3 4 5 6 >a/x.f
  git commit -am "change file"

  git checkout B
  write_lines B 1 2 3 4 5 7 >a/x.f
  git commit -am "change file differently"
)

git init both-modify-binary
(cd both-modify-binary
  mkdir a && printf '\x00 binary' >a/x.f
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  printf '\x00 A' >a/x.f
  git commit -am "change binary file"

  git checkout B
  printf '\x00 B' >a/x.f
  git commit -am "change binary file differently"
)

git init both-modify-file-with-binary-attr
(cd both-modify-file-with-binary-attr
  mkdir a && echo 'not binary' >a/x.f
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  echo 'A binary' >a/x.f
  git commit -am "change pseudo-binary file"

  git checkout B
  echo 'B binary' >a/x.f
  git commit -am "change pseudo-binary file differently"
)

git init big-file-merge
(cd big-file-merge
  git config --local core.bigFileThreshold 100
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  seq 37 >a/x.f
  git commit -am "turn normal file into big one (102 bytes)"
  git branch expected

  git checkout B
  write_lines 1 2 3 4 5 6 >a/x.f
  git commit -am "a normal but conflicting file change"
)

git init no-merge-base
(cd no-merge-base
  git checkout -b A
  echo "A" >content && git add . && git commit -m "content A"

  git checkout --orphan B
  echo "B" >content && git add . && git commit -m "content B"

  git checkout -b expectation
)

git init multiple-merge-bases
(cd multiple-merge-bases
  write_lines 1 2 3 4 5 >content
  git add . && git commit -m "initial"

  git branch A
  git branch B

  git checkout A
  write_lines 0 1 2 3 4 5 >content
  git commit -am "change in A" && git tag A1

  git checkout B
  write_lines 1 2 3 4 5 6 >content
  git commit -am "change in B" && git tag B1

  git checkout A
  git merge B1

  git checkout B
  git merge A1

  git checkout A
  write_lines 0 1 2 3 4 5 A >content
  git commit -am "conflicting in A"

  git checkout B
  git rm content
  write_lines 0 2 3 4 5 six >renamed
  git commit -m "rename in B"
)

git init rename-and-modification
(cd rename-and-modification
  mkdir a && write_lines original 1 2 3 4 5 >a/x.f
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  git mv a/x.f x.f
  git commit -am "move a/x.f to the top-level"

  git checkout B
  write_lines 1 2 3 4 5 6 >a/x.f
  git commit -am "changed a/x.f"
)

git init symlink-modification
(cd symlink-modification
  touch a b o
  ln -s o link
  git add . && git commit -m "original"

  git branch A
  git branch B

  git checkout A
  rm link && ln -s a link
  git commit -am "set link to point to 'a'"

  git checkout B
  rm link && ln -s b link
  git commit -am "set link to point to 'b'"
)

git init symlink-addition
(cd symlink-addition
  touch a b
  git add . && git commit -m "original without symlink"

  git branch A
  git branch B

  git checkout A
  ln -s a link && git add .
  git commit -m "new link to point to 'a'"

  git checkout B
  ln -s b link && git add .
  git commit -m "new link to point to 'b'"
)

git init added-file-vs-added-directory
(cd added-file-vs-added-directory
  git commit --allow-empty -m "empty base"

  git branch A
  git branch B

  # A adds a file at `e`, while B adds a file below the directory `e`. Resolving
  # this tree/non-tree pair removes B's `e/e` path-tree leaf; its now-empty `e`
  # parent must not remain visible as a change during the inverse scheduling pass.
  git checkout A
  write_lines file >e
  git add e
  git commit -m "add file e"

  git checkout B
  mkdir e
  write_lines nested >e/e
  git add e/e
  git commit -m "add directory e"
)

git init added-symlink-blocks-gitlink-directory
(cd added-symlink-blocks-gitlink-directory
  git commit --allow-empty -m "empty base"
  base="$(git rev-parse HEAD)"

  git branch A
  git branch B

  # A adds a non-tree at `d`, while B adds a directory at the same path. The nested
  # gitlink is deliberate: tree/non-tree handling must not accidentally route this
  # structural conflict through the blob or submodule merge cases.
  git checkout A
  ln -s target d
  git add d
  git commit -m "add symlink d"

  git checkout B
  git update-index --index-info <<EOF
160000 commit $base	d/a/d
EOF
  git commit -m "add nested gitlink below d"
)

git init gitlink-vs-renamed-symlink-directory-with-siblings
(cd gitlink-vs-renamed-symlink-directory-with-siblings
  git commit --allow-empty -m "gitlink target"
  root="$(git rev-parse HEAD)"
  mkdir -p a/a
  ln -s target a/a/a
  git add a/a/a
  git update-index --add --cacheinfo 160000,$root,h
  git commit -m "symlink and gitlink base"

  git branch A
  git branch B

  # A replaces the symlink-containing directory with a regular child and keeps
  # the gitlink at `h`. B instead removes that gitlink, moves the symlink below
  # a new `h` directory, and adds a sibling below it. Resolving the resulting
  # type mismatch may consume the sibling's structural path before cleanup.
  git checkout A
  git rm a/a/a
  mkdir a
  write_lines changed >a/b
  git add a/b
  git commit -m "replace the symlink directory"

  git checkout B
  git mv a/a/a moved-link
  git update-index --force-remove h
  mkdir -p h/b
  git mv moved-link h/a
  write_lines sibling >h/b/a
  git add .
  git commit -m "replace the gitlink with a renamed symlink and sibling"

  # gix follows the detected `a` -> `h` directory rename and relocates A's
  # added `a/b` to `h/b~A`. Git leaves the addition at `a/b`; this fixture
  # records the existing semantic difference while guarding the path cleanup.
  git checkout -b expected B
  git update-index --add --cacheinfo "100644,$(git rev-parse A:a/b),h/b~A"
  git commit -m "expected gix merge"

  # FIXME: merge symmetry: only when B is ours, gix also preserves A's blocking
  # gitlink at its unique path.
  git checkout -b expected-reversed
  git update-index --add --cacheinfo "160000,$root,h~B"
  git commit -m "expected reversed gix merge"
)

git init same-source-rewrites-after-consumed-path
(cd same-source-rewrites-after-consumed-path
  mkdir -p a/a
  printf payload >a/a/a
  printf payload >b
  git add .
  git commit -m "identical files at nested and root paths"

  git branch A
  git branch B

  # Every non-directory entry deliberately has the same object ID. This gives
  # rewrite detection several equally valid source/destination pairings. A
  # removes `a/a/a` and reuses its payload at unrelated paths.
  git checkout A
  git rm a/a/a
  mkdir -p e/a g/e
  ln -s payload e/a/g
  printf payload >g/e/a
  git add .
  git commit -m "remove the nested source and add identical entries"

  # B retains `a/a/a`, adds an executable sibling, replaces `b` with a nested
  # copy, and adds another copy. Resolving one ambiguous rewrite can consume
  # the shared source before a later same-source rewrite cleans it up.
  git checkout B
  mkdir -p a/e h/e
  printf payload >a/e/a
  chmod +x a/e/a
  git rm b
  mkdir -p b/b
  printf payload >b/b/f
  printf payload >h/e/a
  git add .
  git update-index --chmod=+x a/e/a
  git commit -m "retain and multiply the identical payload"

  # gix pairs both sides with the same ambiguous base source and carries B's
  # executable mode to A's destination. Git leaves both additions in place.
  git checkout -b expected B
  git update-index --force-remove a/a/a
  git update-index --force-remove a/e/a
  git update-index --add --cacheinfo "120000,$(git rev-parse A:e/a/g),e/a/g"
  git update-index --add --cacheinfo "100755,$(git rev-parse B:a/e/a),g/e/a"
  git commit -m "expected gix merge"
)

git init rename-delete-after-consumed-path
(cd rename-delete-after-consumed-path
  mkdir -p a h/d f e/f
  write_lines shared >a/a
  chmod +x a/a
  write_lines four >b
  write_lines seven >e/f/a
  chmod +x e/f/a
  write_lines shared >f/a
  write_lines shared >h/d/a
  git add .
  git update-index --chmod=+x a/a e/f/a
  git commit -m "base with repeated rename candidates"

  git branch A
  git branch B

  # A moves `h/d/a` below `a`, turns `f/a` into `f`, and replaces `e/`.
  # The repeated `shared` payload deliberately gives rename detection several
  # possible sources, matching the scheduling ambiguity found by the fuzzer.
  git checkout A
  git rm -r a e
  mkdir -p a/a a/d
  write_lines four >a/a/a
  git mv h/d/a a/d/a
  mv f/a moved
  rmdir f
  mv moved f
  write_lines five >d
  write_lines shared >e
  git add -A
  git commit -m "rename repeated payloads and replace directories"

  # B deletes A's rename source and replaces `b`, `e`, and `f` with opposite
  # file/directory shapes. Resolving another rename/delete pair can consume the
  # path node for `h/d/a` before that pending deletion follows a directory rename.
  git checkout B
  git rm -r a b e f h
  mkdir -p b/a
  write_lines shared >b/a/a
  write_lines four >e
  write_lines four >f
  git add .
  git commit -m "delete rename sources and replace directories"

  # Ambiguous identity-only rename pairing makes gix keep a smaller tree than
  # Git and merge the repeated payloads at the surviving paths.
  shared_four=$(
    printf '%s\n' \
      '<<<<<<< A' \
      shared \
      ======= \
      four \
      '>>>>>>> B' |
      git hash-object -w --stdin
  )
  four_shared=$(
    printf '%s\n' \
      '<<<<<<< A' \
      four \
      ======= \
      shared \
      '>>>>>>> B' |
      git hash-object -w --stdin
  )
  git checkout -b expected B
  git read-tree --empty
  git update-index --add --cacheinfo "100644,$four_shared,b/a/a"
  git update-index --add --cacheinfo "100644,$(git rev-parse A:a/d/a),b/d/a"
  git update-index --add --cacheinfo "100644,$(git rev-parse A:d),d"
  git update-index --add --cacheinfo "100644,$shared_four,e"
  git update-index --add --cacheinfo "100644,$shared_four,f"
  git commit -m "expected gix merge"

  reversed_four_shared=$(
    printf '%s\n' \
      '<<<<<<< B' \
      four \
      ======= \
      shared \
      '>>>>>>> A' |
      git hash-object -w --stdin
  )
  reversed_shared_four=$(
    printf '%s\n' \
      '<<<<<<< B' \
      shared \
      ======= \
      four \
      '>>>>>>> A' |
      git hash-object -w --stdin
  )
  # Reversing the merge keeps the same paths and pairings, with directional
  # conflict-marker labels.
  git checkout -f -b expected-reversed B
  git read-tree --empty
  git update-index --add --cacheinfo "100644,$reversed_shared_four,b/a/a"
  git update-index --add --cacheinfo "100644,$(git rev-parse A:a/d/a),b/d/a"
  git update-index --add --cacheinfo "100644,$(git rev-parse A:d),d"
  git update-index --add --cacheinfo "100644,$reversed_four_shared,e"
  git update-index --add --cacheinfo "100644,$reversed_four_shared,f"
  git commit -m "expected reversed gix merge"
)

git init modified-file-vs-gitlink-directory
(cd modified-file-vs-gitlink-directory
  ln -s target a
  git add a
  git commit -m "symlink base"
  base="$(git rev-parse HEAD)"

  git branch A
  git branch B

  # A replaces the symlink with an executable file while B replaces it with a
  # directory. The nested gitlink makes its addition sort before the base-file
  # deletion, exercising merge scheduling independently of diff order.
  git checkout A
  rm a
  write_lines modified >a
  chmod +x a
  git add --chmod=+x a
  git commit -m "replace a with an executable"

  git checkout B
  git rm a
  git update-index --index-info <<EOF
160000 commit $base	a/h
EOF
  git commit -m "replace a with a gitlink directory"
)

git init relocated-addition-blocked-by-rename
(cd relocated-addition-blocked-by-rename
  mkdir -p c
  write_lines base >c/c
  git add .
  git commit -m "file in c"

  git branch A
  git branch B

  # A's exact file rename also implies the directory rename `c` -> `a`. B adds
  # `c/a/c`, so directory-rename handling relocates it to `a/a/c`, where A's
  # renamed file at `a/a` blocks the required directory.
  git checkout A
  mkdir -p a
  git mv c/c a/a
  rmdir c
  git commit -m "rename c/c to a/a"

  git checkout B
  mkdir -p c/a
  write_lines added >c/a/c
  git add .
  git commit -m "add below renamed directory"
)

git init nested-rename-blocks-relocated-addition
(cd nested-rename-blocks-relocated-addition
  mkdir a
  write_lines base >a/a
  git add .
  git commit -m "file in a"

  git branch A
  git branch B

  # Both sides rename the same file. A puts it where its containing directory
  # used to be, while B nests it another level below that directory.
  git checkout A
  git mv a/a moved
  rmdir a
  git mv moved a
  git commit -m "move a/a to a"

  git checkout B
  git mv a/a moved
  mkdir -p a/a
  git mv moved a/a/a
  git commit -m "move a/a to a/a/a"
)

git init directory-rename-vs-directory-to-file
(cd directory-rename-vs-directory-to-file
  mkdir a
  write_lines same >a/a
  git add .
  git commit -m "file in a"

  git branch A
  git branch B

  # A renames the containing directory. B moves its only file to the directory's
  # former path, replacing the directory with that file. This is a different-renames
  # conflict whose tree/non-tree handling must update the rename side's path tree.
  git checkout A
  git mv a e
  git commit -m "rename a to e"

  git checkout B
  git mv a/a moved
  rmdir a
  git mv moved a
  git commit -m "replace a directory with its file"
)

git init directory-rename-vs-renamed-file-replacement
(cd directory-rename-vs-renamed-file-replacement
  mkdir -p h/h
  write_lines nested >h/h/a
  write_lines outside >a
  git add .
  git commit -m "directory and outside file"

  git branch A
  git branch B

  # A moves the directory away. B deletes its contents and moves an unrelated
  # file onto the vacated directory path. Unlike the contained-file variant
  # above, the replacement rename has a distinct source, so it can meet the
  # structural directory rewrite before the nested rename/delete pair does.
  git checkout A
  git mv h/h f
  rmdir h
  git commit -m "rename the directory"

  git checkout B
  git rm h/h/a
  mkdir -p h
  git mv a h/h
  git commit -m "replace the directory with a renamed file"
)

git init unrelated-renames-overlapping-destinations
(cd unrelated-renames-overlapping-destinations
  mkdir -p a/a h/b
  write_lines first >a/a/a
  write_lines second >h/b/a
  git add .
  git commit -m "two files in separate directories"

  git branch A
  git branch B

  # A's directory rename places `h/b/a` below `c`. B independently renames
  # `a/a/a` to the non-tree `c` and renames `h/b/a` elsewhere. The two rename
  # destinations therefore overlap even though their source files are unrelated.
  git checkout A
  git mv h c
  git commit -m "rename h to c"

  git checkout B
  git mv h/b/a moved-h
  git mv a/a/a moved-a
  rmdir a/a
  mkdir -p a
  git mv moved-h a/a
  git mv moved-a c
  git commit -m "rename both files to crossing destinations"
)

git init renamed-file-inside-renamed-directory
(cd renamed-file-inside-renamed-directory
  mkdir -p a/a h/b
  write_lines first >a/a/a
  write_lines second >h/b/a
  git add .
  git commit -m "two files in separate directories"

  git branch A
  git branch B

  # A renames `h` to `c`. B replaces the contents of `h` with a file renamed
  # from elsewhere while moving the original `h/b/a` to `a/a`. Directory-rename
  # handling must therefore defer and relocate a rewrite, not just an addition.
  git checkout A
  git mv h c
  git commit -m "rename h to c"

  git checkout B
  git mv h/b/a moved-h
  git mv a/a/a moved-a
  rmdir a/a h/b h
  mkdir -p a h
  git mv moved-h a/a
  git mv moved-a h/h
  git commit -m "replace a renamed directory with an outside file"
)

git init unrelated-renames-to-same-path-with-type-mismatch
(cd unrelated-renames-to-same-path-with-type-mismatch
  write_lines payload >file-source
  ln -s payload link-source
  git add .
  git commit -m "regular file and symlink"

  git branch A
  git branch B

  # Both sides rename a different base entry to `target`. A's entry is a
  # symlink while B's is a regular file, so their destination cannot be
  # content-merged. Git keeps the symlink at `target` and relocates the regular
  # file to the side-qualified `target~B`, independently of merge direction.
  git checkout A
  git mv link-source target
  git commit -m "rename the symlink to target"

  git checkout B
  git mv file-source target
  git commit -m "rename the regular file to target"
)

git init renamed-file-vs-file-to-directory-with-siblings
(cd renamed-file-vs-file-to-directory-with-siblings
  write_lines base >a
  git add a
  git commit -m "base file"

  git branch A
  git branch B

  # A renames the base file away while B replaces its old path with a directory
  # containing two children. Both children encounter the same rename/delete
  # conflict; handling the first must not make the second try to remove an
  # already-pruned rename-destination node.
  git checkout A
  git mv a h
  git commit -m "rename the file"

  git checkout B
  git rm a
  mkdir a
  write_lines first >a/a
  write_lines second >a/c
  git add .
  git commit -m "replace the file with two children"
)

git init identical-additions-with-different-relations
(cd identical-additions-with-different-relations
  # A's earlier directory deletion shifts the relation ID assigned to the shared
  # addition. The relation is diff-local, so both additions still have one effect.
  mkdir -p a-shift
  echo shift >a-shift/file
  git add . && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git rm -r a-shift
  mkdir -p b-target
  echo shared-addition >b-target/file
  git add . && git commit -m "delete a directory and add the shared tree"

  git checkout B
  mkdir -p b-target
  echo shared-addition >b-target/file
  git add . && git commit -m "add the shared tree"
)

git init identical-deletions-with-different-relations
(cd identical-deletions-with-different-relations
  # A deletes an earlier directory as well, so the shared deletion receives a
  # different relation ID on each side despite removing the same tree.
  mkdir -p a-shift b-target
  echo shift >a-shift/file
  echo shared-deletion >b-target/file
  git add . && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git rm -r a-shift b-target
  git commit -m "delete the earlier and shared trees"

  git checkout B
  git rm -r b-target
  git commit -m "delete the shared tree"
)

git init identical-rewrites-with-different-relations
(cd identical-rewrites-with-different-relations
  # Moving the same file into a new directory gives each rewrite a destination
  # relation. A's earlier deletion shifts that ID without changing the rename.
  mkdir -p a-shift
  echo shift >a-shift/file
  echo shared-rewrite >b-source
  git add . && git commit -m "base"

  git branch A
  git branch B

  git checkout A
  git rm -r a-shift
  mkdir c-target
  git mv b-source c-target/file
  mkdir b-source
  echo from-A >b-source/A-only
  git add . && git commit -m "delete an earlier tree and rename the shared file"

  git checkout B
  mkdir c-target
  git mv b-source c-target/file
  mkdir b-source
  echo from-B >b-source/B-only
  git add . && git commit -m "rename the shared file"
)

git init type-change-to-symlink
(cd type-change-to-symlink
  touch a b link
  git add . && git commit -m "original without symlink"

  git branch A
  git branch B

  git checkout A
  git rm link
  ln -s a link && git add .
  git commit -m "new link to point to 'a'"

  git checkout B
  git rm link
  ln -s b link && git add .
  git commit -m "new link to point to 'b'"
)



baseline non-tree-to-tree A-B A B
baseline deleted-file-added-dir A-B A B
baseline deleted-file-added-gitlink-directory A-B A B
baseline tree-to-non-tree A-B A B
baseline tree-to-non-tree-with-rename A-B A B
baseline non-tree-to-tree-with-rename A-B A B
baseline rename-add-same-symlink A-B A B
baseline rename-add-exe-bit-conflict A-B A B
baseline remove-executable-mode A-B A B
baseline simple side-1-3-without-conflict side1 side3
baseline simple fast-forward side1 main
baseline simple no-change main main
baseline simple side-1-3-without-conflict-diff3 side1 side3
baseline simple side-1-2-various-conflicts side1 side2
baseline simple side-1-2-various-conflicts-diff3 side1 side2
baseline simple single-content-conflict side1 side4
baseline simple single-content-conflict-diff3 side1 side4
baseline simple tweak1-side2 tweak1 side2
baseline simple tweak1-side2-diff3 tweak1 side2
baseline simple side-1-unrelated side1 unrelated
baseline simple side-1-unrelated-diff3 side1 unrelated
baseline rename-delete A-B A B
baseline rename-delete A-similar A A
baseline rename-delete B-similar B B
baseline rename-change-matrix A-B A B
baseline same-rename-with-content A-B A B
baseline same-rename-and-file-to-directory A-B A B
baseline renames-to-same-destination A-B A B
baseline identical-renames-to-same-destination A-B A B
baseline identical-renames-to-same-destination-with-mode-change A-B A B "gix resolves the identical content and mode change cleanly, while Git leaves an add/add mode conflict"
baseline deleted-file-added-dir-with-rename A-B A B
baseline rename-add A-B A B
baseline rename-add A-B-diff3 A B
baseline rename-add-symlink A-B A B
baseline rename-add-symlink A-B-diff3 A B
baseline rename-rename-plus-content A-B A B
baseline rename-rename-plus-content A-B-diff3 A B
baseline rename-add-delete A-B A B
baseline rename-rename-delete-delete A-B A B
baseline super-1 A-B A B
baseline super-1 A-B-diff3 A B
baseline super-2 A-B A B
baseline super-2 A-B-diff3 A B

baseline rename-within-rename A-B-deviates A B "Git doesn't detect the rename-nesting, we do neither"
baseline rename-within-rename-2 A-B-deviates A B "Git keeps the doubly-renamed file in both rename destinations as it doesn't compose nested directory renames - we do, and merge contents cleanly in one place, in both merge directions"
baseline conflicting-rename A-B A B
baseline conflicting-rename-2 A-B A B
baseline conflicting-rename-complex A-B A B "Git has different rename tracking - overall result it's still close enough"

baseline same-rename-different-mode A-B A B "Git works for the A/B case, but for B/A it forgets to set the executable bit"
baseline renamed-symlink-with-conflict A-B A B
baseline added-file-changed-content-and-mode A-B A B "We improve on executable bit handling, but loose on diff quality as we are definitely missing some tweaks"

baseline type-change-and-renamed A-B A B
baseline change-and-delete A-B A B
baseline submodule-both-modify A-B A B "The tree-only merge cannot inspect submodule reachability, so it keeps ours as the provisional entry and reports SubmoduleMerge."
baseline gitlink-replaced-by-files A-B A B
baseline both-modify-union-attr A-B A B
baseline both-modify-union-attr A-B-diff3 A B
baseline both-modify-binary A-B A B
baseline both-modify-binary A-B A B
baseline both-modify-file-with-binary-attr A-B A B
baseline big-file-merge A-B A B "Git actually ignores core.bigFileThreshold during merging and tries a normal merge (or binary one) anyway. We don't ignore it and treat big files like binary files" \
                                no-reverse
baseline no-merge-base A-B A B
baseline no-merge-base A-B-diff3 A B

baseline multiple-merge-bases A-B A B
baseline multiple-merge-bases A-B-diff3 A B

baseline rename-and-modification A-B A B
baseline symlink-modification A-B A B
baseline symlink-addition A-B A B
baseline added-file-vs-added-directory A-B A B
baseline added-symlink-blocks-gitlink-directory A-B A B
baseline gitlink-vs-renamed-symlink-directory-with-siblings A-B A B "gix relocates A's addition through the detected directory rename, while Git keeps it at its original path; FIXME: merge symmetry: reversing gix also preserves A's blocking gitlink"
baseline same-source-rewrites-after-consumed-path A-B A B "ambiguous identical blobs make gix pair both sides with one base source, while Git retains both additions"
baseline rename-delete-after-consumed-path A-B A B "ambiguous identical blobs make gix pair rename sources differently than Git"
baseline modified-file-vs-gitlink-directory A-B A B
baseline relocated-addition-blocked-by-rename A-B A B
baseline nested-rename-blocks-relocated-addition A-B A B
baseline directory-rename-vs-directory-to-file A-B A B
baseline directory-rename-vs-renamed-file-replacement A-B A B
baseline unrelated-renames-overlapping-destinations A-B A B
baseline renamed-file-inside-renamed-directory A-B A B
baseline unrelated-renames-to-same-path-with-type-mismatch A-B A B
baseline renamed-file-vs-file-to-directory-with-siblings A-B A B
baseline identical-additions-with-different-relations A-B A B
baseline identical-deletions-with-different-relations A-B A B
baseline identical-rewrites-with-different-relations A-B A B
baseline type-change-to-symlink A-B A B

##
## Only once the tree-merges were performed can we refer to their objects
## when making tree-conflict resolution expectations. It's important
## to get these right.
##
(cd added-file-vs-added-directory
  # The ancestor is empty, so choosing it keeps neither addition.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps precisely the side selected by the merge direction.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd deleted-file-added-gitlink-directory
  # Both operations are compatible, so forced conflict resolution changes nothing:
  # the shared deletion applies and B's directory remains in both directions.
  git read-tree B
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A
  make_resolve_tree ours A B
  make_resolve_tree ours B A
)

(cd added-symlink-blocks-gitlink-directory
  # The ancestor is empty, so choosing it keeps neither addition in either direction.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps precisely the side named first: either A's symlink or B's
  # directory containing the nested gitlink.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd modified-file-vs-gitlink-directory
  # Ancestor resolution restores the original symlink in both directions.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps precisely the selected replacement.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd relocated-addition-blocked-by-rename
  # The explicit file rename prevents the inferred directory rename from relocating
  # B's addition through a non-tree. The changes are therefore compatible, and forced
  # conflict resolution has nothing to discard in either direction.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  git read-tree "$merged_tree_id"
  make_resolve_tree ancestor A B
  make_resolve_tree ours A B

  IFS= read -r -d '' merged_reversed_tree_id <A-B-reversed.merge-info
  git read-tree "$merged_reversed_tree_id"
  make_resolve_tree ancestor B A
  make_resolve_tree ours B A
)

(cd nested-rename-blocks-relocated-addition
  # gix keeps the rename/rename stages in addition to the file/directory stages
  # that Git reports. The resulting tree and retained side content are identical.
  blob=$(git rev-parse main:a/a)
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $blob 2	a
100644 blob $blob 1	a/a
100644 blob $blob 3	a/a/a
100644 blob $blob 2	a~A
EOF
  make_conflict_index nested-rename-blocks-relocated-addition-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $blob 3	a
100644 blob $blob 1	a/a
100644 blob $blob 2	a/a/a
100644 blob $blob 3	a~A
EOF
  make_conflict_index nested-rename-blocks-relocated-addition-A-B-reversed
)

(cd directory-rename-vs-directory-to-file
  # Ancestor resolution applies neither rename and restores `a/a`.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps precisely the directory rename or directory-to-file
  # replacement selected by the merge direction.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd directory-rename-vs-renamed-file-replacement
  # Git retains the original file as stage 1 at its rename destination. Like the
  # other rename/delete cases, gix records only the side that kept the file.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  rm .git/index
  git read-tree "$merged_tree_id"
  git update-index --force-remove f/a
  git update-index --index-info <<EOF
100644 blob $(git rev-parse A:f/a) 2	f/a
EOF
  make_conflict_index directory-rename-vs-renamed-file-replacement-A-B

  rm .git/index
  git read-tree "$merged_tree_id"
  git update-index --force-remove f/a
  git update-index --index-info <<EOF
100644 blob $(git rev-parse A:f/a) 3	f/a
EOF
  make_conflict_index directory-rename-vs-renamed-file-replacement-A-B-reversed

  # Ancestor resolution rejects both conflicting renames and restores the original
  # directory and outside file.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps precisely the directory rename or file replacement selected
  # by the merge direction.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd unrelated-renames-overlapping-destinations
  # Ancestor rejects the conflicting rename destinations and keeps both base files.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours keeps the complete side selected by the merge direction.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd renamed-file-inside-renamed-directory
  # Git puts the relocated `c/h` into stages 1, 2, and 3 even though all three
  # entries are identical. gix records the successful directory relocation as
  # stage 0 and reserves conflict stages for the genuinely divergent rename.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:h/b/a) 3	a/a
100644 blob $(git rev-parse main:h/b/a) 2	c/b/a
100644 blob $(git rev-parse main:a/a/a)	c/h
100644 blob $(git rev-parse main:h/b/a) 1	h/b/a
EOF
  make_conflict_index renamed-file-inside-renamed-directory-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:h/b/a) 2	a/a
100644 blob $(git rev-parse main:h/b/a) 3	c/b/a
100644 blob $(git rev-parse main:a/a/a)	c/h
100644 blob $(git rev-parse main:h/b/a) 1	h/b/a
EOF
  make_conflict_index renamed-file-inside-renamed-directory-A-B-reversed

  # Ancestor rejects the divergent rename of `h/b/a`, but the separate file
  # rename is cleanly relocated through A's directory rename to `c/h`.
  git read-tree main
  git update-index --force-remove a/a/a
  git update-index --add --cacheinfo "100644,$(git rev-parse main:a/a/a),c/h"
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing A still retains the independently relocated file from B.
  git read-tree A
  git update-index --force-remove a/a/a
  git update-index --add --cacheinfo "100644,$(git rev-parse main:a/a/a),c/h"
  make_resolve_tree ours A B

  # Choosing B keeps B's divergent rename, while the other file still follows
  # the independently resolved directory relocation.
  git read-tree B
  git update-index --force-remove h/h
  git update-index --add --cacheinfo "100644,$(git rev-parse main:a/a/a),c/h"
  make_resolve_tree ours B A
)

(cd unrelated-renames-to-same-path-with-type-mismatch
  # Ancestor resolution rejects both incompatible destination renames.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing ours applies only the rename from the side named first.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd renamed-file-vs-file-to-directory-with-siblings
  # Git retains the base file as stage 1 at its rename destination. As in the
  # other rename/delete cases, gix records only the side that kept the file.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  rm .git/index
  git read-tree "$merged_tree_id"
  git update-index --force-remove h
  git update-index --index-info <<EOF
100644 blob $(git rev-parse A:h) 2	h
EOF
  make_conflict_index renamed-file-vs-file-to-directory-with-siblings-A-B

  rm .git/index
  git read-tree "$merged_tree_id"
  git update-index --force-remove h
  git update-index --index-info <<EOF
100644 blob $(git rev-parse A:h) 3	h
EOF
  make_conflict_index renamed-file-vs-file-to-directory-with-siblings-A-B-reversed

  # Ancestor resolution rejects the rename and the file-to-directory
  # replacement, restoring the base file.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Choosing A keeps its rename while still accepting B's non-conflicting
  # children at the now-vacant source path.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  git read-tree "$merged_tree_id"
  make_resolve_tree ours A B

  # Choosing B rejects A's rename and keeps B's replacement directory.
  git read-tree B
  make_resolve_tree ours B A
)

(cd gitlink-vs-renamed-symlink-directory-with-siblings
  # Git retains the renamed symlink's base stage and leaves `a/b` unconflicted.
  # gix instead records only the surviving rename side and the relocated
  # addition as the two sides of its file/directory conflict.
  rm .git/index
  git update-index --index-info <<EOF
120000 blob $(git rev-parse B:h/a) 3	h/a
100644 blob $(git rev-parse B:h/b/a)	h/b/a
100644 blob $(git rev-parse A:a/b) 2	h/b~A
EOF
  make_conflict_index gitlink-vs-renamed-symlink-directory-with-siblings-A-B

  rm .git/index
  git update-index --index-info <<EOF
120000 blob $(git rev-parse B:h/a) 2	h/a
100644 blob $(git rev-parse B:h/b/a)	h/b/a
100644 blob $(git rev-parse A:a/b) 3	h/b~A
160000 commit $(git rev-parse main:h) 2	h~B
EOF
  make_conflict_index gitlink-vs-renamed-symlink-directory-with-siblings-A-B-reversed

  # Ancestor keeps the renamed symlink at its base path, while the independent
  # sibling below B's replacement directory remains applicable.
  git read-tree main
  git update-index --force-remove h
  git update-index --add --cacheinfo "100644,$(git rev-parse B:h/b/a),h/b/a"
  make_resolve_tree ancestor A B
  git read-tree main
  git update-index --force-remove h
  git update-index --add --cacheinfo "100644,$(git rev-parse B:h/b/a),h/b/a"
  make_resolve_tree ancestor B A

  # With A as ours, its addition follows the detected directory rename while
  # the conflicting gitlink and B's sibling are rejected.
  git read-tree --empty
  git update-index --add --cacheinfo "100644,$(git rev-parse A:a/b),h/b"
  make_resolve_tree ours A B

  # With B as ours, retain B's replacement directory.
  git read-tree B
  make_resolve_tree ours B A
)

(cd rename-delete-after-consumed-path
  # Git associates the repeated blobs with different rename sources. Record
  # gix's structured conflicts explicitly; the worktree tree is documented by
  # the `expected` branches above.
  rm .git/index
  git update-index --index-info <<EOF
100755 blob $(git rev-parse main:a/a) 1	a/a
100644 blob $(git rev-parse A:a/d/a) 2	a/d/a
100644 blob $(git rev-parse A:a/a/a) 2	b/a/a
100644 blob $(git rev-parse B:b/a/a) 3	b/a/a
100644 blob $(git rev-parse A:a/d/a)	b/d/a
100644 blob $(git rev-parse A:d)	d
100644 blob $(git rev-parse A:e) 2	e
100644 blob $(git rev-parse B:e) 3	e
100644 blob $(git rev-parse A:f) 2	f
100644 blob $(git rev-parse B:f) 3	f
EOF
  make_conflict_index rename-delete-after-consumed-path-A-B

  rm .git/index
  git update-index --index-info <<EOF
100755 blob $(git rev-parse main:a/a) 1	a/a
100644 blob $(git rev-parse A:a/d/a) 3	a/d/a
100644 blob $(git rev-parse B:b/a/a) 2	b/a/a
100644 blob $(git rev-parse A:a/a/a) 3	b/a/a
100644 blob $(git rev-parse A:a/d/a)	b/d/a
100644 blob $(git rev-parse A:d)	d
100644 blob $(git rev-parse B:e) 2	e
100644 blob $(git rev-parse A:e) 3	e
100644 blob $(git rev-parse B:f) 2	f
100644 blob $(git rev-parse A:f) 3	f
EOF
  make_conflict_index rename-delete-after-consumed-path-A-B-reversed
)

(cd simple
  rm .git/index
  # 'whatever' is tree-conflict, 'greeting' is content conflict with markers
  git update-index --index-info <<EOF
100644 $(oid 45b983be36b73c0788dc9cbcb76cbb80fc7bb057) 0	greeting
100644 $(oid 09c277aa66897c58157f57a374eacc63a407dcab) 0	numbers
100644 $(oid 5716ca5987cbf97d6bb54920bea6adde242d87e6) 0	whatever
EOF
  make_resolve_tree ours side1 side2

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 092bfb9bdf74dd8cfd22e812151281ee9aa6f01a) 0	greeting
100644 $(oid 09c277aa66897c58157f57a374eacc63a407dcab) 0	numbers
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	whatever/empty
EOF
  make_resolve_tree ours side2 side1

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 9dc97bdc2426e68423360e3e5299280b2cf6b8ff) 0	greeting
100644 $(oid 09c277aa66897c58157f57a374eacc63a407dcab) 0	numbers
100644 $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99) 0	whatever
EOF
  make_resolve_tree ancestor side1 side2

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 1a2664a9924754c698e323f756f9f87f3f2fb337) 0	greeting
100644 $(oid 09c277aa66897c58157f57a374eacc63a407dcab) 0	numbers
100644 $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99) 0	whatever
EOF
  make_resolve_tree ancestor side2 side1

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid a4ae6e4709228b5da6001cb9d1cfa7736851e2a6) 0	greeting
100644 $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99) 0	whatever
100644 $(oid 542802a799ded74fa01c47ba2f8925e284a369e2) 0	Αυτά μου φαίνονται κινέζικα
EOF
  make_resolve_tree ancestor tweak1 side2

  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid 45b983be36b73c0788dc9cbcb76cbb80fc7bb057) 0	greeting
100644 $(oid 5716ca5987cbf97d6bb54920bea6adde242d87e6) 0	whatever
100644 $(oid 65bc6a1e238f4bf05b28fd05240636e2cfb657e0) 0	Αυτά μου φαίνονται κινέζικα
EOF
  make_resolve_tree ours tweak1 side2

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid ea28dcd7f627a2a7bbd09daa679c452180617c9f)	greeting
100644 blob $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99)	whatever
100644 blob $(oid f801a62deed900f8a80ff35e3339474ad6352a93)	Αυτά μου φαίνονται κινέζικα
EOF
  make_resolve_tree ancestor side2 tweak1

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid 092bfb9bdf74dd8cfd22e812151281ee9aa6f01a)	greeting
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	whatever/empty
100644 blob $(oid 09c277aa66897c58157f57a374eacc63a407dcab)	Αυτά μου φαίνονται κινέζικα
EOF
  make_resolve_tree ours side2 tweak1
)

(cd rename-add-symlink
  rm .git/index
  # the symlink of 'bar' from A
  git update-index --index-info <<EOF
120000 blob $(oid 19102815663d23f8b75a47e7a01965dcdc96468c)	bar
EOF
  make_resolve_tree ours A B

  rm .git/index
  # the merged form of 'bar' from B, not replaced by symlink
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	bar
EOF
  make_resolve_tree ours B A

  rm .git/index
  # foo is renamed to bar, type clash means neither A nor B can be added - empty tree
  # It is not able to 'get foo back', it can't track that currently.
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A
)

(cd deleted-file-added-dir-with-rename
  rm .git/index
  # Unlike Git, we don't retain the base stage at the rename destination.
  git update-index --index-info <<EOF
100644 blob $(oid df967b96a579e45a18b8251732d16804b2e56a55) 3	renamed
100644 blob $(oid d5f7fc3f74f7dec08280f370a975b112e8f60818)	x/a
EOF
  make_conflict_index deleted-file-added-dir-with-rename-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid df967b96a579e45a18b8251732d16804b2e56a55) 2	renamed
100644 blob $(oid d5f7fc3f74f7dec08280f370a975b112e8f60818)	x/a
EOF
  make_conflict_index deleted-file-added-dir-with-rename-A-B-reversed

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid df967b96a579e45a18b8251732d16804b2e56a55)	x
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A
)

(cd same-rename-and-file-to-directory
  # Forced ours keeps the agreed rename and resolves the added child's content in favor
  # of the current side. Use each complete side tree as the directional expectation.
  git read-tree A
  make_resolve_tree ours A B

  git read-tree B
  make_resolve_tree ours B A
)

(cd rename-change-matrix
  # `different-renames` needs Git's content-conflicted blob at both rename destinations.
  # The first NUL-delimited field is the merged tree written by `git merge-tree`.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  different_merged_id=$(git rev-parse "$merged_tree_id:different-renames/ours")
  IFS= read -r -d '' merged_reversed_tree_id <A-B-reversed.merge-info
  different_merged_reversed_id=$(git rev-parse "$merged_reversed_tree_id:different-renames/theirs")

  # Expected A/B index, grouped by the matrix above. It differs from Git only in two places,
  # neither of which alters the merged tree:
  #
  # * delete-source: only A's renamed side remains. Git also places the source's base blob at
  #   stage 1 of the renamed path; gix doesn't invent that path for the base entry.
  # * directory-old: gix considers the added child relocated by the directory rename and records
  #   it at stage 0. Git reports a "file location" conflict and records the relocated child at
  #   stage 3, even though its diagnostic also suggests the same destination.
  #
  # The other matrix entries match Git's index:
  # * add-destination: A's rename and B's addition occupy stages 2 and 3 at `target`.
  # * delete-destination: the old target is stage 1 and A's replacement is stage 2.
  # * different-renames: retain the base source and put the merged conflict blob at both destinations.
  # * modify-destination: the old destination and both replacements occupy stages 1, 2, and 3.
  # * modify-source: the edit follows the rename and resolves cleanly at stage 0.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:.gitattributes)	.gitattributes
100644 blob $(git rev-parse A:add-destination/target) 2	add-destination/target
100644 blob $(git rev-parse B:add-destination/target) 3	add-destination/target
100644 blob $(git rev-parse main:delete-destination/target) 1	delete-destination/target
100644 blob $(git rev-parse A:delete-destination/target) 2	delete-destination/target
100644 blob $(git rev-parse A:delete-source/renamed) 2	delete-source/renamed
100644 blob $(git rev-parse main:different-renames/file) 1	different-renames/file
100644 blob $different_merged_id 2	different-renames/ours
100644 blob $different_merged_id 3	different-renames/theirs
100644 blob $(git rev-parse B:directory-old/added)	directory-renamed/added
100644 blob $(git rev-parse A:directory-renamed/file)	directory-renamed/file
100644 blob $(git rev-parse main:modify-destination/target) 1	modify-destination/target
100644 blob $(git rev-parse A:modify-destination/target) 2	modify-destination/target
100644 blob $(git rev-parse B:modify-destination/target) 3	modify-destination/target
100644 blob $(git rev-parse B:modify-source/file)	modify-source/renamed
EOF
  make_conflict_index rename-change-matrix-A-B

  # The reversed index has the same shape, with stages 2 and 3 exchanged and directional
  # content-conflict labels reversed.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:.gitattributes)	.gitattributes
100644 blob $(git rev-parse B:add-destination/target) 2	add-destination/target
100644 blob $(git rev-parse A:add-destination/target) 3	add-destination/target
100644 blob $(git rev-parse main:delete-destination/target) 1	delete-destination/target
100644 blob $(git rev-parse A:delete-destination/target) 3	delete-destination/target
100644 blob $(git rev-parse A:delete-source/renamed) 3	delete-source/renamed
100644 blob $(git rev-parse main:different-renames/file) 1	different-renames/file
100644 blob $different_merged_reversed_id 3	different-renames/ours
100644 blob $different_merged_reversed_id 2	different-renames/theirs
100644 blob $(git rev-parse B:directory-old/added)	directory-renamed/added
100644 blob $(git rev-parse A:directory-renamed/file)	directory-renamed/file
100644 blob $(git rev-parse main:modify-destination/target) 1	modify-destination/target
100644 blob $(git rev-parse B:modify-destination/target) 2	modify-destination/target
100644 blob $(git rev-parse A:modify-destination/target) 3	modify-destination/target
100644 blob $(git rev-parse B:modify-source/file)	modify-source/renamed
EOF
  make_conflict_index rename-change-matrix-A-B-reversed
)

(cd same-rename-with-content
  # The merged tree matches Git, but the conflict index differs. Git puts the base and the
  # original A and B blobs at stages 1, 2, and 3 of `target`. gix instead keeps the base at
  # its original path `file` and puts the already-merged conflict blob into both destination
  # stages. The structured conflict still retains the original side entries.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:file) 1	file
100644 blob $(git rev-parse expected:target) 2	target
100644 blob $(git rev-parse expected:target) 3	target
EOF
  make_conflict_index same-rename-with-content-A-B

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:file) 1	file
100644 blob $(git rev-parse expected-reversed:target) 2	target
100644 blob $(git rev-parse expected-reversed:target) 3	target
EOF
  make_conflict_index same-rename-with-content-A-B-reversed
)

(cd renames-to-same-destination
  # Git leaves this unresolved as an add/add at `target`, with A at stage 2 and B at stage 3
  # and neither original path in the index. The following trees instead record gix's explicit
  # conflict-resolution modes.
  #
  # Ancestor resolution applies neither rename, so both original files remain in both directions.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:one)	one
100644 blob $(git rev-parse main:two)	two
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # "Ours" for A/B applies A's `one` -> `target` and leaves B's source `two` untouched,
  # unlike Git's unresolved index described above.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse A:target)	target
100644 blob $(git rev-parse main:two)	two
EOF
  make_resolve_tree ours A B

  # Reversing the merge makes B ours: apply `two` -> `target` and leave `one` untouched,
  # again replacing Git's unresolved add/add with a resolved tree.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(git rev-parse main:one)	one
100644 blob $(git rev-parse B:target)	target
EOF
  make_resolve_tree ours B A
)

(cd identical-renames-to-same-destination
  # Identical entries make the two renames compatible rather than a tree
  # conflict, so conflict-resolution policy must not alter the clean result.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  git read-tree "$merged_tree_id"
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A
  make_resolve_tree ours A B
  make_resolve_tree ours B A
)

(cd identical-renames-to-same-destination-with-mode-change
  # Git leaves the two destination modes at stages 2 and 3. gix resolves the
  # identical content and the one-sided executable change into a stage-0 entry.
  git read-tree expected
  make_conflict_index identical-renames-to-same-destination-with-mode-change-A-B
  make_conflict_index identical-renames-to-same-destination-with-mode-change-A-B-reversed

  # Ancestor rejects both colliding renames.
  git read-tree main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Ours applies the selected side's rename and retains the other source.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd rename-rename-plus-content
  rm .git/index
  # both sides rename 'foo' into something else.
  git update-index --index-info <<EOF
100644 blob $(oid 8a1218a1024a212bb3db30becd860315f9f3ac52)	foo
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # 'bar' is the name in 'A', and there is a merge with the content from 'B'
  # which we auto-resolve.
  git update-index --index-info <<EOF
100644 blob $(oid d0549c3d3c96a464289f3b820b7d96aedc58924b)	bar
EOF
  make_resolve_tree ours A B

  rm .git/index
  # 'baz' is the name in 'B', and there is a merge with the content from 'A'
  # which we auto-resolve.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	baz
EOF
  make_resolve_tree ours B A
)

(cd rename-add-delete
  rm .git/index
  # 'foo' is deleted, 'bar' is added in 'A', but renamed to 'bar' in 'B'.
  # Do nothing, *should* keep 'foo', and it's re-added later, which copies it.
  # But instead we keep foo the first time, but then it 'turns' and we process a remaining
  # addition that once again sees the rename from the other side, which is not a conflict
  # and thus removes 'foo' after all, and merges 'bar'.
  # It's not super-correct, but it's only an issue for virtual merge bases, which are kind of
  # hidden anyway.
  git update-index --index-info <<EOF
100644 blob $(oid 0cde534c2fca6c92c07b3e7a696665e844b9b933)	bar
EOF
  make_resolve_tree ancestor A B
  # B A isn't tested, as it is not an Err() conflict.

  # This case ends up being exactly the same as the ancestor, but with the merge-conflict
  # auto-resolved to 'ours'.
  # This time, it's expected to not have 'foo', but 'ours' in the clashing pair is a deletion. The rename side
  # is dropped, but what's left is the rename/add pair once the algorithm turns around/flips.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid f286e5cdd97ac6895438ea4548638bb98ac9bd6b)	bar
EOF
  make_resolve_tree ours A B
)

(cd rename-rename-delete-delete
  rm .git/index
  # 'A' deletes 'bar' and 'B' turns 'bar' into 'baz'. 'A' renames 'foo' into 'bar', and
  # 'B' deletes 'foo'. 'ancestor' resolves to avoid any edits, leaving the state from 'main'.
  git update-index --index-info <<EOF
100644 blob $(oid 5716ca5987cbf97d6bb54920bea6adde242d87e6)	bar
100644 blob $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99)	foo
EOF
  make_resolve_tree ancestor A B
  # this works in reverse as well (this time).
  make_resolve_tree ancestor B A

  rm .git/index
  # As 'ours' is a deletion of 'foo', it goes through, but we also acknowledge 'theirs'
  # as it gives better results, so end up with `baz`.
  git update-index --index-info <<EOF
100644 blob $(oid 257cc5642cb1a054f08cc83f2d943e56fd3ebe99)	baz
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Here we end up in exactly the same spot as if we'd do a normal merge,
  # which ends `baz` in a conflict. However, with content-merges set to 'ours'
  # it ends up like it should, giving a good result.
  git update-index --index-info <<EOF
  100644 blob $(oid 5716ca5987cbf97d6bb54920bea6adde242d87e6)	baz
EOF
  make_resolve_tree ours B A
)

(cd super-1
  # Each of the ancestor files are renamed in a conflicting way, and here
  # with ancestor choice, nothing happens, making this equivalent to `main`
  git checkout main
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # We do indeed perform the renames like this, and the content is merges as well as possible,
  # (here) configured to content-merge with 'ours' as well where needed.
  git update-index --index-info <<EOF
100644 blob $(oid 4b5599c7c2ed4390417d9699bec86144a386873d)	four
100644 blob $(oid 64012489f118cb4011c8902b4a635f70dcb0c0ca)	six
100644 blob $(oid e33f5e94470d3b5fa0220ff6a9cabb78a3f72fa3)	two
EOF
  make_resolve_tree ours A B

  rm .git/index
  # The same, but from the other side.
  git update-index --index-info <<EOF
  100644 blob $(oid 64012489f118cb4011c8902b4a635f70dcb0c0ca)	four
  100644 blob $(oid e33f5e94470d3b5fa0220ff6a9cabb78a3f72fa3)	six
  100644 blob $(oid 4178ea6795c4c3e07b4e17e6a04aa49584b07ecd)	two
EOF
  make_resolve_tree ours B A
)

(cd super-2
  rm .git/index
  # 'B' changes foo, and moves it into 'olddir/bar', but `A' deleted 'foo', and adds 'newdir/bar/file'
  # after renaming 'olddir' to 'newdir'.
  # As `B` only has a single change that gets dropped when it clashes with the deletion of 'foo',
  # all other changes of 'A' can just be applied without any conflict whatsoever.
  git update-index --index-info <<EOF
100644 blob $(oid 8a1218a1024a212bb3db30becd860315f9f3ac52)	foo
100644 blob $(oid 78981922613b2afb6025042ff6bd878ac1994e85)	newdir/a
100644 blob $(oid 61780798228d17af2d34fce4cfbdf35556832472)	newdir/b
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	newdir/bar/file
100644 blob $(oid f2ad6c76f0115a6ba5b00456a849810e7ec0af20)	newdir/c
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Similar to the ancestor version, but now we choose 'ours', so the rename of 'foo' gets
  # dropped and it just gets deleted. Everything else is then 'A'.
  git update-index --index-info <<EOF
100644 blob $(oid 78981922613b2afb6025042ff6bd878ac1994e85)	newdir/a
100644 blob $(oid 61780798228d17af2d34fce4cfbdf35556832472)	newdir/b
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	newdir/bar/file
100644 blob $(oid f2ad6c76f0115a6ba5b00456a849810e7ec0af20)	newdir/c
EOF
  make_resolve_tree ours A B

  rm .git/index
  # 'B' changes 'foo' and moves it to 'olddir/bar', which gets tracked to be
  # 'newdir/bar' and is taken verbatim. The clash that it finds it
  # resolves in 'B's favor, leaving only 'newdir/bar'.
  git update-index --index-info <<EOF
100644 blob $(oid 78981922613b2afb6025042ff6bd878ac1994e85)	newdir/a
100644 blob $(oid 61780798228d17af2d34fce4cfbdf35556832472)	newdir/b
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	newdir/bar
100644 blob $(oid f2ad6c76f0115a6ba5b00456a849810e7ec0af20)	newdir/c
EOF
  make_resolve_tree ours B A
)

(cd conflicting-rename
  rm .git/index
  # 'A' renames 'a' to 'a-renamed', 'B' renames 'a' to 'a-different'.
  # All these conflicts are dropped in favor of keeping the 'ancestor' *location*.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/sub/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/w
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/x.f
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Much like the ancestor version, except that it applied the 'A' rename,
  # along with its *merged* content.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-renamed/sub/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/sub/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/w
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-renamed/x.f
EOF
  make_resolve_tree ours A B
  rm .git/index
  # Just like 'A' above, but with the 'B' rename chosen and all the merges.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-different/sub/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-different/sub/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-different/w
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-different/x.f
EOF
  make_resolve_tree ours B A
)

(cd conflicting-rename-2
  rm .git/index
  # Like 'conflicting-rename', but this one only renames a single sub-directory for very much the same effect.
  # Thus, keeping the 'ancestor' version is the same as 'main', except for merged content.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/sub/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/w
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/sub-renamed/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub-renamed/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/w
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
EOF
  make_resolve_tree ours A B
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/sub-different/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub-different/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/w
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
EOF
  make_resolve_tree ours B A
)

(cd conflicting-rename-complex
  rm .git/index
  # 'A" renames 'a' to 'a-renamed', but 'B' moves 'a/sub/' up one level, and replaces everything in its wake
  # so its two files are the only ones left.
  # As result, we actually have one unconflicting change which ends up creating the new directory 'a-renamed',
  # but everything else is conflicting so it keeps the 'ancestor' version.
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/z
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/sub/y.f
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/z
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/w
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/x.f
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # 'Ours' keeps its own version of the files 'B' deleted along with its own rename destinations,
  # where the cleanly mergeable 'y.f' content is still used. Of 'theirs', only the unconflicting
  # composed rename destination 'a-renamed/z' remains.
  git update-index --index-info <<EOF
  100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-renamed/sub/y.f
  100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/sub/z
  100644 blob $(oid 8a1218a1024a212bb3db30becd860315f9f3ac52)	a-renamed/x.f
  100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/z
  100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/w
EOF
  make_resolve_tree ours A B

  rm .git/index
  # It applies the merged result of the content, and interestingly also managed to reconcile the rename from 'A'.
  # However, it also drops all of 'their' conflicting changes in favor of 'ours', a respectable result.
  git update-index --index-info <<EOF
  100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a-renamed/y.f
  100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a-renamed/z
EOF
  make_resolve_tree ours B A
)

(cd renamed-symlink-with-conflict
  rm .git/index
  # 'A' changes 'a/x.f' and renames the 'link', while 'B' also changes 'a/x.f' in a mergable fashion,
  # while renaming 'link' to something else which is where the conflict comes from.
  # Choosing the 'ancestor' means to not rename 'link' at all, while merging the file.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Here we choose the name of 'link' in 'A'.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link-renamed
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Here we choose the name of 'link' in 'B'.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link-different
EOF
  make_resolve_tree ours B A
)

(cd type-change-and-renamed
  rm .git/index
  # 'A' changes `link` to a file, while 'B' keeps the link, but renames it.
  # 'ancestor' just keeps the original version of 'link'
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # 'A' changes the type of 'link' to be a file, and that's what's used here.
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/x.f
100644 blob $(oid f89a08d1e226b9a319210641b63b07dcf0bd705f)	link
EOF
  make_resolve_tree ours A B

  rm .git/index
  # 'B' renames the link, and that is picked up as well.
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link-renamed
EOF
  make_resolve_tree ours B A
)

(cd change-and-delete
  rm .git/index
  # 'A' changes 'link' to be a file, and changes the file, while 'B' deletes everything,
  # causing each file to be irreconcilable.
  # 'ancestor' keeps everything as is.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/x.f
120000 blob $(oid e29fa63dae4ccf0788897a7025da868083178fdf)	link
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # 'A' changes everything, and that's the change we keep.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/x.f
100644 blob $(oid f89a08d1e226b9a319210641b63b07dcf0bd705f)	link
EOF
  make_resolve_tree ours A B

  rm .git/index
  # 'B' deletes everything, which is what we keep.
  make_resolve_tree ours B A
)

(cd submodule-both-modify
  rm .git/index
  # There is only one submodule. 'A' and 'B' change it in a fast-forwardable manner,
  # but we can't handle this at all yet, and thus have to consider it irreconcilable.
  # The 'ancestor' resolution just keeps what was.
  git update-index --index-info <<EOF
160000 commit $(oid e835c0c403c8e494c0ca98f3d25d0b8464c18d38)	sub
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Otherwise it's the state of 'A'.
  git update-index --index-info <<EOF
160000 commit $(oid 64466ebdff775ad618d9cc993cf52840e0af528c)	sub
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Otherwise it's the state of 'B'.
  git update-index --index-info <<EOF
160000 commit $(oid ea6eb701e03c2497915c25a851f3da8f8e362ca0)	sub
EOF
  make_resolve_tree ours B A
)

(cd gitlink-replaced-by-files
  # Resolving tree conflicts with the ancestor does not resolve the content
  # conflict, so retain Git's directional conflict-marker trees.
  IFS= read -r -d '' merged_tree_id <A-B.merge-info
  git read-tree "$merged_tree_id"
  make_resolve_tree ancestor A B
  IFS= read -r -d '' merged_reversed_tree_id <A-B-reversed.merge-info
  git read-tree "$merged_reversed_tree_id"
  make_resolve_tree ancestor B A

  # ResolveWith::Ours also configures the blob merge to keep the current side.
  git read-tree A
  make_resolve_tree ours A B
  git read-tree B
  make_resolve_tree ours B A
)

(cd multiple-merge-bases
  rm .git/index
  # 'A' modifies and 'B' deletes the single file in the tree.
  # 'ancestor' keeps the original, which is already the result of the merge of
  # the merge-bases.
  git update-index --index-info <<EOF
100644 blob $(oid 09c277aa66897c58157f57a374eacc63a407dcab)	content
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # 'A' keeps the modified version.
  git update-index --index-info <<EOF
100644 blob $(oid 0a6a0ba83635bc00e7c79a4b5b6e50381385c1af)	content
EOF
  make_resolve_tree ours A B

  rm .git/index
  # 'B' applies the deletion to get an empty tree.
  make_resolve_tree ours B A
)

(cd non-tree-to-tree
  rm .git/index
  # 'A' changes the single file 'a', while 'B' replaces it with a directory structure 'a',
  # without a rename though.
  # We manage to pick the 'ancestor', just a single file, while discarding all follow-up changes.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Picks 'A' which is just a single, modified (and mergable) file.
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Picks 'B' which is a whole directory tree.
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/d
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/e
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/b
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/c
EOF
  make_resolve_tree ours B A
)

(cd tree-to-non-tree
  rm .git/index
  # 'A' modifies a nested file 'a/sub/b', while 'B' replaces 'a/' with file 'a'.
  # Ignore *their* changes for 'ancestor' resolution, and the modification,
  # but apply all others which are deletions.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/sub/b
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A
  # *ours* is the same as ancestor, we want to keep the tree and changes, but it only
  # applies to the one modification that protects the change.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/sub/b
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Now *ours* is the single file which replaces a tree.
  git update-index --index-info <<EOF
100644 blob $(oid fa49b077972391ad58037050f2a75f74e3671e92)	a
EOF
  make_resolve_tree ours B A
)

(cd tree-to-non-tree-with-rename
  rm .git/index
  # 'A' modifies a nested file 'a/sub/b', while 'B' replaces 'a/' with file 'a'.
  # I let it pass as it's an edge-case to some extent.
  git update-index --index-info <<EOF
100644 blob $(oid 44065282f89b9bd6439ed2e4674721383fd987eb)	a/sub/b
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  rm .git/index
  # Thanks to the rename, this version keeps one additional file, 'a/e'
  git update-index --index-info <<EOF
  100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/sub/b
EOF
  make_resolve_tree ours A B

  rm .git/index
  # Now *ours* is the single file which replaces a tree.
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a
EOF
  make_resolve_tree ours B A
)

(cd non-tree-to-tree-with-rename
  # 'A' changes the single file 'a', while 'B' replaces it with a directory structure 'a'.
  # The rename now sends this off-course, as it removes the previously 'protective' entry
  # and thus makes all changes from 'B' succeed without us detecting any problem with that.
  # Also, here we don't actually have irreconcilable tree-changes because of that.
  rm .git/index
  git update-index --index-info <<EOF
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/d
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/e
100644 blob $(oid b414108e81e5091fe0974a1858b4d0d22b107f70)	a/sub/b
100644 blob $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391)	a/sub/c
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # Thanks to the rename, this time there isn't even an irreconcilable conflict.
  # This is deactivated as the test-suite can't handle this (one) special case.
  # The state really is the one from the ancestors, as there are no irreconcilable
  # tree changes, it's all the same. But the test expects to see changes when 'ours'
  # is chosen so we can't easily run this here.
  #  make_resolve_tree ours A B
  #  make_resolve_tree ours B A
)

(cd rename-within-rename
  # 'A' and 'B' change all content in a mergable manner. 'A' renames 'a' to 'a-renamed',
  # and 'B' renames 'a/sub' to 'a/sub-renamed'.
  # Ideally, we get both together, but doing so added a lot of complexity so maybe give
  # that another go and try to keep it simple.
  # In ancestor mode, only those ancestors of conflicts are kept unchanged, so some renames
  # go through.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
100644 $(oid 44065282f89b9bd6439ed2e4674721383fd987eb) 0	a/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a/sub/z
EOF
  make_resolve_tree ancestor A B
  make_resolve_tree ancestor B A

  # *ours* is `a-renamed` everything, with merges.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/sub/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/sub/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
EOF
  make_resolve_tree ours A B

  # Now ours is the renamed sub-directory, with merges. It can bring everything together even.
  rm .git/index
  git update-index --index-info <<EOF
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/sub-renamed/y.f
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/sub-renamed/z
100644 $(oid e69de29bb2d1d6434b8b29ae775ad8c2e48c5391) 0	a-renamed/w
100644 $(oid b414108e81e5091fe0974a1858b4d0d22b107f70) 0	a-renamed/x.f
EOF
  make_resolve_tree ours B A
)
