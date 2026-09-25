#!/usr/bin/env bash
set -eu -o pipefail

# Record each case as a mailmap, newline-separated contacts, and a Git-generated baseline.
# The Rust tests only read these files, reusing the fixture across construction and merge checks.

git init -q partial-updates
# Later entries update only the fields they provide, including case-insensitive name-specific
# mappings. An unmatched name must still use the email-only fallback.
cat > partial-updates/.mailmap <<'EOF'
First <first@example.com> <old@example.com>
Second <OLD@example.com>
<last@example.com> <old@example.com>
<other@example.com> <other-old@example.com>
Other <other-old@example.com>
Fallback <fallback@example.com> <shared@example.com>
Earlier <earlier@example.com> Jane <shared@example.com>
<later@example.com> jAnE <SHARED@example.com>
EOF
cat > partial-updates/contacts <<'EOF'
Original <old@example.com>
Original <other-old@example.com>
Jane <shared@example.com>
Unknown <shared@example.com>
EOF
git -C partial-updates check-mailmap --stdin < partial-updates/contacts > partial-updates/baseline.git

git init -q case-folding-with-non-utf8-keys
# Interleave ASCII and non-UTF-8 keys so both email-only and name-specific lookups exercise the
# same ASCII case folding. printf materializes 0xff without embedding invalid UTF-8 in this script.
printf '%b\n' \
  'Alpha <alpha@example.com> <A>' \
  'Byte <byte@example.com> <B\xff>' \
  'Zulu <zulu@example.com> <Z>' \
  'Alpha <alpha@example.com> A <shared>' \
  'Byte <byte@example.com> B\xff <shared>' \
  'Zulu <zulu@example.com> Z <shared>' \
  > case-folding-with-non-utf8-keys/.mailmap
printf '%b\n' \
  'Original <a>' \
  'Original <b\xff>' \
  'Original <z>' \
  'a <shared>' \
  'b\xff <shared>' \
  'z <shared>' \
  > case-folding-with-non-utf8-keys/contacts
git -C case-folding-with-non-utf8-keys check-mailmap --stdin \
  < case-folding-with-non-utf8-keys/contacts > case-folding-with-non-utf8-keys/baseline.git

# Only the inputs and baselines are needed after generation; keep repository metadata out of the archive.
rm -rf partial-updates/.git case-folding-with-non-utf8-keys/.git
