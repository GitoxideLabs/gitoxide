# Reftable performance baseline

This is an indicative development baseline, not a release threshold. It makes
the initial cost profile reproducible and gives later changes a stable set of
operations to compare.

## Environment

- Date: 2026-08-28
- Host: Apple M4 Pro (`arm64`)
- Operating system: macOS 26.5.2 (25F84)
- Rust: `rustc 1.92.0 (ded5c06cf 2025-12-08) (Homebrew)`
- Git: `2.50.1 (Apple Git-155)`
- Command: `cargo bench -p gix-reftable --all-features --bench operations -- --noplot`

The host was also running unrelated CPU-heavy jobs, so the final emitted
benchmark binary was launched with macOS `taskpolicy -a` to use normal
application scheduling and keep the confidence intervals stable. This changes
scheduling policy, not the benchmark workload.

Criterion's point estimates and 95% confidence intervals were:

| Operation | Records | Estimate | 95% confidence interval |
|---|---:|---:|---:|
| exact lookup | 100 | 38.769 ns | 38.729–38.811 ns |
| full iteration | 100 | 31.398 ns | 31.328–31.467 ns |
| indexed prefix iteration (50 matches) | 100 | 67.862 ns | 67.344–68.324 ns |
| exact lookup | 10,000 | 65.633 ns | 65.547–65.716 ns |
| full iteration | 10,000 | 2.6179 µs | 2.6150–2.6209 µs |
| indexed prefix iteration (5,000 matches) | 10,000 | 5.4113 µs | 5.4051–5.4175 µs |
| deterministic write | 100 | 31.444 µs | 31.330–31.557 µs |
| deterministic write | 10,000 | 2.5241 ms | 2.5180–2.5304 ms |
| compact eight generations | 100 | 23.089 ms | 22.776–23.419 ms |
| compact eight generations | 10,000 | 37.217 ms | 36.975–37.456 ms |

The read benchmarks parse one table before timing. Exact and prefix lookup use
the table's retained indexes; iteration consumes every returned record. The
write benchmark includes encoding but no filesystem I/O. The compaction
benchmark includes temporary-file creation, durability operations, atomic
publication, and obsolete-member cleanup. Stack construction and recursive
temporary-directory teardown both occur outside the timed section.
