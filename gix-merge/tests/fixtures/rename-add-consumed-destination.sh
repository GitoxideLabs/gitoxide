#!/usr/bin/env bash
set -eu

# Reduced from crash-671cfebac375be1a2d9585bd57f8328eb212d058. Repeated
# gitlinks create crossing rename sources. Normalizing them first keeps the added
# gitlink at a/h and the binary at h/h; no destination type collision remains.
git init repo
cd repo
empty_tree=$(git mktree </dev/null)
commit_id=$(echo base | git commit-tree "$empty_tree")
added_commit_id=$(echo added | git commit-tree "$empty_tree")
blob_id=$(printf 'binary payload 7\0\n' | git hash-object -w --stdin)

git update-index --index-info <<EOF
160000 $commit_id	a/a/h
100644 $blob_id	c/h
160000 $commit_id	h/h/a
EOF
git write-tree >trees

git read-tree --empty
git update-index --index-info <<EOF
160000 $added_commit_id	a/h
160000 $commit_id	c/h/h
160000 $commit_id	h/h/a
EOF
git write-tree >>trees

git read-tree --empty
git update-index --index-info <<EOF
160000 $commit_id	c/a/h
100644 $blob_id	h/h
EOF
git write-tree >>trees
printf '%s\n' "$commit_id" "$added_commit_id" "$blob_id" >objects
