use std::io::{self, Read, Write};

use gix_encoding::{Converter, Encoding, convert, convert::Status, read, write};
use gix_error::TestResult;

fn cases() -> Vec<(Encoding, &'static str)> {
    vec![
        (Encoding::Utf8, "aé日本😀"),
        (Encoding::Utf16, "aé日本😀"),
        (Encoding::Utf16Le, "aé日本😀"),
        (Encoding::Utf16Be, "aé日本😀"),
        (Encoding::Utf16LeBom, "aé日本😀"),
        (Encoding::Utf32, "aé日本😀"),
        (Encoding::Utf32Le, "aé日本😀"),
        (Encoding::Utf32Be, "aé日本😀"),
        (Encoding::Ascii, "a\0z"),
        (Encoding::Iso8859_1, "aé£"),
        #[cfg(feature = "western")]
        (Encoding::Windows1252, "“été” €"),
        #[cfg(feature = "western")]
        (Encoding::Iso8859_15, "aé€"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1255, "אב"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1258, "ấ"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1255, "א"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1255, "אב"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1258, "A"),
        #[cfg(feature = "legacy")]
        (Encoding::Windows1258, "ắ"),
        #[cfg(feature = "asian")]
        (Encoding::ShiftJis, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::Cp932, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::EucJp, "日本語"),
        #[cfg(feature = "asian")]
        (Encoding::Iso2022Jp, "日本a語"),
        #[cfg(feature = "asian")]
        (Encoding::Gb2312, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Gbk, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Gb18030, "中文😀"),
        #[cfg(feature = "asian")]
        (Encoding::Big5, "中文"),
        #[cfg(feature = "asian")]
        (Encoding::Big5Hkscs, "Ê̄中Ê"),
        #[cfg(feature = "asian")]
        (Encoding::Big5Hkscs, "Ê"),
        #[cfg(feature = "asian")]
        (Encoding::EucKr, "한국어"),
        #[cfg(feature = "asian")]
        (Encoding::Cp949, "한국어"),
    ]
}

fn split_convert(
    input: &[u8],
    from: Encoding,
    to: Encoding,
    split: usize,
    capacity: usize,
) -> gix_error::Result<Vec<u8>> {
    let mut converter = Converter::new(from, to);
    let mut result = Vec::new();
    let mut retained = Vec::new();
    for (chunk, last) in [(&input[..split], false), (&input[split..], true)] {
        retained.extend_from_slice(chunk);
        let mut output = vec![0; capacity];
        loop {
            let progress = converter.convert(&retained, &mut output, last)?;
            assert!(
                progress.read <= retained.len(),
                "the converter never reads beyond its input"
            );
            assert!(
                progress.written <= output.len(),
                "the converter never writes beyond its output"
            );
            result.extend_from_slice(&output[..progress.written]);
            retained.drain(..progress.read);
            match progress.status {
                Status::InputEmpty => {
                    assert!(!last, "final input must finish or request output space");
                    break;
                }
                Status::Finished => {
                    assert!(last && retained.is_empty(), "finishing consumes every source byte");
                    break;
                }
                Status::OutputFull if progress.read == 0 && progress.written == 0 => {
                    assert!(output.len() < 32, "32 output bytes guarantee progress");
                    output.resize(32, 0);
                }
                Status::OutputFull => {}
            }
        }
    }
    Ok(result)
}

#[test]
fn every_input_split_with_tiny_output_buffers() -> TestResult {
    for (encoding, text) in cases() {
        let encoded = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        for (input, from, to, expected) in [
            (text.as_bytes(), Encoding::Utf8, encoding, encoded.as_slice()),
            (encoded.as_slice(), encoding, Encoding::Utf8, text.as_bytes()),
        ] {
            for split in 0..=input.len() {
                for capacity in 0..=8 {
                    assert_eq!(
                        split_convert(input, from, to, split, capacity)?,
                        expected,
                        "{from} -> {to}, split {split}, output capacity {capacity}"
                    );
                }
            }
        }
    }
    Ok(())
}

