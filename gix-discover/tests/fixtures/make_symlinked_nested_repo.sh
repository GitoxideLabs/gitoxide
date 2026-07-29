#!/usr/bin/env bash
set -eu -o pipefail

# The nested repository distinguishes link/..'s lexical parent from its physical parent.
git init -q .
mkdir -p subdir/real-dir
git init -q lexical-parent
ln -s ../subdir lexical-parent/link

# A symlink above a repository checks that discovery retains the caller's path spelling.
git init -q real-parent/repo
mkdir real-parent/repo/nested
# This exercises preserving the symlinked spelling when a bare repository is found directly or from objects/.
git init -q --bare real-parent/bare.git
ln -s real-parent linked-parent
