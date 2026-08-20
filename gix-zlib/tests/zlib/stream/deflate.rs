use std::{
    io,
    io::{Read, Write},
};

use bstr::ByteSlice;

use gix_zlib::stream::deflate::{self, Compress, FlushCompress};
use gix_zlib::{Compression, Decompress};

type TestResult = gix_error::TestResult;

pub(crate) fn compressed(data: &[u8]) -> Vec<u8> {
    let mut writer = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    writer.write_all(data).expect("in-memory writes never fail");
    writer.flush().expect("in-memory flushes never fail");
    writer.into_inner()
}

/// Provide streaming decompression using the `std::io::Read` trait.
/// If `std::io::BufReader` is used, an allocation for the input buffer will be performed.
struct InflateReader<R> {
    inner: R,
    decompressor: Decompress,
}

impl<R> InflateReader<R>
where
    R: io::BufRead,
{
    pub fn from_read(read: R) -> InflateReader<R> {
        InflateReader {
            decompressor: Decompress::new(),
            inner: read,
        }
    }
}

impl<R> io::Read for InflateReader<R>
where
    R: io::BufRead,
{
    fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
        gix_zlib::stream::inflate::read(&mut self.inner, &mut self.decompressor, into)
    }
}

#[test]
fn small_file_decompress() -> TestResult {
    fn fixture_path(path: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../gix-odb/tests/fixtures")
            .join(path)
    }
    let r = InflateReader::from_read(io::BufReader::new(std::fs::File::open(fixture_path(
        "objects/37/d4e6c5c48ba0d245164c4e10d5f41140cab980",
    ))?));
    #[expect(clippy::unbuffered_bytes)]
    let mut bytes = r.bytes();
    let content = bytes.by_ref().take(16).collect::<Result<Vec<_>, _>>()?;
    assert_eq!(content.as_slice().as_bstr(), b"blob 9\0hi there\n".as_bstr());
    assert!(bytes.next().is_none());
    Ok(())
}

#[test]
fn all_at_once() -> TestResult {
    let mut w = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    assert_eq!(w.write(b"hello")?, 5);
    w.flush()?;

    let out = w.into_inner();
    assert!(out.len() == 12 || out.len() == 13);

    assert_deflate_buffer(out, b"hello")?;
    Ok(())
}

fn assert_deflate_buffer(out: Vec<u8>, expected: &[u8]) -> io::Result<()> {
    let mut actual = Vec::new();
    InflateReader::from_read(out.as_slice()).read_to_end(&mut actual)?;
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn higher_levels_compress_better() -> TestResult {
    let data: Vec<u8> = (0..128 * 1024).map(|i| (i % 100) as u8).collect();
    let mut sizes = Vec::new();
    for level in [Compression::NONE, Compression::BEST_SPEED, Compression::DEFAULT] {
        let mut writer = deflate::Write::new(Vec::new(), level);
        writer.write_all(&data)?;
        writer.flush()?;
        let compressed = writer.into_inner();
        assert_deflate_buffer(compressed.clone(), &data)?;
        sizes.push(compressed.len());
    }
    assert!(
        sizes[0] > sizes[1] && sizes[1] > sizes[2],
        "each level compresses better than the one before it: {sizes:?}"
    );
    Ok(())
}

#[test]
fn big_file_small_writes() -> TestResult {
    let mut w = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    let bytes = include_bytes!(
        "../../../../gix-odb/tests/fixtures/objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
    );
    for chunk in bytes.chunks(2) {
        assert_eq!(w.write(chunk)?, chunk.len());
    }
    w.flush()?;

    assert_deflate_buffer(w.into_inner(), bytes)?;
    Ok(())
}

#[test]
fn big_file_a_few_big_writes() -> TestResult {
    let mut w = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    let bytes = include_bytes!(
        "../../../../gix-odb/tests/fixtures/objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
    );
    for chunk in bytes.chunks(4096 * 9) {
        assert_eq!(w.write(chunk)?, chunk.len());
    }
    w.flush()?;

    assert_deflate_buffer(w.into_inner(), bytes)?;
    Ok(())
}

#[test]
fn compressor_lifecycle_counters_and_flush_modes() -> TestResult {
    for flush in [
        FlushCompress::None,
        FlushCompress::Partial,
        FlushCompress::Sync,
        FlushCompress::Full,
    ] {
        let mut compressor = Compress::new(Compression::BEST_SPEED);
        assert_eq!(compressor.total_in(), 0);
        assert_eq!(compressor.total_out(), 0);

        let input = b"compress through each flush mode";
        let mut output = [0; 256];
        let first_status = compressor.compress(input, &mut output, flush)?;
        assert!(matches!(
            first_status,
            gix_zlib::Status::Ok | gix_zlib::Status::BufError
        ));
        assert_eq!(compressor.total_in(), input.len() as u64);
        let first_written = compressor.total_out() as usize;

        assert_eq!(
            compressor.compress(&[], &mut output[first_written..], FlushCompress::Finish)?,
            gix_zlib::Status::StreamEnd
        );
        let total_written = compressor.total_out() as usize;
        assert_deflate_buffer(output[..total_written].to_vec(), input)?;

        compressor.reset();
        assert_eq!(compressor.total_in(), 0);
        assert_eq!(compressor.total_out(), 0);
    }
    Ok(())
}

#[test]
fn writer_clone_and_reset() -> TestResult {
    let original = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    let mut cloned = original.clone();
    cloned.write_all(b"clone owns a fresh compressor")?;
    cloned.flush()?;
    assert_deflate_buffer(cloned.into_inner(), b"clone owns a fresh compressor")?;
    assert!(original.into_inner().is_empty());

    let mut writer = deflate::Write::new(Vec::new(), Compression::BEST_SPEED);
    writer.write_all(b"first")?;
    writer.flush()?;
    writer.reset();
    writer.write_all(b"second")?;
    writer.flush()?;
    let streams = writer.into_inner();

    let mut first = gix_zlib::Inflate::default();
    let mut output = [0; 16];
    let (_, consumed, written) = first.once(&streams, &mut output)?;
    assert_eq!(
        &output[..written],
        b"first",
        "the first member should decode independently"
    );

    first.reset();
    let (_, second_consumed, written) = first.once(&streams[consumed..], &mut output)?;
    assert_eq!(
        &output[..written],
        b"second",
        "resetting the inflater should allow decoding the second member"
    );
    assert_eq!(
        consumed + second_consumed,
        streams.len(),
        "decoding both members should consume the complete concatenated stream"
    );
    Ok(())
}
