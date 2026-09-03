#!/usr/bin/env bash

set -euo pipefail

usage() {
    printf 'usage: %s (--fast|--thorough)\n' "${0##*/}"
}

if [[ "$#" -ne 1 ]]; then
    usage >&2
    exit 2
fi

case "$1" in
--fast | --thorough)
    mode="$1"
    ;;
*)
    usage >&2
    exit 2
    ;;
esac

# Run with a real PTY: non-CI journey snapshots exercise TUI output, and
# macOS script(1) changes the byte stream they are meant to verify.
repo_root="$(git rev-parse --show-toplevel)"
cd -- "$repo_root"
stderr_file="$(mktemp "${TMPDIR:-/tmp}/ci-check-local.stderr.XXXXXX")"
trap 'rm -f -- "$stderr_file"' EXIT

print_command() {
    printf '%q ' "$@"
}

run() {
    printf 'check: '
    print_command "$@"
    printf '\n'
    if "$@" >/dev/null 2>"$stderr_file"; then
        return
    else
        status=$?
    fi
    cat -- "$stderr_file" >&2
    printf 'FAILED (%d): ' "$status" >&2
    print_command "$@" >&2
    printf '\n' >&2
    return "$status"
}

require_clean() {
    changes="$(git status --porcelain=v1)"
    if [[ -n "$changes" ]]; then
        printf '%s\n%s\n' "FAILED: test -z \"\$(git status --porcelain=v1)\"" "$changes" >&2
        return 1
    fi
}

run_fast_checks() {
    run cargo fmt --all -- --check
    run cargo machete
    run just clippy -D warnings -A unknown-lints --no-deps
    # cargo-deny 0.20 accepts workspace/features before the check subcommand.
    run cargo deny --workspace --all-features check bans licenses sources
}

if [[ "$mode" == --fast ]]; then
    run_fast_checks
    exit 0
fi

require_clean
run_fast_checks
# Normal CI avoids rewriting platform-specific fixture archives. The explicit
# nextest run below still executes the full workspace with archive creation.
run env GIX_TEST_IGNORE_ARCHIVES=1 just ci-test
run just doc-tests
run env GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI=1 cargo nextest run --workspace --no-fail-fast --exclude gix-error
# Archive-generating tests legitimately rewrite these tracked fixtures. Restore
# only this known class so every other mutation stays visible.
git restore -- ':(glob)**/tests/fixtures/generated-archives/*.tar'
run just ci-journey-tests
require_clean
run tix enrich tree checks-pass
if command -v dua >/dev/null 2>&1; then
    # A warm full-workspace sweep currently settles around 70 GB.
    printf 'target size after checks (expect about 70 GB):\n'
    NO_COLOR=1 dua target 2>/dev/null
else
    printf 'note: install dua to monitor target size\n' >&2
fi
