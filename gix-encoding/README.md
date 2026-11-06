# gix-encoding

Strict, streaming character conversion for Git, implemented in safe Rust without
a runtime dependency on `iconv` or `encoding_rs`.

```rust
use std::io::{Read, Write};
use gix_encoding::{Encoding, read, write};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let encoding: Encoding = "UTF-16LE".parse()?;
assert_eq!(encoding.to_string(), "UTF-16LE");

let mut utf8 = String::new();
read::Decoder::new(&b"h\0i\0"[..], encoding).read_to_string(&mut utf8)?;

let mut encoder = write::Encoder::new(Vec::new(), encoding);
encoder.write_all(utf8.as_bytes())?;
let encoded = encoder.finish()?;
assert_eq!(encoded, b"h\0i\0");
# Ok(())
# }
```

`read::{Decoder, Encoder, Transcoder}` and `write::{Decoder, Encoder, Transcoder}`
wrap standard I/O. Decoders produce UTF-8; encoders consume UTF-8; transcoders
accept arbitrary source and destination encodings. Buffering is bounded (16 KiB
for readers, 8 KiB plus an incomplete character for writers). Errors and short
writes retain unwritten output. Writers require explicit `finish()` or
`try_finish()`; `flush()` does not terminate a stream, and dropping does no I/O.

`Converter` has no per-stream heap allocations. Compressed mapping tables are
allocated on first use and shared by subsequent conversions. Its `convert::Progress`
reports consumed input and written output, with `convert::Status` indicating
whether more input/output is needed or conversion has finished.
Keep unconsumed input when a character straddles chunks. A 32-byte
output buffer guarantees progress. `convert()` allocates a result, while
`convert_to()` replaces a caller-owned vector and reuses its capacity. These
convenience functions clear partial output on errors.

## Encoding detection

