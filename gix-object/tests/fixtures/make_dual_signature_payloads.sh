#!/usr/bin/env bash
set -eu -o pipefail

# Capture the exact bytes Git sends to a verifier for each object format and
# signature order. The verifier deliberately reports failure: only its input
# matters, so this fixture needs no cryptographic keys or installed GPG.
cat >record-signature-input <<'SCRIPT'
#!/bin/sh
cat >"$GIX_SIGNATURE_PAYLOAD"
exit 1
SCRIPT
chmod +x record-signature-input
for kind in sha1 sha256; do
    git init -q --object-format="$kind" "$kind"
    tree=$(git -C "$kind" hash-object -t tree -w --stdin </dev/null)
    for first in gpgsig gpgsig-sha256; do
        second=gpgsig
        if test "$first" = gpgsig; then
            second=gpgsig-sha256
        fi
        name="$kind-$first"
        {
            printf 'tree %s\n' "$tree"
            printf 'author A <a@example.com> 1000000000 +0000\n'
            printf 'committer A <a@example.com> 1000000000 +0000\n'
            printf 'encoding ISO-8859-1\n'
            printf '%s -----BEGIN PGP SIGNATURE-----\n first\n -----END PGP SIGNATURE-----\n' "$first"
            printf 'custom keep this\n continuation\n'
            printf '%s -----BEGIN PGP SIGNATURE-----\n second\n -----END PGP SIGNATURE-----\n' "$second"
            printf '\nmessage without a final newline\ngpgsig literal body content'
        } >"$name.commit"
        commit_id=$(git -C "$kind" hash-object -t commit -w --stdin <"$name.commit")
        GIX_SIGNATURE_PAYLOAD="$PWD/$name.payload" git -C "$kind" \
            -c gpg.program="$PWD/record-signature-input" verify-commit "$commit_id" \
            >"$name.output" 2>&1 || :
        test -s "$name.payload"
    done
done
