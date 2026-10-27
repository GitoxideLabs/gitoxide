#!/usr/bin/env bash
set -eu -o pipefail

git init -q -b main
git commit -q --allow-empty -m initial
mkdir dir
echo hi >file
echo x >dir/f
git add file dir/f
# Index entries make this fixture independent of filesystem symlink support.
link_id=$(printf file | git hash-object -w --stdin)
git update-index --add --cacheinfo "120000,$link_id,link"
git update-index --add --cacheinfo "160000,$(git rev-parse HEAD),sub"
git commit -q -m entries

# A trailing slash requires a tree; internal empty and dot components are literal.
for pathspec in '' file file/ link link/ sub sub/ dir dir/ dir// dir/f dir/f/ dir//f dir/./f dir/../file /file; do
  printf 'HEAD:%s\n' "$pathspec" >>baseline.git
  MSYS2_ARG_CONV_EXCL='*' git rev-parse -q --verify "HEAD:$pathspec" >>baseline.git 2>/dev/null || echo $? >>baseline.git
done
