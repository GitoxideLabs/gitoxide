#!/usr/bin/env bash
# Walk tests/journey/parity/*.sh and produce docs/parity/SHORTCOMINGS.md
# as the canonical ledger of:
#   - `shortcoming "<reason>"` calls (closed-as-deferred rows)
#   - `compat_effect "<reason>"` calls (clap-wired, semantics-deferred rows)
#
# Output is stable: re-running on unchanged input produces byte-identical
# bytes so CI can gate on `git diff --exit-code docs/parity/SHORTCOMINGS.md`.
#
# Usage:
#   bash etc/parity/shortcomings.sh              # writes docs/parity/SHORTCOMINGS.md
#   bash etc/parity/shortcomings.sh --check      # diffs against committed file, exits 1 if stale
set -eu

# Requires gawk — uses the 3-arg `match($0, /regex/, a)` capture-array form.
# POSIX awk and mawk/BSD awk silently produce empty captures, which would
# make this generator emit empty sections without failing loudly.
if ! command -v gawk >/dev/null 2>&1; then
  echo "shortcomings.sh: requires gawk (awk must support 3-arg match). Install gawk." >&2
  exit 2
fi
AWK="$(command -v gawk)"

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$repo_root/docs/parity/SHORTCOMINGS.md"
check_mode=0
[[ "${1:-}" == "--check" ]] && check_mode=1

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

{
  echo "# Parity Shortcomings Ledger"
  echo
  echo "Auto-generated from \`tests/journey/parity/*.sh\` by \`etc/parity/shortcomings.sh\`."
  echo "**Do not edit by hand** — re-run the generator."
  echo
  echo "Two row classes:"
  echo "- **deferred** — \`shortcoming \"<reason>\"\`: row closed as a legitimate deferral; reason describes the gap."
  echo "- **compat** — \`compat_effect \"<reason>\"\`: row green at effect mode (exit-code parity); byte-output parity is a known follow-up."
  echo
}  > "$tmp"

for f in "$repo_root"/tests/journey/parity/*.sh; do
  [[ "$(basename "$f")" == "_smoke.sh" ]] && continue
  cmd="$(basename "$f" .sh)"

  # awk: walk the file, remember the most recent `title "..."`, and when
  # we hit a `shortcoming` or `compat_effect` call, emit one CSV-ish row
  # with class|title|line|reason. Title strings are single-quoted-safe
  # because the matcher is anchored to double-quote delimiters.
  rows="$("$AWK" -v cmd="$cmd" '
    /^[[:space:]]*title[[:space:]]+"/ {
      match($0, /title[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") title = a[1]
      next
    }
    /^[[:space:]]*shortcoming[[:space:]]+"/ {
      match($0, /shortcoming[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") {
        gsub(/\|/, "\\|", a[1])
        safe_title = title
        gsub(/\|/, "\\|", safe_title)
        printf("deferred\t%s\t%d\t%s\n", safe_title, NR, a[1])
      }
      next
    }
    /^[[:space:]]*compat_effect[[:space:]]+"/ {
      match($0, /compat_effect[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") {
        gsub(/\|/, "\\|", a[1])
        safe_title = title
        gsub(/\|/, "\\|", safe_title)
        printf("compat\t%s\t%d\t%s\n", safe_title, NR, a[1])
      }
      next
    }
  ' "$f")"

  [[ -z "$rows" ]] && continue

  {
    echo "## ${cmd}"
    echo
    echo "| Class | Section | Reason | Source |"
    echo "|---|---|---|---|"
  } >> "$tmp"

  # Sort by line number for stable output.
  echo "$rows" | sort -t$'\t' -k3,3n | while IFS=$'\t' read -r class title line reason; do
    printf '| %s | `%s` | %s | [%s:%s](../../tests/journey/parity/%s.sh#L%s) |\n' \
      "$class" "$title" "$reason" "$(basename "$f")" "$line" "$cmd" "$line" >> "$tmp"
  done
  echo >> "$tmp"
done

if [[ $check_mode -eq 1 ]]; then
  if ! diff -u "$out" "$tmp" >&2; then
    echo "shortcomings.sh --check: $out is stale — re-run without --check to regenerate" >&2
    exit 1
  fi
  echo "shortcomings.sh --check: $out is up to date" >&2
  exit 0
fi

mv "$tmp" "$out"
trap - EXIT
echo "wrote $out"
