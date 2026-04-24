# Minimal smoke test for tests/parity.sh + expect_parity helper.
# Not tied to a real command — just exercises the wiring.
# Delete or keep as documentation, user's choice.

title "parity smoke test"
(sandbox
  (with "a trivial git invocation"
    it "git --version matches itself (sanity)" && {
      # expect_parity is designed for git ↔ gix; for smoke we fake both
      # by invoking git on both sides via $exe_plumbing override.
      # Here we only verify the helper's exit-code comparison path works.
      git_out="$(git --version 2>&1)"
      test -n "$git_out" || fail "git not callable"
      echo "${GREEN} - OK (git is callable: $git_out)"
    }
  )

  (with "expect_parity exit-code match path"
    # Run a command that exits 0 on both sides. Use `git --version` vs
    # `git --version` by pointing $exe_plumbing at /usr/bin/env git for
    # this smoke only.
    saved_exe_plumbing="$exe_plumbing"
    exe_plumbing="$(command -v git)"
    it "same command in both slots returns OK" && {
      expect_parity effect -- --version
    }
    it "OK message includes active hash" && {
      export GIX_TEST_FIXTURE_HASH=sha256
      out="$(expect_parity effect -- --version 2>&1 || true)"
      unset GIX_TEST_FIXTURE_HASH
      if [[ "$out" != *"hash=sha256"* ]]; then
        fail "expect_parity OK line missing hash tag; got: $out"
      fi
      echo 1>&2 "${GREEN} - OK (hash tag present)"
    }
    exe_plumbing="$saved_exe_plumbing"
  )

  (with "only_for_hash guard"
    saved_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"

    export GIX_TEST_FIXTURE_HASH=sha1
    it "runs under sha1 when coverage=dual" && {
      only_for_hash dual && expect_run 0 true
    }
    it "runs under sha1 when coverage=sha1-only" && {
      only_for_hash sha1-only && expect_run 0 true
    }

    export GIX_TEST_FIXTURE_HASH=sha256
    it "runs under sha256 when coverage=dual" && {
      only_for_hash dual && expect_run 0 true
    }
    it "skips under sha256 when coverage=sha1-only" && {
      if only_for_hash sha1-only; then
        fail "expected only_for_hash sha1-only to return non-zero under sha256"
      fi
      echo 1>&2 "${GREEN} - OK (skip path taken)"
    }

    GIX_TEST_FIXTURE_HASH="$saved_hash"
  )

  (with "fixture helpers respect GIX_TEST_FIXTURE_HASH"
    saved_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"

    export GIX_TEST_FIXTURE_HASH=sha256
    (small-repo-in-sandbox
      it "small-repo-in-sandbox honors sha256" && {
        format="$(git config --local extensions.objectformat 2>/dev/null || echo sha1)"
        if [[ "$format" != "sha256" ]]; then
          fail "expected sha256, got $format"
        fi
        echo 1>&2 "${GREEN} - OK (small-repo-in-sandbox → sha256)"
      }
    )

    bare_target="$(mktemp -d)/bare.git"
    bare-repo-with-remotes "$bare_target" origin /tmp/whatever
    (cd "$bare_target"
      it "bare-repo-with-remotes honors sha256" && {
        format="$(git config --local extensions.objectformat 2>/dev/null || echo sha1)"
        if [[ "$format" != "sha256" ]]; then
          fail "expected sha256 in bare repo, got $format"
        fi
        echo 1>&2 "${GREEN} - OK (bare-repo-with-remotes → sha256)"
      }
    )
    rm -rf "$(dirname "$bare_target")"

    GIX_TEST_FIXTURE_HASH="$saved_hash"
  )

  (with "expect_parity_reset runs setup per-binary"
    # Prove: the named setup-fn fires once before git runs and once before
    # gix (stand-in = git again) runs. Without reset, stateful fixture ops
    # like `git fetch --unshallow` can't be asserted back-to-back.
    SEEN_FILE="$(mktemp)"
    function _setup_mark_and_init() {
      echo x >> "$SEEN_FILE"
      git init -q
      git config commit.gpgsign false
      git -c user.email=x@x -c user.name=x commit -q --allow-empty -m init
    }
    saved_exe_plumbing="$exe_plumbing"
    exe_plumbing="$(command -v git)"
    it "setup fires once per binary invocation" && {
      expect_parity_reset _setup_mark_and_init effect -- status
      count="$(wc -l < "$SEEN_FILE" | tr -d ' ')"
      if [[ "$count" != "2" ]]; then
        fail "expected setup to run 2x (once per binary), got $count"
      fi
      echo 1>&2 "${GREEN} - OK (setup ran per-binary, count=$count)"
    }
    it "byte-level divergence in reset mode is reported and FAILs" && {
      # Craft a setup that produces different content for each invocation
      # by reading $SEEN_FILE's current line count, so git and gix see
      # different statuses and bytes-mode should FAIL. We toggle set -e
      # off briefly so the intended FAIL doesn't abort the suite.
      > "$SEEN_FILE"
      function _setup_divergent() {
        echo x >> "$SEEN_FILE"
        git init -q
        git config commit.gpgsign false
        local n
        n="$(wc -l < "$SEEN_FILE" | tr -d ' ')"
        touch "file-$n"
      }
      set +e
      expect_parity_reset _setup_divergent bytes -- status --porcelain 2>/dev/null
      rc=$?
      set -e
      if [[ "$rc" == "0" ]]; then
        fail "expected bytes-mode FAIL (setup is divergent), got OK"
      fi
      echo 1>&2 "${GREEN} - OK (bytes-mode divergence surfaces, rc=$rc)"
    }
    rm -f "$SEEN_FILE"
    exe_plumbing="$saved_exe_plumbing"
  )

  (with "parity.sh runs each file under both hash kinds"
    it "GIX_TEST_FIXTURE_HASH is set by the runner" && {
      if [[ -z "${GIX_TEST_FIXTURE_HASH:-}" ]]; then
        fail "GIX_TEST_FIXTURE_HASH not set — parity.sh should set it per iteration"
      fi
      echo 1>&2 "${GREEN} - OK (runner set GIX_TEST_FIXTURE_HASH=$GIX_TEST_FIXTURE_HASH)"
    }
  )

  (with "compat_effect emits grep-able marker on green rows"
    # Pin $exe_plumbing to git so effect-mode parity trivially passes;
    # we're asserting the marker shape, not semantics.
    saved_exe_plumbing="$exe_plumbing"
    exe_plumbing="$(command -v git)"
    it "OK row carries [compat] <reason> on stderr" && {
      out="$(compat_effect "diff emission deferred under -v" -- --version 2>&1)"
      if [[ "$out" != *"[compat] diff emission deferred under -v"* ]]; then
        fail "compat_effect missing [compat] marker; got: $out"
      fi
      if [[ "$out" != *"- OK"* ]]; then
        fail "compat_effect did not surface the underlying OK line; got: $out"
      fi
      echo 1>&2 "${GREEN} - OK (marker + OK both present)"
    }
    it "propagates FAIL when underlying parity diverges" && {
      # Force a divergence by pointing $exe_plumbing at /bin/true, which
      # returns exit 0 with no output vs git --version's versioned text.
      # effect mode only cares about exit codes, so this still passes;
      # use -c nosuch.key=... -c to force git to fail differently.
      set +e
      exe_plumbing="/bin/false"
      compat_effect "forced divergence" -- --version >/dev/null 2>&1
      rc=$?
      exe_plumbing="$(command -v git)"
      set -e
      if [[ "$rc" == "0" ]]; then
        fail "expected compat_effect to FAIL when exit codes diverge, got rc=0"
      fi
      echo 1>&2 "${GREEN} - OK (FAIL propagates, rc=$rc)"
    }
    exe_plumbing="$saved_exe_plumbing"
  )
)
