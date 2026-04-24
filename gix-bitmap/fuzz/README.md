# Fuzz Testing

This directory contains fuzz tests for `gix-bitmap` using [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html).

## Running Fuzz Tests

### Prerequisites
- Nightly Rust toolchain: `rustup install nightly`
- cargo-fuzz: `cargo install cargo-fuzz`

### Targets
- `ewah`: exercises EWAH bitmap decoding.

### Running the target
```bash
cargo +nightly fuzz run ewah -- -max_total_time=60
```

## Artifact Reproducers

The `artifacts/ewah` directory contains minimized reproducer inputs produced by libFuzzer or Google OSS-Fuzz.

The integration test module `fuzzed` reads every regular file in that directory and runs it through the parser under test. This lets `cargo test -p gix-bitmap fuzzed` reproduce known fuzz failures without starting a fuzzing session.

When OSS-Fuzz reports a new failure, place the minimized testcase in `artifacts/ewah`, then run `cargo test -p gix-bitmap fuzzed`. To confirm it against the original harness, run `cargo fuzz run ewah artifacts/ewah/<reproducer>` from this directory.
