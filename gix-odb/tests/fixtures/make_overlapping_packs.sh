#!/usr/bin/env bash
set -eu -o pipefail

# Ten packs of strictly decreasing size, so a new handle probes their indices in the order they are
# written here, and an index that answers moves from slot 2 or from slot 9 of that order.
# The packs share blobs, which makes the order of the indices observable: the first index that holds a
# blob answers for it.
# - `shared` is in the biggest pack and the one after it.
# - `in-2` is only in the pack in slot 2, and `in-2-and-front` is also in the biggest pack.
# - `in-9` and `in-9-and-front` are the same for the pack in slot 9, the smallest.
git init -q

function blob() {
  echo "$1" | git hash-object -w --stdin
}

# Write a pack of the given blobs, and `$2` more filler blobs.
function pack() {
  local name=$1 fillers=$2
  shift 2
  {
    if (($#)); then printf '%s\n' "$@"; fi
    for ((i = 0; i < fillers; i++)); do blob "$name filler $i"; done
  } | git pack-objects -q .git/objects/pack/pack >/dev/null
}

for name in shared in-2 in-2-and-front in-9 in-9-and-front; do
  blob "$name" > "$name"
done

pack biggest 40 "$(cat shared)" "$(cat in-2-and-front)" "$(cat in-9-and-front)"
pack second 30 "$(cat shared)"
pack third 20 "$(cat in-2)" "$(cat in-2-and-front)"
for slot in 3 4 5 6 7 8; do
  pack "slot $slot" $((12 - slot))
done
pack smallest 0 "$(cat in-9)" "$(cat in-9-and-front)"
git prune-packed
