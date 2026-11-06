# Mapping data

The files in `../src/tables` contain numeric observations of GNU libc `iconv`.
They were generated with **glibc 2.43**, Ubuntu package `2.43-2ubuntu2.4`, on
AArch64 Linux. The format is explicitly little endian and is independent of
the generator's machine byte order. No `iconv` implementation source or system
character-map files are copied into the crate. Runtime conversion is pure Rust
and does not load a system encoding library.

The checked-in `.bin` files remain uncompressed for reproducible regeneration.
At build time, `build.rs` compresses them with `gix-zlib` into Cargo's `OUT_DIR`.
Each generated file contains its uncompressed length as a little-endian `u32`,
followed by one complete zlib stream. Only feature-enabled tables are embedded;
the runtime decompresses each lazily and shares its cached bytes across threads.
Tests check every compressed table against its raw source byte-for-byte.

Run the following from the repository root on a system providing GNU `iconv`
and a C99 compiler:

```bash
bash gix-encoding/etc/generate.sh
cargo test -p gix-encoding --all-features --test encoding -- legacy::
cargo test -p gix-encoding --all-features --release --test encoding -- legacy:: --include-ignored
```

The generator queries every possible one- and two-byte sequence and all EUC-JP
three-byte prefixes. It queries every Unicode scalar independently for the
reverse maps; it does **not** invert the decoder map, since `iconv` makes
asymmetric choices for duplicate mappings. GB 18030's four-byte space is
exhaustively enumerated and coalesced into contiguous ranges. Repeating
generation with the same `iconv` data produces byte-identical tables. Changes
in a newer implementation can be inspected by regenerating and running the
independent differential tests before updating the checked-in data.

The regular `.bin` files start with four `u32` counts: decoding trie nodes,
BMP encoding pages, supplementary encoding entries, and two-scalar decoding
entries. They contain, in order:

- Decoding nodes with 256 `u32` slots each. A slot holds a Unicode scalar,
  `0xffffffff` for invalid input, bit 31 plus a child-node index, or bit 30 plus
  a two-scalar entry index. Multibyte encodings reserve 256 second-byte nodes,
  so the node for a lead byte has index `lead + 1`, avoiding a dependent lookup
  in conversion loops. Further EUC-JP nodes follow those dense pages.
- 256 `u16` indices selecting a BMP encoding page, with `0xffff` for absent pages.
  Multibyte encodings have all 256 pages in order, allowing direct indexing.
- Encoding pages with 256 `u32` slots each, holding the destination bytes packed
  most significant byte first. Zero represents NUL; `0xffffffff` is unmappable;
  `0xfffffffe` represents `iconv`'s zero-byte output for Unicode tag characters.
- Sorted `(Unicode scalar, packed bytes)` supplementary entries, both `u32`.
- Two-scalar decoding entries, each containing two `u32` Unicode scalars.

The GB 18030 range files contain `(start, length, mapped_start)` `u32` triples,
sorted by input pointer or Unicode scalar respectively.

Windows-1255 and Windows-1258 decoding has finite state because `iconv` combines
adjacent Hebrew and Vietnamese characters. Their `*-combining.bin` files have
a `u32` state count, followed by each state's buffered scalar and 256 transitions.
A transition contains two output scalars and the next state, all `u32`.
`0xffffffff` indicates an invalid transition and `0xfffffffe` indicates no
output scalar. The generator discovers states by their pending scalar, keeps
a minimal representative input, and queries all 256 transitions from each
state. There are 34 and 158 states respectively.

ISO-2022-JP uses the same JIS X 0208 mappings as EUC-JP, plus its ASCII and
JIS Roman shift states. The differential test checks all Unicode scalars,
all escape sequences, and all JIS pairs against `iconv` independently.

Git's `reencode_string_iconv()` does not call `iconv`'s final flush operation.
The Rust converter likewise leaves ISO-2022-JP in its current shift state at
EOF. It does preserve a pending Windows-1255/1258 or Big5-HKSCS character at
EOF, where Git's omission can lose the final character. The tests compare
these encodings against `iconv` with its final flush, and test the preservation
explicitly.
