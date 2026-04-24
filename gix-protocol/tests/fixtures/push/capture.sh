#!/usr/bin/env bash
# gix-protocol/tests/fixtures/push/capture.sh
#
# Record pkt-line byte streams for push fixtures. Produces two files per
# scenario: `<name>.c2s.bin` (client → server) and `<name>.s2c.bin`
# (server → client).
#
# Layout on disk:
#   <name>.c2s.bin   — client → server (command list + pack)
#   <name>.s2c.bin   — server → client (ref advertisement + report)
#
# Regenerate:
#   bash gix-protocol/tests/fixtures/push/capture.sh
#
# Requires: git >= 2.30.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
out="$here"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

tee_receive_pack="$work/tee-receive-pack.sh"
cat >"$tee_receive_pack" <<'WRAPPER'
#!/usr/bin/env bash
set -euo pipefail
c2s="${CAPTURE_C2S:?CAPTURE_C2S must be set}"
s2c="${CAPTURE_S2C:?CAPTURE_S2C must be set}"
tee "$c2s" | git-receive-pack "$@" | tee "$s2c"
WRAPPER
chmod +x "$tee_receive_pack"

record() {
  local name="$1"; shift
  local src="$1"; shift
  local dst="$1"; shift
  CAPTURE_C2S="$out/$name.c2s.bin" \
  CAPTURE_S2C="$out/$name.s2c.bin" \
  git -C "$src" push --receive-pack="$tee_receive_pack" "file://$dst" "$@"
}

# ── fixture: empty-to-new-branch ────────────────────────────────────────────
src="$work/empty-src"
dst="$work/empty-dst.git"
git init -q -b main "$src"
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "initial"
git init -q --bare "$dst"
record "empty-to-new-branch" "$src" "$dst" main

# ── fixture: fast-forward ───────────────────────────────────────────────────
src="$work/ff-src"
dst="$work/ff-dst.git"
git init -q -b main "$src"
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "c1"
git init -q --bare "$dst"
git -C "$src" push "file://$dst" main   # seed
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "c2"
record "fast-forward" "$src" "$dst" main

# ── fixture: delete-ref ─────────────────────────────────────────────────────
src="$work/del-src"
dst="$work/del-dst.git"
git init -q -b main "$src"
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "init"
git -C "$src" branch gone
git init -q --bare "$dst"
git -C "$src" push "file://$dst" main gone   # seed both
record "delete-ref" "$src" "$dst" ":gone"

# ── fixture: non-ff-rejected ────────────────────────────────────────────────
# Scenario: server advances main to c2, client still has c1 and tries to push
# the original c1 as if it were new (rewind scenario).
src="$work/nff-src"
dst="$work/nff-dst.git"
git init -q -b main "$src"
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "c1"
c1_oid=$(git -C "$src" rev-parse HEAD)
git init -q --bare "$dst"
# Seed the server with c1.
git -C "$src" push "file://$dst" main
# Advance the server's main to a new commit (c2) by creating it in src and pushing.
git -C "$src" -c user.email=x@x -c user.name=x -c commit.gpgsign=false -c tag.gpgsign=false commit --allow-empty -qm "c2"
git -C "$src" push "file://$dst" main
# Now rewind the client's main back to c1 (simulating a force-push or reset scenario).
git -C "$src" reset -q --hard "$c1_oid"
# Client is at c1, server is at c2. Pushing c1 as main is non-ff (attempt to rewind).
set +e
record "non-ff-rejected" "$src" "$dst" main
set -e

echo "captured fixtures in $out"
ls -la "$out"/*.bin
