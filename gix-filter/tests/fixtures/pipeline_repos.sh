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

# Git bypasses conversion for UTF-8, even when the bytes are not valid UTF-8.
(mkdir utf8 && cd utf8
  echo '* -text working-tree-encoding=UTF-8' >.gitattributes
)

# Verify the attribute parser uses iconv labels rather than WHATWG aliases.
(mkdir latin1 && cd latin1
  echo '* -text working-tree-encoding=latin-1' >.gitattributes
)

# An explicit round-trip check may reject a non-native UTF-16 byte order.
(mkdir utf16 && cd utf16
  echo '* -text working-tree-encoding=UTF-16' >.gitattributes
)
