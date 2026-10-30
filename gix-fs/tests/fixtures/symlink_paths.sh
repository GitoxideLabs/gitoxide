#!/usr/bin/env bash
set -eu -o pipefail

# Path validation rejects live, dangling, and ancestor links, even inside the root.
gix_testtools_require_symlinks
mkdir -p base/dir
printf contents >base/file
printf contents >base/dir/file
printf obstruction >base/blocked
ln -s dir base/dir-link
ln -s file base/file-link
printf contents >base/removed
ln -s removed base/dangling
rm base/removed
ln -s .. base/outside
ln -s base root-link
