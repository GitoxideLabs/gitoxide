#!/bin/sh
set -eu

# Regenerate passwordless test identities for exercising external Git-compatible signers.
# These keys are fixtures only and must never be used outside the test suite.
cd "$(dirname "$0")"
fixture_tmp="$(mktemp -d)"
trap 'rm -rf "$fixture_tmp"' EXIT

rm -f ssh-private ssh-private.pub ssh-allowed-signers openpgp-secret.asc openpgp-public.asc \
    x509-key.pem x509-cert.pem x509-identity.p12

ssh-keygen -q -t ed25519 -N '' -C signing@example.com -f ssh-private
printf 'signing@example.com %s\n' "$(cat ssh-private.pub)" >ssh-allowed-signers

chmod 700 "$fixture_tmp"
GNUPGHOME="$fixture_tmp" gpg --batch --passphrase '' --quick-generate-key \
    'Gitoxide Signing Fixture <signing@example.com>' ed25519 sign 0
GNUPGHOME="$fixture_tmp" gpg --batch --armor --export-secret-keys \
    'Gitoxide Signing Fixture' >openpgp-secret.asc
GNUPGHOME="$fixture_tmp" gpg --batch --armor --export \
    'Gitoxide Signing Fixture' >openpgp-public.asc

openssl req -new -newkey rsa:2048 -x509 -nodes -days 36500 \
    -subj '/CN=Gitoxide Signing Fixture/emailAddress=signing@example.com' \
    -keyout x509-key.pem -out x509-cert.pem
openssl pkcs12 -export -passout pass: -inkey x509-key.pem -in x509-cert.pem -out x509-identity.p12
