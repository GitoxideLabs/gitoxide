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

- 2 commits contributed to the release over the course of 16 calendar days.
- 1 commit was understood as [conventional](https://www.conventionalcommits.org).
- 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

- **Uncategorized**
  - Merge pull request #3165 from GitoxideLabs/encoding ([`1b025f7`](https://github.com/GitoxideLabs/gitoxide/commit/1b025f7cda5411be7f77f01a911881431a6aa737))
  - Add strict streaming character conversion for Git ([`2effc16`](https://github.com/GitoxideLabs/gitoxide/commit/2effc1662becd2df36dfbbb566349920943bfb7e))

</details>
