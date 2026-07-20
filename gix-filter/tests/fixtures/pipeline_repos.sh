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

(mkdir driver-only && cd driver-only
  cat <<EOF > .gitattributes
* filter=arrow
EOF
)

# Unknown encodings must not prevent worktree conversion. The surrounding EOL
# and process filters make it observable that only the encoding stage is skipped.
(mkdir unknown-encoding && cd unknown-encoding
  cat <<EOF > .gitattributes
* text eol=crlf working-tree-encoding=definitely-not-an-encoding filter=arrow
EOF
)
