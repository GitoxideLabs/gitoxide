#!/usr/bin/env bash
set -eu -o pipefail

(mkdir all-filters && cd all-filters
  cat <<EOF > .gitattributes
* ident text=auto eol=crlf working-tree-encoding=ISO-8859-1 filter=arrow
EOF
)

(mkdir no-filters && cd no-filters
  touch .gitattributes
)

(mkdir unknown-encoding && cd unknown-encoding
  cat <<EOF > .gitattributes
* text eol=crlf working-tree-encoding=not-an-encoding
EOF
)

(mkdir driver-only && cd driver-only
  cat <<EOF > .gitattributes
* filter=arrow
EOF
)

# Git's plain UTF-16 requires a BOM, whereas explicit endianness forbids either
# BOM. Keep the inputs and Git's results for read-only pipeline regression tests.
(git init utf16 && cd utf16
  export LC_ALL=C
  cat <<EOF > .gitattributes
*.UTF-16 working-tree-encoding=UTF-16
*.UTF-16LE working-tree-encoding=UTF-16LE
*.UTF-16BE working-tree-encoding=UTF-16BE
EOF
  printf 'a\n' > utf8
  printf 'a\0\n\0' > bomless.UTF-16LE
  printf '\0a\0\n' > bomless.UTF-16BE
  printf '\377\376a\0\n\0' > le-bom.UTF-16
  printf '\376\377\0a\0\n' > be-bom.UTF-16
  cp bomless.UTF-16LE bomless.UTF-16

  # Successful conversions, empty stderr, and UTF-8 blobs prove that conversion
  # is available, rather than mistaking missing iconv support for BOM rejection.
  for file in le-bom.UTF-16 be-bom.UTF-16 bomless.UTF-16LE bomless.UTF-16BE; do
    git add "$file" 2>"$file.stderr"
    test ! -s "$file.stderr"
    git --no-pager show ":$file" > "$file.git"
    cmp utf8 "$file.git"
  done

  if git add bomless.UTF-16 2>bomless.UTF-16.stderr; then
    echo 'Git unexpectedly accepted UTF-16 without a BOM' >&2
    exit 1
  fi
  grep -F "BOM is required in 'bomless.UTF-16' if encoded as UTF-16" bomless.UTF-16.stderr

  for encoding in UTF-16LE UTF-16BE; do
    for bom in le be; do
      file="$bom-bom.$encoding"
      cp "$bom-bom.UTF-16" "$file"
      if git add "$file" 2>"$file.stderr"; then
        echo "Git unexpectedly accepted a BOM in $file" >&2
        exit 1
      fi
      grep -F "BOM is prohibited in '$file' if encoded as $encoding" "$file.stderr"
    done
  done
)
