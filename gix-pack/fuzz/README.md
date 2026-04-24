# Fuzz Testing

This directory contains fuzz tests for `gix-pack` using [cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html).

## Running Fuzz Tests

### Prerequisites
- Nightly Rust toolchain: `rustup install nightly`
- cargo-fuzz: `cargo install cargo-fuzz`

### Targets
- `data_file`: exercises `.pack` parsing, checksum verification, entry lookup, and entry decoding.
- `index_file`: exercises `.idx` parsing, checksum verification, lookups, prefix lookups, and iteration.
- `multi_index_file`: exercises multi-pack-index parsing, checksum verification, lookups, prefix lookups, and iteration.

### Running a specific target
```bash
cargo +nightly fuzz run data_file -- -max_total_time=60
cargo +nightly fuzz run index_file -- -max_total_time=60
cargo +nightly fuzz run multi_index_file -- -max_total_time=60
```

### Running all targets
```bash
for target in data_file index_file multi_index_file; do
    cargo +nightly fuzz run "$target" -- -max_total_time=60
done
```

## Artifact Reproducers

The `artifacts/<target>` directories contain minimized reproducer inputs produced by libFuzzer or Google OSS-Fuzz.

The integration test module `fuzzed` reads every regular file in the populated artifact directories and runs it through the parser under test. This lets `cargo test -p gix-pack-tests fuzzed` reproduce known fuzz failures without starting a fuzzing session.

When OSS-Fuzz reports a new failure, place the minimized testcase in the matching `artifacts/<target>` directory, then run `cargo test -p gix-pack-tests fuzzed`. To confirm it against the original harness, run `cargo fuzz run <target> artifacts/<target>/<reproducer>` from this directory.
