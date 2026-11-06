# Comparing conversion with Git's iconv

Run the complete matrix (both `legacy` and `asian` features are required) on an
otherwise idle GNU/Linux machine:

```sh
cargo bench -p gix-encoding --bench convert --all-features -- --check
cargo bench -p gix-encoding --bench convert --all-features -- --stream --check
```

The executable prints CSV with median nanoseconds per conversion and the ratio
`gix / iconv`. Ratios at or below `1.0` mean that `gix-encoding` was no slower for
that input. `--check` exits unsuccessfully if **any** measured case is slower;
there is no aggregate score or hidden tolerance. Omit it to collect a report
while optimizing. `--quick` reduces sample duration and count for development.
`GIX_ENCODING_BENCH_FILTER=UTF-16LE` selects cases by name for focused work; a
filtered run is not evidence for the entire matrix.

Each timed operation constructs a converter, converts the complete input, and
returns an owned output buffer. The iconv reference calls libc directly in the
same process, so no process startup or filesystem I/O contaminates its timing.
Its result buffer follows Git's allocation policy: initially one byte per input
byte plus a terminator, growing to bytes written plus twice the unconsumed input
plus 32 bytes after `E2BIG`. Like Git's `malloc`, the reference does not initialize
spare capacity before iconv writes it. Both implementations allocate their own
result, and their bytes must match before measurements begin. Encoding-name `CString` construction is
outside the measurements, as Git also starts with NUL-terminated labels.
Correctness checks initialize compressed tables before timing, so results exclude
the one-time decompression cost.

The `--stream` mode measures reusable converters and reusable iconv descriptors
with 8 KiB input and output chunks. Both retain split source characters between
chunks and reset their state for each iteration. Output buffers and descriptors
are allocated before timing; each emitted output slice is consumed by
`black_box`. A separate untimed pass checks the complete streamed output byte
for byte. These measurements expose steady-state throughput without allocation
or descriptor-opening costs. Both modes must pass to establish non-regression
for both APIs.

Each mode's matrix covers both directions at roughly 1 KiB and 1 MiB. ASCII and mixed
Unicode exercise UTF-16 and Latin-1 separately; other cases cover UTF-32,
Windows-1252, Japanese stateful and multibyte encodings, Chinese encodings, and
Korean. Timed batches are calibrated to at least 100 ms, run nine times in
alternating order, and summarized by their median. The benchmark does not flush
iconv's terminal shift state, matching Git's `reencode_string_iconv()`.

A measured pass establishes a performance comparison for this machine, libc,
compiler, and corpus. It cannot establish that every possible input on every
platform is faster. Record the exact toolchain, CPU, libc, and command alongside
published numbers, and rerun the complete matrix after converter changes.
