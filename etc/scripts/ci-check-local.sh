#!/usr/bin/env bash

set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd -- "$repo_root"

print_command() {
    printf '%q ' "$@"
}

run() {
    printf 'check: '
    print_command "$@"
    printf '\n'
    "$@" >/dev/null || {
        status=$?
        printf 'FAILED (%d): ' "$status" >&2
        print_command "$@" >&2
        printf '\n' >&2
        return "$status"
    }
}

require_clean() {
    changes="$(git status --porcelain=v1)"
    if [[ -n "$changes" ]]; then
        printf '%s\n%s\n' "FAILED: test -z \"\$(git status --porcelain=v1)\"" "$changes" >&2
        return 1
    fi
}

require_clean
run cargo fmt --all -- --check
run cargo machete
run just clippy -D warnings -A unknown-lints --no-deps
run cargo deny check bans licenses sources --workspace --all-features
run env GIX_TEST_IGNORE_ARCHIVES=1 just ci-test
run just doc-tests
run env GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI=1 cargo nextest run --workspace --no-fail-fast --exclude gix-error
run just ci-journey-tests
require_clean
