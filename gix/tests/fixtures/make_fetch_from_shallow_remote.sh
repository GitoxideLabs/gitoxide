#!/usr/bin/env bash
set -eu -o pipefail

# The shallow source has a new tip, but its boundary and all earlier history are in `full`.
git init full
git -C full commit --allow-empty -m root
git -C full commit --allow-empty -m base
git clone --depth=1 "file://$PWD/full" shallow
git -C shallow commit --allow-empty -m tip
