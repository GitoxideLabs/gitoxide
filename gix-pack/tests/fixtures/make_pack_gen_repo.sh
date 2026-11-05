#!/usr/bin/env bash
set -eu -o pipefail

git init -q

function write_files() {
  local base_dir=${1:?directory to write them into}
  local num_files=${2:?amount of files to write}
  local nonce=${3:?something to make files more unique}
  local file_id file_name contents=

  mkdir -p "$base_dir"
  for ((file_id = 1; file_id <= num_files; file_id++)); do
    printf -v file_name '%0*d' "${#num_files}" "$file_id"
    contents+="$file_id"$'\n'
    printf '%s%s\n' "$contents" "$nonce" > "$base_dir/$file_name"
  done
}

dirs=(. a b c a/a a/b a/c a/a/a)
rounds=15

git checkout -q -b main
for ((round = 1; round <= rounds; round++)); do
  dir_index=$(( round % ${#dirs[@]} ))
  num_files=$(( (round + 1) * 6 ))
  write_files "${dirs[$dir_index]}" $num_files "$round"
  git add .
  git commit -qm "$round $num_files"
done

echo hello world > referee
git add referee
git commit -qm "to be forgotten"
git tag -m "a tag object" referrer
git reset --hard HEAD~1

# speed up all access by creating a pack
git gc --aggressive
