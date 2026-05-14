#!/usr/bin/env bash
set -eu -o pipefail

# This fixture is based on Git's SHA-1 disambiguation tests and relies on
# exact SHA-1 prefixes in object names, refs, and expected messages.
export GIT_DEFAULT_HASH=sha1

function baseline() {
  local spec=${1:?first argument is the spec to test}
  {
    echo "$spec"
    git rev-parse -q --verify "$spec" 2>/dev/null || echo $?
  }>> baseline.git
}

function loose-obj() {
  # Read content from stdin, compute header and hash, write compressed object
  script=$(cat <<'EOF'
import sys
import hashlib
import zlib
import os

type = sys.argv[1]
objects_dir = sys.argv[2]
content = sys.stdin.buffer.read()
header = f"{type} {len(content)}\0".encode()
full = header + content
sha1 = hashlib.sha1(full).hexdigest()
compressed = zlib.compress(full)

bucket = f"{objects_dir}/" + sha1[:2]
filename = sha1[2:]

os.makedirs(bucket, exist_ok=True)
with open(f"{bucket}/{filename}", "wb") as f:
    f.write(compressed)

print(sha1)
EOF
)
  python3 -c "$script" "$@"
}

# The contents of this file is based on https://github.com/git/git/blob/8168d5e9c23ed44ae3d604f392320d66556453c9/t/t1512-rev-parse-disambiguation.sh#L38
git init --bare blob.prefix
(
  cd blob.prefix
  # Both start with "dead..", under both SHA-1 and SHA-256
  echo brocdnra | git hash-object -w --stdin
  echo brigddsv | git hash-object -w --stdin
  # Both start with "beef.."
  echo 1agllotbh | git hash-object -w --stdin
  echo 1bbfctrkc | git hash-object -w --stdin

  baseline "dead"
  baseline "beef"
)


git init --bare blob.bad
(
  cd blob.bad
  # Both have the prefix "bad0"
  # Maybe one day we have a test to see how disambiguation reporting deals with this.
  echo xyzfaowcoh | loose-obj bad objects
  echo xyzhjpyvwl | loose-obj bad objects
  baseline "bad0"

  echo 1bbfctrkc | loose-obj bad objects
  baseline "e328"
  baseline "e328^{object}"
)

