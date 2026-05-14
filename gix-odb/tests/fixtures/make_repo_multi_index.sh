#!/usr/bin/env bash
set -eu -o pipefail

omit_multi_index=${1:-no}

git init -q

function write_files() {
  local base_dir=${1:?directory to write them into}
  local num_files=${2:?amount of files to write}
  local nonce=${3:?something to make files more unique}

  mkdir -p "$base_dir"
  for file_id in $(seq -w "$num_files"); do
    seq "$file_id" > "$base_dir/$file_id"
    echo "$nonce" >> "$base_dir/$file_id"
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
