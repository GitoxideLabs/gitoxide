#!/usr/bin/env bash
set -eu -o pipefail

omit_multi_index=${1:-no}

git init -q

function write_files() {
  local base_dir=${1:?directory to write them into}
  local num_files=${2:?amount of files to write}
  local nonce=${3:?something to make files more unique}
  local file_id file_name lines=''

  mkdir -p "$base_dir"
  # Grow the sequence in Bash to avoid launching `seq` once for each of the 810 files.
  for ((file_id = 1; file_id <= num_files; file_id++)); do
    printf -v file_name '%0*d' "${#num_files}" "$file_id"
    lines+="$file_id"$'\n'
    printf '%s%s\n' "$lines" "$nonce" > "$base_dir/$file_name"
  done
}

dirs=(. a b c a/a a/b a/c a/a/a)
rounds=15

git checkout -q -b main
for round in $(seq $rounds); do
  dir_index=$(( round % ${#dirs[@]} ))
  num_files=$(( (round + 1) * 6 ))
  write_files "${dirs[$dir_index]}" $num_files "$round"
  git add .
  git commit -qm "$round $num_files"
  git repack
done

echo hello world > referee
git add referee
git commit -qm "to be forgotten"
git tag -m "a tag object" referrer
git reset --hard HEAD~1

if [ "$omit_multi_index" == "no" ]; then
  git multi-pack-index write
fi

git count-objects -v > expected-count-objects