function oid_to_path() {
  local basename=${1#??}
  echo "${1%$basename}/$basename"
}

git init --bare blob.corrupt
(
  cd blob.corrupt
  # Both have the prefix "cafe".
  # Maybe one day we have a test to see how disambiguation reporting deals with this.
  echo bnkxmdwz | git hash-object -w --stdin
  oid=$(echo bmwsjxzi | git hash-object -w --stdin)
  oidf=objects/$(oid_to_path "$oid")
  chmod -- 644 "$oidf"
  echo broken >"$oidf"

  baseline "cafea"
  baseline "cafea^{object}"
)

# This function writes out its parameters, one per line
function write_lines () {
  	printf "%s\n" "$@"
}

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

GIT_AUTHOR_EMAIL=author@example.com
GIT_AUTHOR_NAME='A U Thor'
GIT_AUTHOR_DATE='1112354055 +0200'
TEST_COMMITTER_LOCALNAME=committer
TEST_COMMITTER_DOMAIN=example.com
GIT_COMMITTER_EMAIL=committer@example.com
GIT_COMMITTER_NAME='C O Mitter'
GIT_COMMITTER_DATE='1112354055 +0200'


git init ambiguous_blob_tree_commit
(
  tick
  cd ambiguous_blob_tree_commit
  (
    write_lines 0 1 2 3 4 5 6 7 8 9
    echo
    echo b1rwzyc3
  ) >a0blgqsjc
  # create one blob 0000000000b36
  git add a0blgqsjc
  # create one tree 0000000000cdc
  git write-tree

  sed -e "s/|$//" >patch <<EOF
diff --git a/frotz b/frotz
index 000000000..ffffff 100644
--- a/frotz
+++ b/frotz
@@ -10,3 +10,4 @@
 9
 |
 b1rwzyc3
+irwry
EOF

  (
    GIT_INDEX_FILE=frotz
    export GIT_INDEX_FILE
    git apply --build-fake-ancestor frotz patch
  )

  # create one commit 0000000000e4f
  git commit -m a2onsxbvj

  # this commit is ffffffd8 and not ambiguous with the 00000* objects.
  # This is not relevant to gitoxide but we need the complete history.
  echo "hoaxj" | git commit-tree 0000000000cdc -p 000000000

  baseline "0000000000"
  baseline "0000000000cdc:a0blgqsjc" # unambiguous by nature
  baseline "0000000000:a0blgqsjc"    # would be ambiguous, but only trees can have this syntax
  baseline "0000000000cdc:missing"   # definitely not present
  baseline "0000000000cdc^{tree}"    # unambiguous with tree assertion
  baseline "0000000000^{tree}"       # ambiguous with tree assertion, but git can't do it
  baseline "0000000000e4f"           # unambiguous
  baseline "0000000000e"             # unambiguous
  baseline "0000000000e4f^{commit}"  # and with commit assertion
  baseline "0000000000^{commit}"     # ambiguous name, but there is only one commit, but git can't do it
  baseline "0000000000^0"            # another transformation that only works for commits
  baseline "0000000000f8f"           # disambiguated tag
  baseline "0000000000^{tag}"        # disambiguated by type, but git can't do it

  baseline "000000000..000000000"    # only one commit is present with this prefix and we prefer these in ranges
  baseline "..000000000"
  baseline "000000000.."

  baseline "000000000...000000000"    # only one commit is present with this prefix and we prefer these in ranges
  baseline "...000000000"
  baseline "000000000..."
  baseline "0000000000^{/x}"          # disambiguation thanks to a commit being required
  baseline "@^{/x}"                   # the same, but with named revision
  baseline "@^{/^.*x}"                # regex are available in git

  cd ..
  git clone ambiguous_blob_tree_commit ambiguous_commits
  cd ambiguous_commits;

  # create one tag 0000000000f8f (making the previous baseline tests ambiguous, but it could be unambiguous since they point to the same commit)
  git tag -a -m j7cp83um v1.0.0

  baseline "0000000000f8"             # unambiguous
  baseline "0000000000^{tag}"         # git can't do this yet
  baseline "v1.0.0-0-g0000000000e4f"  # unambiguous commit
  baseline "v1.0.0-0-g0000000000"     # ambiguous commit, but we know we need a commit here. The tag doesn't matter.


  # commit 0000000000043
  git mv a0blgqsjc d12cr3h8t
  echo h62xsjeu >>d12cr3h8t
  git add d12cr3h8t

  tick
  git commit -m czy8f73t

  # commit 00000000008ec
  git mv d12cr3h8t j000jmpzn
  echo j08bekfvt >>j000jmpzn
  git add j000jmpzn

  tick
  git commit -m ioiley5o

  # commit 0000000005b0
  git checkout v1.0.0^0
  git mv a0blgqsjc f5518nwu

  write_lines h62xsjeu j08bekfvt kg7xflhm >>f5518nwu
  git add f5518nwu

  tick
  git commit -m b3wettvi
  side=$(git rev-parse HEAD)

  # commit 000000000066
  git checkout main

  # If you use recursive, merge will fail and you will need to
  # clean up a0blgqsjc as well.  If you use resolve, merge will
  # succeed.
  git merge --no-commit -s recursive $side || true
  git rm -f f5518nwu j000jmpzn

  git rm -f a0blgqsjc
  (
    git cat-file blob $side:f5518nwu
    echo j3l0i9s6
  ) >ab2gs879
  git add ab2gs879

  tick
  git commit -m ad2uee

  baseline "0000000000f^{tree}"  # there is a tag, tree and blob with this prefix, and tags can also be trees thus this is ambiguous if the tree is different
  baseline "00000000^{commit}"   # this fails as there are many commits with this prefix now

  baseline "v1.0.0-0-g000000000" # git doesn't take advantage of the generation and anchor reference yet
  baseline "v1.0.0-2-g000000000" # ^
  baseline "v1.0.0-4-g000000000" # ^

  baseline "v1.0.0-1-g000000000" # This should legitimately fail (currently git accidentally fails) as there are two commits at gen 1 with this prefix.

  baseline "000000000..000000000"  # only one commit is present with this prefix and we prefer these in ranges
  baseline "..000000000"
  baseline "000000000.."

  baseline "000000000...000000000" # only one commit is present with this prefix and we prefer these in ranges
  baseline "...000000000"
  baseline "000000000..."

  baseline "00000000006^!"     # exclude parents (without anything related to ambiguity
  baseline "00000000006^@"     # include parents (without actual commit)
)

git clone ambiguous_commits duplicate_ambiguous_objects
(
  cd duplicate_ambiguous_objects
  git rev-parse --disambiguate=000000000 >expect
  git pack-objects .git/objects/pack/pack <expect
  git rev-parse --disambiguate=000000000 >actual
  diff actual expect # git deduplicates the same objects even though they are in the loose and packed odb
)

git clone ambiguous_blob_tree_commit ambiguous_refs
(
    cd ambiguous_refs
  	TREE=$(git mktree </dev/null)
  	REF=$(git rev-parse HEAD)
  	VAL=$(git commit-tree $TREE </dev/null)
  	git update-ref refs/heads/$REF $VAL

  	baseline "$REF" # there is a ref and an object with the same name

    TREE=$(git mktree </dev/null)
    REF=$(git rev-parse --short HEAD)
    VAL=$(git commit-tree $TREE </dev/null)
    git update-ref refs/heads/$REF $VAL

  	baseline "$REF" # there is a ref and an object with the same name
)

for name in committish treeish tree commit blob; do
  clone_dir=ambiguous_objects_disambiguation_config_$name
  git clone ambiguous_commits $clone_dir
  (
      cd $clone_dir
      git config core.disambiguate $name
      baseline "0000000000"
      baseline "0000000000f"
      baseline "0000000000f^{tree}"
      baseline "00000000000..00000000000"
  )
done

git init complex_graph
(
  cd complex_graph
  tick

  echo g > file
  git add file && git commit -m $'G\n\n initial message'
  git branch g

  tick
  git checkout --orphan=h
  echo h > file
  git add file && git commit -m H

  tick
  git checkout main
  git merge h --allow-unrelated-histories || :
  { echo g && echo h && echo d; } > file
  git add file
  git commit -m D
  git branch d

  tick
  git checkout --orphan=i
  echo i > file
  git add file && git commit -m I
  git tag -m I-tag i-tag

  tick
  git checkout --orphan=j
  echo j > file
  git add file && git commit -m J

  tick
  git checkout i
  git merge j --allow-unrelated-histories || :
  { echo i && echo j && echo f; } > file
  git add file
  git commit -m F
  git branch f

  tick
  git checkout --orphan=e
  echo e > file
  git add file && git commit -m E

  tick
  git checkout main
  git merge e i --allow-unrelated-histories || :
  { echo g && echo h && echo i && echo j && echo d && echo e && echo f && echo b; } > file
  git add file && git commit -m B
  git tag -m b-tag b-tag && git branch b

  tick
  git checkout i
  echo c >> file
  git add file && git commit -m $'C\n\n message recent'
  git branch c
  git reset --hard i-tag

  tick
  git checkout main
  git merge c || :
  { echo g && echo h && echo i && echo j && echo d && echo e && echo f && echo b && echo c && echo a; } > file
  git add file && git commit -m A
  git branch a

  git remote add origin .
  cat <<EOF>>.git/config

  [branch "main"]
  	remote = origin
  	merge = refs/heads/main
EOF

  git fetch

  baseline ":/message" # finds 'message recent' instead of 'initial message'
  baseline ":/!-message" # above, negated
  baseline ":/mes.age" # regexes work too
  baseline ":/!-mes.age" # negated above
  baseline ":/not there" # definitely not in graph
  baseline "@^{/!-B}"    # negation from branch
  baseline ":file"      # index lookup, default stage 0
  baseline ":1:file"    # stage 1
  baseline ":5:file"    # invalid stage
  baseline ":foo"       # not found
  # parents
  baseline "a"
  baseline "a^1"
  baseline "a~1"
  baseline "a^0"
  baseline "a~0"
  baseline "a^42"
  baseline "a~42"
  baseline "a~3"

  baseline "b"
  baseline "a^"
  baseline "c"
  baseline "a^2"
  baseline "d"
  baseline "a^^"
  baseline "a^1^1"
  baseline "a~2"
  baseline "e"
  baseline "a^^2"
  baseline "j"
  baseline "b^3^2"
  baseline "a^^3^2"

  # invalid
  baseline "^^"
  baseline "^^HEAD"

  baseline "@{-1}"
  baseline "@{-2}"
  baseline "@{-3}"
  baseline "@{-4}"
  baseline "@{-5}"
  baseline "@{-6}"

  baseline "@{0}"
  baseline "@{3}"
  baseline "HEAD@{5}"
  baseline "main@{12345}"

  baseline "@^{}"
  baseline "main^{}"
  baseline "b-tag^{}"

  baseline "@^{tree}"
  baseline "@:"
  baseline "4b825dc642cb6eb9a060e54bf8d69288fbee4904"

  baseline @{push}
  baseline main@{push}
  baseline main@{upstream}
  baseline @{upstream}

  baseline "^"
  baseline "^!"
  baseline "..."
  baseline @..@
  baseline @...@
)

git init new
(cd new
  baseline '@{1}'
)

git init invalid-head
(cd invalid-head
  >file && git add file && git commit -m "init"
  rm .git/refs/heads/main
  baseline 'HEAD'
  baseline 'HEAD:file'
)
