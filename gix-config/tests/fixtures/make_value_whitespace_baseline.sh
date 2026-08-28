#!/usr/bin/env bash
set -eu -o pipefail

# Record each description, config, and Git value as a NUL-terminated triple in `baseline.git`. The Rust test compares
# these records without spawning Git. `%b` keeps control-byte cases readable here.
unset GIT_CONFIG_COUNT
: >baseline.git

baseline() {
  local description="$1"
  local config="$2"
  printf '%s\0%b\0' "$description" "$config" >>baseline.git
  printf '%b' "$config" | git config --file - --null --get core.k >>baseline.git
}

baseline starts-on-tab '[core]\n\tk = \\\n\tabc\n'
baseline starts-on-spaces '[core]\n\tk = \\\n    abc\n'
baseline no-space-before-continuation '[core]\n\tk =\\\n\t\tabc\n'
baseline empty-continuation '[core]\n\tk = \\\n\t\\\n\tabc\n'
baseline empty-quotes-before-continuation '[core]\n\tk = ""\\\n  abc\n'
baseline empty-quotes-inside-chunk '[core]\n\tk = ""  x\n'
baseline crlf '[core]\r\n\tk = \\\r\n\tabc\r\n'
baseline standalone-cr '[core]\n\tk = \\\n\r \tabc\n'
baseline indentation-after-content '[core]\n\tk = abc\\\n\tdef\n'
baseline quoted-continuation '[core]\n\tk = "\\\n  abc"\n'
baseline vertical-tab-content '[core]\n\tk = \\\n\v\\\n  a\n'
baseline form-feed-content '[core]\n\tk = \\\n\f\\\n  a\n'
