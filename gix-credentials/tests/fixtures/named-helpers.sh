#!/usr/bin/env bash
set -eu -o pipefail

# Helpers in both search locations distinguish Git's exec-path precedence from PATH lookup.
# A shebang-less executable also exercises the Unix execution fallback.
git init -q repo
mkdir 'exec path' 'path helpers' 'not executable'
for definition in 'exec path:probe:exec' 'path helpers:probe:path' 'path helpers:path-only:path' 'exec path:no-shebang:plain' 'exec path:store:builtin-shadow' 'exec path:cache:builtin-shadow' 'exec path:cache--daemon:builtin-shadow'; do
    IFS=: read -r directory name origin <<<"$definition"
    file="$directory/git-credential-$name"
    if test "$name" != no-shebang; then
        printf '#!/bin/sh\n' >"$file"
    else
        : >"$file"
    fi
    printf "printf 'origin=%s\\n'\n" "$origin" >>"$file"
    cat >>"$file" <<'EOF'
printf 'argument=<%s>\n' "$@"
case "$PATH" in
    "$GIT_EXEC_PATH":*) ;;
    *) printf 'exec path is not first in PATH\n' >&2; exit 92 ;;
esac
EOF
    chmod +x "$file"
done
printf '#!/bin/sh\nexit 93\n' >'not executable/git-credential-path-only'
