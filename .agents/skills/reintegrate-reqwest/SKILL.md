---
name: reintegrate-reqwest
description: "Integrate upstream reqwest trunk updates or selected patches into the vendored gix-reqwest crate, preserving local fixes, authorship, and commit-trailer provenance. Use for requested reqwest updates; editing these instructions alone does not start an integration."
---

# Reintegrate reqwest

`gix-reqwest/` is a maintained fork of <https://github.com/seanmonstar/reqwest>.
Its purpose is to carry generally useful HTTP fixes needed by Gitoxide until
upstream catches up. Git-specific URL, configuration, and credential-helper
policy belongs in `gix-transport` or `gix`, not in this HTTP client.

## Establish the source and checkpoint

1. Read the root and `gix-reqwest/AGENTS.md` instructions. Inspect the current
   branch, status, and recent fork commits. Preserve unrelated work. Work and
   commit locally; pushing or opening a PR requires a user request.
2. Locate the user's upstream reqwest clone or worktree and verify its remote
   URL. A Gitoxide checkout is the destination, never the source. The original
   import used `/Users/byron/dev/github.com/seanmonstar/reqwest.native-redirect-fix`;
   do not assume that path or its patch branch still represents upstream trunk.
3. Read provenance from reachable commit messages, without a path filter or
   `--first-parent`: empty checkpoints and commits merged through a side branch
   must remain visible.

   ```bash
   git log --topo-order --extended-regexp --format='%H%n%B' \
       --grep='^Reqwest-(Upstream|Patch|PR): ' HEAD
   ```

   `Reqwest-Upstream` records a complete upstream integration checkpoint.
   `Reqwest-Patch` records an individually imported commit, possibly from a
   local patch branch or unmerged PR; it does **not** advance the trunk baseline.
   `Reqwest-PR` records the source PR URL when known. Resolve full hashes in
   the source clone. For ports, read these local fields before `Original source
   message:`; the original message may contain its own provenance. Git only
   recognizes a final trailer block, so `%(trailers)` cannot read local fields
   placed before the verbatim original message. Original upstream hashes are
   also retained in imported commit bodies.
4. Determine the requested target trunk commit or release. Verify checkpoint
   ancestry with `git merge-base --is-ancestor` in the source clone. If merged
   Gitoxide branches contain multiple checkpoints, choose the furthest
   integrated ancestor on upstream trunk, not the most recent author date.
   Investigate divergent checkpoints or missing source objects before choosing
   a baseline. Review the complete source range with `git log` and `git diff`.

## Merge while preserving the fork

Treat the recorded upstream snapshot as the merge base, the current vendored
files as ours, and the target upstream files as theirs. Use temporary source
snapshots outside the checkout and three-way patches/merges under
`gix-reqwest/`. Never overwrite the vendored directory with the upstream tip:
that would silently discard outstanding fixes. Handle new, removed, and
renamed files as well as changed files; never import `.git`, build artifacts,
or the source worktree's ignored lockfile.

Apply the same mechanical adaptations to both upstream sides before merging:

- Rust crate paths and `extern crate` references use `gix_reqwest`; examples
  depend on the `gix-reqwest` package. Keep upstream URLs and historical
  attributions intact. Avoid changing already-renamed paths a second time.
- Preserve the package name, independent version, Gitoxide repository and
  documentation URLs, fork explanation, and all upstream authors with
  Sebastian Thiel added. Preserve upstream licenses and copyright notices.
- Preserve the fork's QA metadata, explicit Clippy style exceptions, and
  100-column `rustfmt.toml`. Feature names and `reqwest_unstable` retain their
  upstream spelling. Keep the upstream CI and Dependabot configurations
  omitted from the nested `.github/`. Preserve workspace QA and the root
  `.github/workflows/gix-reqwest.yml`; compare its configurations with upstream
  CI when importing test or feature changes to avoid losing coverage.
  Keep the standalone WASM example's empty `[workspace]` table and
  its macro-only `wasm-bindgen-futures` Machete exception.
- Keep the integration skill and `AGENTS.md`. Follow the root prohibition on
  changing `CHANGELOG.md`; record omitted upstream changelog updates in the
  integration commit body rather than importing generated release notes.

Compare each outstanding local patch with upstream by behavior and diff,
including its regression tests. Drop a local delta only when upstream covers
it. Reconcile different implementations; a renamed function or similar commit
title is not proof of equivalence. Retain tests that protect Gitoxide's needs.

Keep commits self-contained. When importing dependent fixes, include necessary
compile adaptations in the introducing commit and explain any later source
commit that becomes empty. Verify individual ports' authors and dates against the original
source Git objects, following any transplant's original-commit reference rather
than copying its actor blindly. Preserve agent authorship for agent-written fixes,
tests, and integration-only changes. Use scoped Gitoxide commit titles and
describe why each retained delta is needed.

## Validate and record completion

Run the fork's configured QA checks/tests for affected features and the
transport's redirect/proxy regressions. Reuse build caches. Touch extracted
Rust files or otherwise ensure Cargo sees their new contents: `git archive`
timestamps can be older than artifacts built from another source worktree.

Useful focused commands, run from the Gitoxide root:

```bash
cargo fmt -p gix-reqwest -p gix-transport -- --check
cargo nextest run --locked -p gix-reqwest --no-default-features \
    --features blocking,http2,socks,rustls,cookies,charset,query,form,json,multipart,stream,hickory-dns,gzip,brotli,zstd,deflate \
    -E 'not binary(badssl) & not test(test_tls_info)'
cargo test --locked -p gix-reqwest \
    --features blocking,socks,native-tls --lib --test proxy --test redirect
GIX_TEST_IGNORE_ARCHIVES=1 cargo test --locked -p gix-transport \
    --features http-client-reqwest-rust-tls,http-client-insecure-credentials \
    --test blocking-transport --test blocking-transport-http-reqwest
GIX_TEST_IGNORE_ARCHIVES=1 cargo test --locked -p gix-transport \
    --features http-client-reqwest,http-client-insecure-credentials \
    --test blocking-transport --test blocking-transport-http-reqwest
```

Also exercise WASM, HTTP/3 (`RUSTFLAGS="--cfg reqwest_unstable"`), DNS, and
documentation configurations when their code or dependencies change. Use
loopback regressions for credentials, custom proxy headers, redirect payloads,
Referer opt-out, and one request timeout budget; distinguish skipped live
Internet tests and unavailable platforms from passing checks. Update the
workspace lockfile for dependency changes and verify packaging metadata.

For a complete trunk update, append exactly one full target commit hash:

```text
Reqwest-Upstream: <full upstream trunk commit>
```

Record that checkpoint only after the entire range has been reconciled and
validated. Explain intentionally omitted changes and remaining local deltas
in the commit body. For an individual patch, use `Reqwest-Patch: <full source
commit>` instead and add `Reqwest-PR: <source PR URL>` when known. Put all local
notes, validation, and provenance before `Original source message:`, followed by
the verbatim source message with nothing appended. A tested, already-covered
update may use an empty checkpoint commit with its justification. Do not invent
a parallel revision metadata file.

Finish with a clean task-owned diff and report the old/new checkpoint, imported
patches, retained deltas, commit hashes, and checks. When considering a return
to upstream, require a released version that covers the retained fixes and
passes the same regressions. Coordinate the public client/builder type change
and dependency migration in one commit; the transport re-export gives callers
a single documented place to obtain the supported types.
