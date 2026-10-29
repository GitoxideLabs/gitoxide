#!/usr/bin/env bash
set -eu -o pipefail

# Modification after a rename must keep the earlier path in porcelain's previous origin.
git init -q
printf 'first\nsecond\nthird\nfourth\n' > original.txt
printf target > typechange.txt
git add original.txt typechange.txt
git commit -q -m root
git tag root
printf 'first\nSECOND\nthird\nfourth\n' > original.txt
git commit -qam modified
git mv original.txt renamed.txt
printf 'first\nSECOND\nTHIRD\nfourth\n' > renamed.txt
rm typechange.txt
ln -s target typechange.txt
git commit -qam renamed
printf 'first\nSECOND\nTHIRD\nfourth\nuncommitted\n' > renamed.txt

# A local shallow boundary must still be honored when all ancestor objects are present.
git clone -q --bare . shallow.git
git rev-parse HEAD > shallow.git/shallow
