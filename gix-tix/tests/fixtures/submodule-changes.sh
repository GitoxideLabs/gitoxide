#!/bin/sh
set -eu

# Gitlinks point outside the superproject's object database, as with vendor/git in grix.
git init -q -b main .
echo text >file
git add file
git commit -qm root
old_commit_id=$(echo old | git hash-object --stdin)
new_commit_id=$(echo new | git hash-object --stdin)

git update-index --add --cacheinfo "160000,$old_commit_id,submodule"
git commit -qm added
git tag added
git update-index --cacheinfo "160000,$new_commit_id,submodule"
git commit -qm modified
git tag modified

# Both directions of a type change must avoid loading the gitlink as a blob.
echo replacement >submodule
git add submodule
git commit -qm to-file
git tag to-file
git update-index --cacheinfo "160000,$old_commit_id,submodule"
git commit -qm to-submodule
git tag to-submodule
rm submodule
git update-index --force-remove submodule
git commit -qm deleted
git tag deleted
