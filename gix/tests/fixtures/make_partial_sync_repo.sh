#!/usr/bin/env bash
set -eu
# Distinct historical values make a blobless client miss an old blob even after
# hydrating the tip. Both upload-pack protocols and partial-clone pushes use it.
git init --initial-branch=main source
git -C source config uploadpack.allowFilter true
git -C source config uploadpack.allowAnySHA1InWant true
printf 'old value\n' >source/value
git -C source add value
git -C source commit -m old
printf 'tip value\n' >source/value
git -C source commit -am tip

# Filtering support is optional in upload-pack, so clients must fall back to
# receiving the full graph if the peer does not advertise it.
git clone --bare source unfiltered
git -C unfiltered config uploadpack.allowFilter false
