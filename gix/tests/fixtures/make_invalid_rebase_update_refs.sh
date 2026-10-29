#!/usr/bin/env bash
set -eu -o pipefail

# Malformed sequencer records must not make occupied branches appear available.
for state in missing-before missing-after invalid-before invalid-after invalid-ref wrong-hash; do
  git init -q --object-format=sha1 "$state"
  mkdir "$state/.git/rebase-merge"
  zero=0000000000000000000000000000000000000000
  zero256=0000000000000000000000000000000000000000000000000000000000000000
  path="$state/.git/rebase-merge/update-refs"
  case "$state" in
    missing-before) printf 'refs/heads/topic\n' >"$path" ;;
    missing-after) printf 'refs/heads/topic\n%s\n' "$zero" >"$path" ;;
    invalid-before) printf 'refs/heads/topic\ninvalid\n%s\n' "$zero" >"$path" ;;
    invalid-after) printf 'refs/heads/topic\n%s\ninvalid\n' "$zero" >"$path" ;;
    invalid-ref) printf 'refs/heads/invalid ref\n%s\n%s\n' "$zero" "$zero" >"$path" ;;
    wrong-hash) printf 'refs/heads/topic\n%s\n%s\n' "$zero256" "$zero256" >"$path" ;;
  esac
done
