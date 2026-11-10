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

# Stream all blobs into a temporary source pack, even though there are few enough for fast-import
# to unpack by default. Fillers can be shared: only the sizes of the indices matter for their order.
names=(shared in-2 in-2-and-front in-9 in-9-and-front)
{
  for ((i = 0; i < ${#names[@]}; i++)); do
    printf 'blob\nmark :%s\ndata <<EOF\n%s\nEOF\n' "$((i + 1))" "${names[i]}"
  done
  for ((i = 0; i < 40; i++)); do
    printf 'blob\nmark :%s\ndata <<EOF\nfiller %s\nEOF\n' "$((i + 6))" "$i"
  done
} | git -c fastimport.unpackLimit=0 fast-import --quiet --export-marks=.git/blob-marks
source_pack=(.git/objects/pack/*)

blob_ids=()
while read -r mark blob_id; do
  blob_ids[${mark#:} - 1]=$blob_id
done <.git/blob-marks
for i in "${!names[@]}"; do
  printf '%s %s\n' "${names[i]}" "${blob_ids[i]}"
done >blobs
read -r shared in_2 in_2_and_front in_9 in_9_and_front <<<"${blob_ids[*]:0:5}"

# Write a pack of the given blobs and `$1` fillers from the source pack.
function pack() {
  local fillers=$1
  shift
  {
    if (($#)); then printf '%s\n' "$@"; fi
    if ((fillers)); then printf '%s\n' "${blob_ids[@]:5:fillers}"; fi
  } | git pack-objects -q .git/objects/pack/pack >/dev/null
}

pack 40 "$shared" "$in_2_and_front" "$in_9_and_front"
pack 30 "$shared"
pack 20 "$in_2" "$in_2_and_front"
for slot in 3 4 5 6 7 8; do
  pack $((12 - slot))
done
pack 0 "$in_9" "$in_9_and_front"
rm "${source_pack[@]}" .git/blob-marks
