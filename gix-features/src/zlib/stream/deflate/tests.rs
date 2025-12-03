mod deflate_stream {
    use std::{
        io,
        io::{Read, Write},
    };

    use bstr::ByteSlice;

    use crate::zlib::stream::deflate;
    use crate::zlib::Decompress;

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
                decompressor: Decompress::new(true, 15),
                inner: read,
            }
        }
    }

    impl<R> io::Read for InflateReader<R>
    where
        R: io::BufRead,
    {
        fn read(&mut self, into: &mut [u8]) -> io::Result<usize> {
            crate::zlib::stream::inflate::read(&mut self.inner, &mut self.decompressor, into)
        }
    }

    #[test]
    fn small_file_decompress() -> Result<(), Box<dyn std::error::Error>> {
        fn fixture_path(path: &str) -> std::path::PathBuf {
            std::path::PathBuf::from("tests/fixtures").join(path)
        }
        let r = InflateReader::from_read(io::BufReader::new(std::fs::File::open(fixture_path(
            "objects/37/d4e6c5c48ba0d245164c4e10d5f41140cab980",
        ))?));
        #[allow(clippy::unbuffered_bytes)]
        let mut bytes = r.bytes();
        let content = bytes.by_ref().take(16).collect::<Result<Vec<_>, _>>()?;
        assert_eq!(content.as_slice().as_bstr(), b"blob 9\0hi there\n".as_bstr());
        assert!(bytes.next().is_none());
        Ok(())
    }

    #[test]
    fn all_at_once() -> Result<(), Box<dyn std::error::Error>> {
        let mut w = deflate::Write::new(Vec::new());
        assert_eq!(w.write(b"hello")?, 5);
        w.flush()?;

        let out = w.inner;
        assert!(out.len() == 12 || out.len() == 13);

        assert_deflate_buffer(out, b"hello")
    }

    fn assert_deflate_buffer(out: Vec<u8>, expected: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let mut actual = Vec::new();
        InflateReader::from_read(out.as_slice()).read_to_end(&mut actual)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn big_file_small_writes() -> Result<(), Box<dyn std::error::Error>> {
        let mut w = deflate::Write::new(Vec::new());
        let bytes = include_bytes!(
            "../../../../../gix-odb/tests/fixtures/objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
        );
        for chunk in bytes.chunks(2) {
            assert_eq!(w.write(chunk)?, chunk.len());
        }
        w.flush()?;

        assert_deflate_buffer(w.inner, bytes)
    }

    #[test]
    fn big_file_a_few_big_writes() -> Result<(), Box<dyn std::error::Error>> {
        let mut w = deflate::Write::new(Vec::new());
        let bytes = include_bytes!(
            "../../../../../gix-odb/tests/fixtures/objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack"
        );
        for chunk in bytes.chunks(4096 * 9) {
            assert_eq!(w.write(chunk)?, chunk.len());
        }
        w.flush()?;

        assert_deflate_buffer(w.inner, bytes)
    }
}
