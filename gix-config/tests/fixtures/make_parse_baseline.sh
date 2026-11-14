#!/usr/bin/env bash
set -eu -o pipefail

# Bare keys end at the line boundary. Git must reject extra tokens instead of reading them as
# additional settings, which could otherwise override a URL in untrusted `.gitmodules` content.
# Record description, config, exit status, and winning URL as NUL-terminated fields.
unset GIT_CONFIG_COUNT
: >baseline.git

baseline() {
  local expected="$1" description="$2" config="$3" status=0
  printf '%b' "$config" | git config --file - --list >/dev/null 2>&1 || status=$?
  test "$status" = "$expected"
  printf '%s\0%b\0%s\0' "$description" "$config" "$status" >>baseline.git
  if test "$status" = 0; then
    printf '%b' "$config" | git config --file - --null --get submodule.module.url >>baseline.git
  else
    printf '\0' >>baseline.git
  fi
}

module='[submodule "module"]\nurl = ../good\n'
baseline 128 extra-setting "${module}x url = ../evil\n"
baseline 128 header-extra-setting '[submodule "module"] x url = ../evil\n'
baseline 128 attached-hash "${module}br#anch = main\n"
baseline 128 spaced-hash "${module}br #comment\n"
baseline 128 attached-semicolon "${module}br;comment\n"
baseline 128 spaced-semicolon "${module}br ;comment\n"
baseline 128 following-header "${module}x[submodule \"module\"] url = ../evil\n"
baseline 128 following-quote "${module}x\"url\" = ../evil\n"
baseline 128 following-dot "${module}x.url = ../evil\n"
baseline 128 bare-continuation "${module}x\\\nurl = ../evil\n"
baseline 128 standalone-cr "${module}x\r"
baseline 128 vertical-tab "${module}x\v\n"
baseline 128 form-feed "${module}x\f\n"
baseline 0 implicit-eof "${module}shallow"
baseline 0 implicit-trailing-space "${module}shallow \t"
baseline 0 implicit-lf "${module}shallow\n"
baseline 0 implicit-crlf "${module}shallow\r\n"
baseline 0 implicit-with-comment-next-line "${module}shallow\n #comment\n"
baseline 0 header-setting '[submodule "module"] url = ../good\n'
baseline 0 header-implicit '[submodule "module"] shallow\nurl = ../good\n'
baseline 0 adjacent-headers '[unused][submodule "module"]url = ../good\n'
baseline 0 explicit-empty-comment "${module}branch = #comment\n"
baseline 0 explicit-attached-comment "${module}url = ../good#comment\n"
baseline 0 explicit-quoted-comment "${module}url = \"../good\";comment\n"
baseline 0 explicit-continuation "${module}url = ../\\\ngood\n"
