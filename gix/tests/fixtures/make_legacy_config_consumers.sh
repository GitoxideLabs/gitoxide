#!/usr/bin/env bash
set -eu -o pipefail

git init -q
# Exercise canonical legacy names in configuration consumers without executing configured commands.
# Legacy and lowercase quoted names select the same driver; uppercase quoted names stay distinct.
cat >>.git/config <<'EOF'
[filter.Legacy]
    clean = legacy-clean
[filter "legacy"]
    smudge = quoted-smudge
[filter "Legacy"]
    clean = distinct-clean
[diff.Legacy]
    command = legacy-diff
[diff "legacy"]
    textconv = quoted-textconv
[diff "Legacy"]
    binary = true
[merge.Legacy]
    driver = legacy-merge
[merge "legacy"]
    recursive = binary
[merge "Legacy"]
    driver = distinct-merge
EOF
test "$(git config --get filter.legacy.clean)" = legacy-clean
test "$(git config --get diff.legacy.command)" = legacy-diff
test "$(git config --get merge.legacy.driver)" = legacy-merge

# URL bases are values derived from canonical subsection names, including local-path bases.
cat >>.git/config <<'EOF'
[url.BASE]
    insteadOf = alias:
    pushInsteadOf = push:
[url "BASE"]
    insteadOf = quoted:
[remote "rewrite"]
    url = alias:repository
    pushurl = quoted:repository
[remote "pushonly"]
    url = push:repository
EOF
git remote get-url rewrite >expected-fetch-url
git remote get-url --push rewrite >expected-push-url
git remote get-url --push pushonly >expected-push-fallback

# Gix also accepts local paths when choosing credential helpers; no helper is invoked here.
cat >>.git/config <<'EOF'
[credential.LOCAL]
    helper = legacy-helper
[credential "LOCAL"]
    helper = quoted-helper
EOF
test "$(git config --get credential.local.helper)" = legacy-helper
