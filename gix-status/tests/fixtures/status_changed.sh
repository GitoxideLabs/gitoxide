#!/usr/bin/env bash
set -eu -o pipefail

git init -q

touch empty
echo -n "content" > executable
chmod +x executable
# Both regular-file modes must become symlinks, rather than just change permissions.
echo content > z-executable
chmod +x z-executable

mkdir dir
echo -n "other content" > dir/content
echo -n "other content" > dir/content2
mkdir dir/sub-dir
(cd dir/sub-dir && ln -sf ../content symlink)

git add -A
git update-index --chmod=+x executable z-executable  # For Windows.
git commit -m "Commit"

chmod +x dir/content
echo "new content" > dir/content2
chmod -x executable
echo -n "foo" > executable

rm empty
ln -sf dir/content empty
rm z-executable
ln -sf dir/content z-executable
git reset
git status --porcelain=v1 -- empty z-executable > .git/type-changes.baseline
# Disabling symlink checkout still reports actual links as mode 120000.
# Keep just the raw modes so the baseline is independent of the object hash.
for filemode in false true; do
  git -c core.symlinks=false -c core.filemode="$filemode" diff --raw -- empty z-executable |
    cut -d ' ' -f 1,2 > ".git/type-change-modes-$filemode.baseline"
done