struct TinyReader<R>(R);

impl<R: Read> Read for TinyReader<R> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        let len = output.len().min(1);
        self.0.read(&mut output[..len])
    }
}

fn read_one_byte_at_a_time(mut reader: impl Read) -> io::Result<Vec<u8>> {
    let mut result = Vec::new();
    let mut byte = [0];
    while reader.read(&mut byte)? != 0 {
        result.push(byte[0]);
    }
    assert_eq!(reader.read(&mut byte)?, 0, "EOF is stable after conversion finishes");
    Ok(result)
}

#[test]
fn reader_and_writer_adapters_preserve_split_characters() -> TestResult {
    for (encoding, text) in cases() {
        let encoded = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        assert_eq!(
            read_one_byte_at_a_time(read::Encoder::new(TinyReader(text.as_bytes()), encoding))?,
            encoded,
            "reader encoding {encoding} retains incomplete UTF-8 input"
        );
        assert_eq!(
            read_one_byte_at_a_time(read::Decoder::new(TinyReader(encoded.as_slice()), encoding))?,
            text.as_bytes(),
            "reader decoding {encoding} retains incomplete encoded input"
        );
        let mut encoder = write::Encoder::new(Vec::new(), encoding);
        for byte in text.as_bytes() {
            encoder.write_all(&[*byte])?;
            encoder.flush()?;
        }
        assert_eq!(
            encoder.finish()?,
            encoded,
            "flush does not finish an incomplete {encoding} stream"
        );
        let mut decoder = write::Decoder::new(Vec::new(), encoding);
        for byte in &encoded {
            decoder.write_all(&[*byte])?;
            decoder.flush()?;
        }
        assert_eq!(
            decoder.finish()?,
            text.as_bytes(),
            "writer decoding {encoding} retains incomplete input"
        );
    }
    Ok(())
}

#[test]
fn transcoders_compose_two_non_utf8_encodings() -> TestResult {
    for (encoding, text) in [
        (Encoding::Iso8859_1, "Été à Noël"),
        (Encoding::Utf32Le, "Aé日本😀"),
        #[cfg(feature = "asian")]
        (Encoding::EucJp, "日本語"),
    ] {
        let source = convert(text.as_bytes(), Encoding::Utf8, encoding)?;
        let expected = convert(text.as_bytes(), Encoding::Utf8, Encoding::Utf16Be)?;
        let mut reader = read::Transcoder::new(TinyReader(source.as_slice()), encoding, Encoding::Utf16Be);
        assert_eq!(reader.read(&mut [])?, 0, "empty output never consumes input");
        assert_eq!(reader.get_ref().0, source, "an empty read leaves the source untouched");
        assert_eq!(
            read_one_byte_at_a_time(reader)?,
            expected,
            "reader transcodes directly from {encoding} to UTF-16BE"
        );
        let mut writer = write::Transcoder::new(Vec::new(), encoding, Encoding::Utf16Be);
        writer.write_all(&source)?;
        assert_eq!(
            writer.finish()?,
            expected,
            "writer transcodes directly from {encoding} to UTF-16BE"
        );
    }
    Ok(())
}

