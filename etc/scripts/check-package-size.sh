#!/usr/bin/env bash

set -eu -o pipefail

function enter () {
  local dir="${1:?need directory to enter}"
  printf '  in %s \t→\t' "$dir"
  cd -- "$dir"
}

function indent () {
  "$@" | grep -F 'package size' | while IFS= read -r line; do
    echo "     $line"
  done
}

# Generous budgets catch accidental package bloat without flagging routine source growth.
echo 'in root: gitoxide CLI'
(enter gix-fsck && indent cargo diet -n --package-size-limit 50KB)
(enter gix-actor && indent cargo diet -n --package-size-limit 50KB)
(enter gix-archive && indent cargo diet -n --package-size-limit 50KB)
(enter gix-worktree-stream && indent cargo diet -n --package-size-limit 50KB)
(enter gix-utils && indent cargo diet -n --package-size-limit 50KB)
(enter gix-fs && indent cargo diet -n --package-size-limit 50KB)
(enter gix-pathspec && indent cargo diet -n --package-size-limit 50KB)
(enter gix-refspec && indent cargo diet -n --package-size-limit 50KB)
(enter gix-path && indent cargo diet -n --package-size-limit 100KB)
(enter gix-attributes && indent cargo diet -n --package-size-limit 50KB)
(enter gix-discover && indent cargo diet -n --package-size-limit 50KB)
(enter gix-index && indent cargo diet -n --package-size-limit 125KB)
(enter gix-worktree && indent cargo diet -n --package-size-limit 75KB)
(enter gix-quote && indent cargo diet -n --package-size-limit 50KB)
(enter gix-revision && indent cargo diet -n --package-size-limit 50KB)
(enter gix-bitmap && indent cargo diet -n --package-size-limit 50KB)
(enter gix-tempfile && indent cargo diet -n --package-size-limit 50KB)
(enter gix-lock && indent cargo diet -n --package-size-limit 50KB)
(enter gix-config && indent cargo diet -n --package-size-limit 175KB)
(enter gix-config-value && indent cargo diet -n --package-size-limit 50KB)
(enter gix-command && indent cargo diet -n --package-size-limit 50KB)
(enter gix-hash && indent cargo diet -n --package-size-limit 50KB)
(enter gix-chunk && indent cargo diet -n --package-size-limit 50KB)
(enter gix-parallel && indent cargo diet -n --package-size-limit 75KB)
(enter gix-ref && indent cargo diet -n --package-size-limit 150KB)
(enter gix-diff && indent cargo diet -n --package-size-limit 125KB)
(enter gix-traverse && indent cargo diet -n --package-size-limit 50KB)
(enter gix-url && indent cargo diet -n --package-size-limit 75KB)
(enter gix-validate && indent cargo diet -n --package-size-limit 50KB)
(enter gix-date && indent cargo diet -n --package-size-limit 50KB)
(enter gix-hashtable && indent cargo diet -n --package-size-limit 50KB)
(enter gix-filter && indent cargo diet -n --package-size-limit 75KB)
(enter gix-status && indent cargo diet -n --package-size-limit 75KB)
(enter gix-sec && indent cargo diet -n --package-size-limit 50KB)
(enter gix-credentials && indent cargo diet -n --package-size-limit 50KB)
(enter gix-prompt && indent cargo diet -n --package-size-limit 50KB)
(enter gix-object && indent cargo diet -n --package-size-limit 125KB)
(enter gix-commitgraph && indent cargo diet -n --package-size-limit 50KB)
(enter gix-pack && indent cargo diet -n --package-size-limit 225KB)
(enter gix-odb && indent cargo diet -n --package-size-limit 150KB)
(enter gix-protocol && indent cargo diet -n --package-size-limit 100KB)
(enter gix-packetline && indent cargo diet -n --package-size-limit 50KB)
(enter gix && indent cargo diet -n --package-size-limit 700KB)
(enter gix-transport && indent cargo diet -n --package-size-limit 150KB)
(enter gitoxide-core && indent cargo diet -n --package-size-limit 225KB)
