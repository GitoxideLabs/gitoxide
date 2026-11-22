# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## v0.1.0 (2026-11-22)

### New Features

- <csr-id-2effc1662becd2df36dfbbb566349920943bfb7e/> add strict streaming character conversion for Git
  I looked at the API through `cargo doc`, and rubber stamped the code
  and tests.
  Slightly reorganized the module layout.
  
  Here are some benchmarks (agent):
  
  gix_over_iconv is a time ratio: 0.25 means one-quarter the execution time, or roughly 4× faster.
  
  Inputs       Median time ratio    Approximate speedup
  ━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━  ━━━━━━━━━━━━━━━━━━━━━
  Small                    0.558                  1.79×
  ───────────  ───────────────────  ─────────────────────
  Large                    0.749                  1.33×
  ───────────  ───────────────────  ─────────────────────
  All cases                0.621                  1.61×
  
  Small inputs generally benefit more, suggesting lower fixed overhead. The gains persist for large inputs, so conversion
  throughput also improves.
  
  The strongest large-input results are:
  
  • EUC-JP encoding: 9.75×, reducing 20.85 ms to 2.14 ms.
  • ISO-2022-JP encoding: 7.52×, reducing 20.75 ms to 2.76 ms.
  • UTF-16LE ASCII encoding: 3.89×.
  • Latin-1 ASCII decoding: 3.75×.
  
  The Japanese encoding gains remain almost identical across input sizes, which points to a substantial advantage in the
  conversion path itself. They are direction-specific: EUC-JP decoding gains only about 8%.
  
  The narrowest margins are large-input Shift-JIS encoding, CP932 encoding, EUC-JP decoding, and Big5 decoding, at roughly
  7–8% faster. Those are the cases most worth watching across machines and compiler versions.
  
  These results support the “no slower than iconv” goal for the measured buffer conversions. The typical large-input
  improvement is around 33% higher throughput, with several much larger wins. They establish performance for these corpora
  and this machine; streaming performance needs its own measurements.
  
  <!-- agent -->
  Gitoxide needs Git's native encoding semantics rather than the WHATWG
  interpretations provided by `encoding_rs`. In particular, ASCII and Latin-1
  must remain distinct from Windows-1252, Shift-JIS from CP932, and UTF-16/32
  must support encoding as well as decoding.
  
  Add a pure-Rust converter with 55 named encodings, an allocation-free
  incremental API, bounded `Read` and `Write` adapters, and reusable-buffer
  convenience functions. Reject malformed input and unrepresentable characters
  with source-byte context. Preserve pending composing characters at EOF even
  where Git's omitted `iconv` `flush` would lose them.
  
  Generate portable numeric mappings from GNU libc `iconv`, with regeneration
  tools and explicit documentation of differences from other iconv libraries.
  Keep Git's worktree policies separate from the raw codecs.
  
  Validate with isolated Git baselines, every Unicode scalar, exhaustive legacy
  byte sequences, Unicode validity comparisons, and adversarial streaming I/O.
  Provide in-process `iconv` benchmarks for both allocated buffers and reusable
  streaming converters; their strict per-case gate avoids hiding regressions
  behind an aggregate result.

### Commit Statistics

<csr-read-only-do-not-edit/>

- 3 commits contributed to the release over the course of 16 calendar days.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Release gix-error v0.5.3, gix-macros v0.2.2, gix-date v0.19.0, gix-actor v0.45.0, gix-trace v0.3.2, gix-validate v0.13.2, gix-path v0.14.2, gix-utils v0.5.3, gix-quote v0.10.2, gix-command v0.12.2, gix-hash v0.29.0, gix-hashtable v0.19.0, gix-parallel v0.50.2, gix-fs v1.0.0, gix-tempfile v27.0.0, gix-object v0.67.0, gix-glob v0.30.0, gix-attributes v0.38.0, gix-zlib v0.4.1, gix-encoding v0.1.0, gix-packetline v0.24.2, gix-filter v0.37.0, gix-chunk v0.10.2, gix-commitgraph v0.43.0, gix-revwalk v0.39.0, gix-traverse v0.65.0, gix-worktree-stream v0.40.0, gix-archive v0.40.0, gix-bitmap v0.6.2, gix-lock v27.0.0, gix-index v0.59.0, gix-config-value v0.21.2, gix-pathspec v0.23.0, gix-ignore v0.25.0, gix-worktree v0.60.0, gix-imara-diff v0.3.4, gix-diff v0.71.0, gix-blame v0.21.0, gix-ref v0.70.0, gix-sec v0.17.0, gix-config v0.63.0, gix-prompt v0.19.2, gix-url v0.41.0, gix-credentials v0.43.0, gix-discover v0.58.0, gix-dir v0.33.0, gix-fsck v0.28.0, gix-notify v0.1.0, gix-fsmonitor v0.1.0, gix-mailmap v0.36.2, gix-revision v0.53.0, gix-merge v0.24.0, gix-negotiate v0.39.0, gix-note v0.4.0, gix-pack v0.78.0, gix-odb v0.88.0, gix-refspec v0.48.0, gix-shallow v0.16.0, gix-transport v0.63.0, gix-protocol v0.69.0, gix-status v0.38.0, gix-submodule v0.37.0, gix-worktree-state v0.38.0, gix v0.91.0, gitoxide-core v0.65.0, gix-tix v0.7.0, gitoxide v0.62.0, safety bump 44 crates ([`df27410`](https://github.com/GitoxideLabs/gitoxide/commit/df27410adf06a49413eb84b9d6bc0a633ff94b78))
  - Merge pull request #3165 from GitoxideLabs/encoding ([`1b025f7`](https://github.com/GitoxideLabs/gitoxide/commit/1b025f7cda5411be7f77f01a911881431a6aa737))
  - Add strict streaming character conversion for Git ([`2effc16`](https://github.com/GitoxideLabs/gitoxide/commit/2effc1662becd2df36dfbbb566349920943bfb7e))

</details>
