# Fuzz Testing

This directory contains fuzz tests for `gix-index` using [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html).

## Running Fuzz Tests

### Prerequisites
- Nightly Rust toolchain: `rustup install nightly`
- cargo-fuzz: `cargo install cargo-fuzz`

### Targets
- `index_file`: exercises index decoding, entry/path access, extension access, verification, and round-trip serialization.

### Running the target
```bash
cargo +nightly fuzz run index_file -- -max_total_time=60
```

## Artifact Reproducers

The `artifacts/index_file` directory contains minimized reproducer inputs produced by libFuzzer or Google OSS-Fuzz.

The integration test module `fuzzed` reads every regular file in that directory and runs it through the parser under test. This lets `cargo test -p gix-index-tests fuzzed` reproduce known fuzz failures without starting a fuzzing session.

When OSS-Fuzz reports a new failure, place the minimized testcase in `artifacts/index_file`, then run `cargo test -p gix-index-tests fuzzed`. To confirm it against the original harness, run `cargo fuzz run index_file artifacts/index_file/<reproducer>` from this directory.