#[test]
fn empty_bom_only_and_terminal_state_are_consistent() -> TestResult {
    for encoding in [
        Encoding::Utf8,
        Encoding::Utf16,
        Encoding::Utf32,
        Encoding::Utf16LeBom,
        Encoding::Utf16BeBom,
    ] {
        let expected = convert(b"", Encoding::Utf8, encoding)?;
        assert_eq!(
            read_one_byte_at_a_time(read::Encoder::new(&b""[..], encoding))?,
            expected,
            "empty reader encoding {encoding} follows the whole-buffer API"
        );
        assert_eq!(
            write::Encoder::new(Vec::new(), encoding).finish()?,
            expected,
            "empty writer encoding {encoding} follows the whole-buffer API"
        );
    }
    for input in [b"\xff\xfe".as_slice(), b"\xfe\xff".as_slice()] {
        assert!(
            convert(input, Encoding::Utf16, Encoding::Utf32)?.is_empty(),
            "a consumed source BOM alone does not create an output BOM"
        );
    }
    let mut converter = Converter::new(Encoding::Utf8, Encoding::Utf16Le);
    let mut output = [0; 32];
    assert_eq!(
        converter.convert(b"x", &mut output, true)?.status,
        Status::Finished,
        "final input finishes conversion"
    );
    assert_eq!(
        converter.convert(b"", &mut output, true)?.status,
        Status::Finished,
        "empty final calls are idempotent"
    );
    assert!(
        converter.convert(b"x", &mut output, true).is_err(),
        "finished streams reject more input"
    );
    converter.reset();
    assert_eq!(
        converter.convert(b"y", &mut output, true)?.written,
        2,
        "reset starts an independent stream"
    );
    Ok(())
}

#[test]
fn malformed_and_unrepresentable_input_fail_without_replacement() -> TestResult {
    for (input, from, to) in [
        (b"\xf0\x9f".as_slice(), Encoding::Utf8, Encoding::Utf16Le),
        (b"\xff".as_slice(), Encoding::Utf8, Encoding::Utf16Le),
        (b"\x00\xd8".as_slice(), Encoding::Utf16Le, Encoding::Utf8),
        #[cfg(feature = "asian")]
        (b"\x81".as_slice(), Encoding::ShiftJis, Encoding::Utf8),
        ("😀".as_bytes(), Encoding::Utf8, Encoding::Ascii),
    ] {
        let mut reader = read::Transcoder::new(TinyReader(input), from, to);
        let error = reader
            .read_to_end(&mut Vec::new())
            .expect_err("malformed or unrepresentable input is rejected");
        assert_eq!(
            error.kind(),
            io::ErrorKind::InvalidData,
            "read errors retain the encoding failure category"
        );
        let mut writer = write::Transcoder::new(Vec::new(), from, to);
        let result = writer.write_all(input).and_then(|()| writer.try_finish());
        assert_eq!(
            result.expect_err("writer rejects invalid conversion").kind(),
            io::ErrorKind::InvalidData,
            "write errors retain the encoding failure category"
        );
    }
    let mut output = vec![1, 2, 3];
    assert!(
        gix_encoding::convert_to(b"ok\xff", Encoding::Utf8, Encoding::Utf16Le, &mut output).is_err(),
        "conversion reports invalid UTF-8"
    );
    assert!(
        output.is_empty(),
        "whole-buffer conversion does not leave partial results after failure"
    );
    Ok(())
}

#[derive(Default)]
struct RecoverableSink {
    bytes: Vec<u8>,
    fail_next_write: Option<io::ErrorKind>,
    fail_next_flush: bool,
    zero_next_write: bool,
    block_after: Option<usize>,
}

