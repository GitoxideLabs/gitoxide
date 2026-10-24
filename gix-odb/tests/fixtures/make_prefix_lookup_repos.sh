#!/usr/bin/env bash
set -eu -o pipefail

for format in sha1 sha256; do
  git init -q --object-format="$format" -b main "$format"
  (
    cd "$format"
    git commit -q --allow-empty -m initial
    for revision in HEAD 'HEAD^{tree}'; do
      for length in 4 7 40; do
        prefix=$(git rev-parse --short="$length" "$revision")
        printf '%s\n' "$prefix" >>prefixes.git
        git rev-parse --verify "$prefix" >>prefixes.git
      done
    done

    # These two blob contents share the prefix "dead" under both object formats.
    blob_id=$(echo brocdnra | git hash-object -w --stdin)
    echo brigddsv | git hash-object -w --stdin >/dev/null
    git rev-parse --disambiguate=dead >ambiguous.git
    test "$(wc -l <ambiguous.git | tr -d ' ')" = 2
    cp -R .git/objects loose-objects

    # Keep one candidate packed and the other loose to exercise cross-store ambiguity.
    printf '%s\n' "$blob_id" | git pack-objects .git/objects/pack/pack >/dev/null
    rm ".git/objects/${blob_id:0:2}/${blob_id:2}"
    git rev-parse --disambiguate=dead >mixed.git
    cmp ambiguous.git mixed.git
  )
done