`detect(input: &[u8]) -> Detection` inspects a **complete** byte slice. It returns
an encoding with categorical `detect::Evidence`, not a numerical probability: byte
validity is exact, but inferring the original encoding generally is not. Even
ICU describes its [confidence scores as somewhat arbitrary](https://unicode-org.github.io/icu-docs/apidoc/released/icu4c/ucsdet_8h.html).

```rust
use gix_encoding::{Detection, Encoding, convert, detect};

# fn main() -> gix_error::TestResult {
let input = b"\xff\xfeh\0i\0";
match detect(input) {
    Detection::Detected { encoding, evidence, bom_len } => {
        // Skip the signature explicitly; raw conversion otherwise preserves it.
        let text = convert(&input[bom_len..], encoding, Encoding::Utf8)?;
        assert_eq!(text, b"hi");
        println!("{encoding}: {evidence:?}");
    }
    Detection::Ambiguous { candidates } => {
        // Use external metadata or ask the caller; these are not likelihood-ranked.
        println!("Compatible encodings: {candidates:?}");
    }
    Detection::Empty => {}
    Detection::InvalidBom { encoding, .. } => {
        println!("BOM indicates {encoding}, but the payload is malformed");
    }
    _ => {}
}
# Ok(())
# }
```

- **BOMs:** recognizes UTF-8/16/32 signatures, longest first, then validates the
  entire payload. A malformed payload is reported separately, without a legacy
  fallback. The returned UTF-16/32 encoding has explicit byte order. `bom_len`
  allows callers to omit the signature without changing raw conversion behavior.
  `FF FE 00 00` is interpreted as UTF-32LE, even though UTF-16LE followed by NUL
  is another possible reading.
- **BOM-less Unicode:** recognizes conservative UTF-16/32 byte patterns in
  ASCII-heavy text (at least four code units, at least half ASCII). Full strict
  validation and a single matching byte-order variant are required. Other
  BOM-less UTF-16/32 input can remain ambiguous.
- **ISO-2022-JP** (with `asian` enabled): recognizes supported non-ASCII
  designations before applying the seven-bit ASCII shortcut, and validates the
  complete stream.
- **ASCII and UTF-8:** returns `Ascii` or `Utf8` evidence, respectively. ASCII is
  compatible with many encodings; even non-ASCII UTF-8 can also be valid legacy
  text. These results express a preferred interpretation, not certainty.
- **Legacy ambiguity:** returns all strictly compatible source encodings in
  `Encoding::ALL` order, preserving distinctions such as Latin-1/Windows-1252
  and Shift-JIS/CP932. Native-endian and BOM aliases are omitted in favor of
  explicit byte order. There are no statistical language models or arbitrary
  Western-encoding fallbacks.
- **Empty and binary-looking input:** empty input provides no encoding evidence.
  Without a BOM, controls other than tab, LF, FF, and CR prevent a preferred text
  interpretation. This is not a binary-file classifier: even binary data can
  decode successfully, and some real text deliberately remains ambiguous.

Detection validates through the end, including incomplete trailing characters,
and uses bounded conversion scratch space rather than retaining decoded copies.
Ambiguous input requires a pass per enabled source encoding and may initialize
all of their lazy mapping tables. Apart from these shared tables, allocations
are limited to the candidate vector and codec error diagnostics. Prefer declared
metadata such as `working-tree-encoding` whenever available.

## Command-line examples

### Detection

The `detect` example reads at most 1 MiB from stdin by default and prints the
sample size and detection result to stdout. An optional `max-bytes` argument
sets the limit in bytes; zero produces `Empty` without reading stdin.

```sh
printf 'hello' | cargo run --quiet -p gix-encoding --example detect
printf 'hello' | cargo run --quiet -p gix-encoding --example detect -- 4096
```

It stops at EOF or the limit, without probing for another byte after reaching
the limit. A capped result is explicitly labeled as a sample: the input may
continue, and the cutoff may split a character or escape sequence. Detection
validates only the sampled bytes as a complete slice, so such a cutoff can
produce ambiguity or `InvalidBom`; it does not establish the encoding or
validity of the unread remainder. Ambiguous, empty, and malformed-BOM results
are printed normally. Argument and I/O errors go to stderr with a nonzero exit
status.

### Conversion

The `transcode` example streams stdin to stdout using two encoding labels.
It requires both `legacy` and `asian` so every supported encoding is available:

```sh
cargo build -p gix-encoding --example transcode --features legacy,asian
echo hi | target/debug/examples/transcode UTF-8 UTF-16LE
```

Or run it directly through Cargo:

```sh
echo hi | cargo run --quiet -p gix-encoding --example transcode --features legacy,asian -- UTF-8 UTF-16LE
```

Invalid arguments, conversion errors, and I/O failures are reported on stderr
with a nonzero exit status. As conversion is streaming, failures can leave
partial output on stdout.

## Encoding features and table storage

The default repertoire is UTF-8/16/32 (including Git's BOM variants), ASCII,
ISO-8859-1, Windows-1252, and ISO-8859-15. Other encoding families are opt-in:

| Feature | Additional encodings |
| --- | --- |
| `western` (default) | Windows-1252 and ISO-8859-15 |
| `legacy` (includes `western`) | Remaining ISO-8859 and Windows code pages, Thai, KOI8, Macintosh, IBM, and CP1125 |
| `asian` | Japanese (Shift-JIS, CP932, EUC-JP, ISO-2022-JP), Korean (EUC-KR, CP949), and Chinese (GB2312, GBK, GB18030, Big5, Big5-HKSCS) |

With `default-features = false`, only Unicode, ASCII, and ISO-8859-1 remain.
Features are additive; disabled variants are absent from `Encoding`, their
labels are rejected, and detection considers only enabled encodings. Enable
both `legacy` and `asian` for the complete repertoire, or select groups as needed:

```toml
[dependencies]
gix-encoding = { version = "0.1", features = ["asian"] }
```

For a release build of the transcoder:

```sh
cargo build -p gix-encoding --release --example transcode --features legacy,asian
```

`build.rs` compresses the checked-in mapping data with `gix-zlib`; only tables
for enabled encodings are embedded in the binary. No external compression tool
or system encoding library is needed. Each table is decompressed by `gix-zlib`
on first access, initialized safely across threads, and kept in memory for the
rest of the process. Unicode conversions do not initialize legacy tables.
Per-stream buffering remains bounded, but first use adds the uncompressed size
of each needed table to the process-wide cache. This trades a one-time allocation
and decompression cost for smaller binaries without changing the mappings.

## Encodings and Git compatibility

`Encoding::ALL` enumerates the supported encodings. `FromStr`, `for_label()`,
`Display`, and `name()` use canonical names and recognized case-insensitive
Git/iconv aliases. ASCII, ISO-8859-1 and Windows-1252 are distinct, as are Shift-JIS
and CP932, and EUC-KR and CP949. Web-only aliases and lossy `//TRANSLIT` or
`//IGNORE` suffixes are not accepted.

Strict conversion does not imply reversibility: native mappings can be
many-to-one, and most legacy iconv encoders discard Unicode language tags
(U+E0000 through U+E007F). Git's round-trip checks remain necessary when byte
identity matters.

With `legacy` and `asian` enabled, the repertoire includes UTF-8/16/32, Git's explicit UTF-16 BOM spellings,
ISO-8859 variants, Windows code pages, KOI8, Macintosh and IBM code pages,
Shift-JIS/CP932, EUC-JP, ISO-2022-JP, EUC-KR/CP949, GB2312, GBK, GB18030,
Big5 and Big5-HKSCS. This is a defined portable repertoire, not every encoding
ever provided by a platform's `iconv`. Mappings are generated from GNU libc
2.43, the library used by the Linux Git baseline. Other system iconv versions
can differ; see [table provenance and regeneration](etc/README.md).
In particular, GNU libiconv's [UTF-16](https://github.com/arthenica/libiconv/blob/master/lib/utf16.h)
and [UTF-32](https://github.com/arthenica/libiconv/blob/master/lib/utf32.h) default
to big-endian and recognize interior BOMs, whereas GNU libc uses native byte
order and recognizes only an initial BOM. `gix-encoding` consistently follows
the GNU libc rules on every platform; explicit byte-order variants avoid this
default-order difference.

Malformed input and unrepresentable characters are errors, with source byte
offsets available through `convert::ConversionError` and reasons through
`convert::ErrorKind`. There is no replacement-character or HTML-entity fallback.

UTF-8 retains a leading BOM as a character. Unsuffixed UTF-16/32 detect input
byte order from a BOM and otherwise use native byte order, as GNU iconv does;
they write native byte order with a BOM when converting nonempty content.
Explicit LE/BE variants treat BOMs as data and do not insert them. Git's
UTF-16LE-BOM reads either byte order and always prepends a little-endian BOM on
output. UTF-16BE-BOM is output-only, matching Git's special handling. Worktree
BOM requirements, empty-file shortcuts, round-trip selection, and UTF-8
attribute bypass are implemented by `gix-filter`, separately from raw codecs.

Git's `reencode_string_iconv()` does not flush terminal conversion state. We
likewise omit the optional ISO-2022-JP reset escape at EOF. We deliberately
preserve pending characters in Windows-1255/1258 and Big5-HKSCS instead of
reproducing Git's loss of trailing composing characters. These cases are checked
against a finalized iconv conversion as well as across all streaming boundaries.

The existing `gix-filter` pipeline continues buffering when its CRLF auto
detection, round-trip checks, or filter ordering require whole-file decisions.
The new adapters can be used directly when those decisions are already known.

## Validation and performance

On GNU/Linux, integration tests use isolated Git fixtures to compare actual
checkout/check-in bytes and failures. They also compare every single byte, all byte
pairs for multibyte/stateful codecs, representative Unicode scalars, and
ISO-2022-JP transitions directly against iconv. Run the exhaustive scalar sweep
when changing codecs or regenerating mappings:

```sh
cargo test -p gix-encoding --all-features --release -- --include-ignored
```

The streaming tests cover every split of representative input with tiny output
buffers, pending character completion, malformed input, interrupted I/O,
retryable sink errors, and bounded read-ahead.

The [benchmark](benches/README.md) compares both complete buffers and reusable
streaming converters with in-process libc iconv, verifies equal bytes before
timing, and offers a strict per-case performance check:

```sh
cargo bench -p gix-encoding --all-features --bench convert -- --check
cargo bench -p gix-encoding --all-features --bench convert -- --stream --check
```

Results characterize the measured compiler, CPU, libc and inputs; they are not
a universal speed guarantee for every input and platform.