impl Write for RecoverableSink {
    fn write(&mut self, input: &[u8]) -> io::Result<usize> {
        if let Some(kind) = self.fail_next_write.take() {
            return Err(kind.into());
        }
        if std::mem::take(&mut self.zero_next_write) {
            return Ok(0);
        }
        if self.block_after.is_some_and(|limit| self.bytes.len() >= limit) {
            self.block_after = None;
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let len = input.len().min(1);
        self.bytes.extend_from_slice(&input[..len]);
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        if std::mem::take(&mut self.fail_next_flush) {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        Ok(())
    }
}

#[test]
fn retryable_sink_failures_preserve_all_accepted_output() -> TestResult {
    let text = "日本😀";
    let expected = convert(text.as_bytes(), Encoding::Utf8, Encoding::Utf16Le)?;
    let mut writer = write::Encoder::new(RecoverableSink::default(), Encoding::Utf16Le);
    writer.write_all(text.as_bytes())?;
    writer.get_mut().fail_next_write = Some(io::ErrorKind::Interrupted);
    writer.get_mut().zero_next_write = true;
    assert_eq!(
        writer.try_finish().expect_err("zero-byte writes are errors").kind(),
        io::ErrorKind::WriteZero,
        "WriteZero retains pending output for retry"
    );
    writer.get_mut().block_after = Some(3);
    assert_eq!(
        writer.try_finish().expect_err("sink temporarily blocks").kind(),
        io::ErrorKind::WouldBlock,
        "sink errors are preserved"
    );
    assert_eq!(
        writer.get_ref().bytes.len(),
        3,
        "a sink failure follows successful short writes"
    );
    writer.get_mut().fail_next_write = Some(io::ErrorKind::Interrupted);
    writer.get_mut().fail_next_flush = true;
    assert_eq!(
        writer.try_finish().expect_err("sink flush temporarily blocks").kind(),
        io::ErrorKind::WouldBlock,
        "finish is retryable after conversion completed"
    );
    writer.try_finish()?;
    assert_eq!(
        writer.get_ref().bytes,
        expected,
        "retrying never duplicates or loses accepted output"
    );
    assert_eq!(
        writer.write(b"x").expect_err("finished writers reject input").kind(),
        io::ErrorKind::BrokenPipe,
        "writes after finish are explicit errors"
    );
    assert_eq!(writer.finish()?.bytes, expected, "finishing twice is harmless");
    Ok(())
}

struct GeneratedReader {
    remaining: usize,
    consumed: usize,
    interrupt: bool,
    would_block: bool,
}

impl Read for GeneratedReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if std::mem::take(&mut self.interrupt) {
            return Err(io::ErrorKind::Interrupted.into());
        }
        if std::mem::take(&mut self.would_block) {
            return Err(io::ErrorKind::WouldBlock.into());
        }
        let len = output.len().min(self.remaining);
        output[..len].fill(b'a');
        self.remaining -= len;
        self.consumed += len;
        Ok(len)
    }
}

#[test]
fn reader_is_lazy_bounded_and_recovers_from_source_errors() -> TestResult {
    let source = GeneratedReader {
        remaining: 10_000_000,
        consumed: 0,
        interrupt: true,
        would_block: true,
    };
    let mut reader = read::Encoder::new(source, Encoding::Utf16Le);
    assert_eq!(reader.get_ref().consumed, 0, "construction does not read the source");
    assert_eq!(
        reader.read(&mut []).expect("empty reads do no I/O"),
        0,
        "empty reads leave pending source failures untouched"
    );
    assert_eq!(
        reader.read(&mut [0]).expect_err("the source temporarily blocks").kind(),
        io::ErrorKind::WouldBlock,
        "Interrupted is retried and WouldBlock reaches the caller"
    );
    let mut byte = [0];
    assert_eq!(reader.read(&mut byte)?, 1, "conversion can resume after a source error");
    assert_eq!(byte[0], b'a', "the first output byte is preserved");
    assert!(
        reader.get_ref().consumed <= 8192,
        "reading one byte cannot buffer the whole source"
    );
    let consumed = reader.get_ref().consumed;
    reader.get_mut().interrupt = true;
    let source = reader.into_inner();
    assert_eq!(source.consumed, consumed, "extracting the reader does not perform I/O");
    Ok(())
}

#[test]
fn writer_applies_backpressure_to_large_inputs() -> TestResult {
    let input = vec![b'a'; 128 * 1024];
    let mut writer = write::Encoder::new(Vec::new(), Encoding::Utf32Be);
    let accepted = writer.write(&input)?;
    assert!(
        accepted > 0 && accepted <= 8192,
        "a write accepts only bounded buffered work"
    );
    writer.write_all(&input[accepted..])?;
    let actual = writer.finish()?;
    assert_eq!(
        actual,
        convert(&input, Encoding::Utf8, Encoding::Utf32Be)?,
        "large writes preserve every byte while applying backpressure"
    );
    Ok(())
}
