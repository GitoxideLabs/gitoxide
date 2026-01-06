use gix_odb::pack;

use crate::fixture_path;

fn pack_at(at: &str) -> pack::data::File {
    pack::data::File::at(fixture_path(at).as_path(), gix_hash::Kind::Sha1).expect("valid pack file")
}

mod method {
    use std::sync::atomic::AtomicBool;

    use gix_features::progress;

    use crate::{
        hex_to_id,
        pack::{data::file::pack_at, SMALL_PACK},
    };

    #[test]
    fn checksum() {
        let p = pack_at(SMALL_PACK);
        assert_eq!(p.checksum(), hex_to_id("0f3ea84cd1bba10c2a03d736a460635082833e59"));
    }

    #[test]
    fn verify_checksum() -> Result<(), Box<dyn std::error::Error>> {
        let p = pack_at(SMALL_PACK);
        assert_eq!(
            p.verify_checksum(&mut progress::Discard, &AtomicBool::new(false))?,
            p.checksum()
        );
        Ok(())
    }

    #[test]
    fn iter() -> Result<(), Box<dyn std::error::Error>> {
        let pack = pack_at(SMALL_PACK);
        let it = pack.streaming_iter()?;
        assert_eq!(it.count(), pack.num_objects() as usize);
        Ok(())
    }
}

/// All hardcoded offsets are obtained via `git pack-verify --verbose  tests/fixtures/packs/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx`
mod decode_entry {
    use bstr::ByteSlice;
    use gix_pack::{cache, data::decode::entry::ResolvedBase};

    use crate::{
        fixture_path, fixup,
        pack::{data::file::pack_at, SMALL_PACK},
    };

    fn content_of(path: &str) -> Vec<u8> {
        fixup(std::fs::read(fixture_path(path)).expect("valid fixture"))
    }

    #[test]
    fn commit() {
        let buf = decode_entry_at_offset(1968);
        assert_eq!(buf.len(), 187);
        assert_eq!(
            buf.capacity(),
            187,
            "for undeltified objects, there is no change in allocation or resizing"
        );
    }

    #[test]
    fn blob_ofs_delta_two_links() {
        let buf = decode_entry_at_offset(3033);
        assert_eq!(buf.len(), 173, "buffer length is the actual object size");
        assert_eq!(
            buf.capacity(),
            2381,
            "capacity is much higher as we allocate everything into a single, bigger, reusable buffer, which depends on base sizes"
        );
        assert_eq!(
            buf.as_bstr(),
            content_of("objects/b8aa61be84b78d7fcff788e8d844406cc97132bf.txt").as_bstr()
        );
    }

    #[test]
    fn blob_ofs_delta_single_link() {
        let buf = decode_entry_at_offset(3569);
        assert_eq!(buf.len(), 1163, "buffer length is the actual object size");
        assert_eq!(
            buf.capacity(),
            2398,
            "capacity is much higher as we allocate everything into a single, bigger, reusable buffer, which depends on base sizes"
        );
        assert_eq!(
            buf.as_bstr(),
            content_of("objects/f139391424a8c623adadf2388caec73e5e90865b.txt").as_bstr()
        );
    }

    /// Regression test for PR #2345: Ensures that when decompressing the base object in a delta chain,
    /// the output buffer is properly bounded to prevent the decompressor from overshooting and
    /// corrupting delta instruction data that follows in the buffer.
    ///
    /// ## Background
    /// When resolving delta chains, the code allocates a buffer structured as:
    /// `[first_buffer][second_buffer][delta_instructions]`
    /// The fix in PR #2345 bounds the output buffer passed to `decompress_entry_from_data_offset`
    /// to only `[first_buffer][second_buffer]` (i.e., `out_size - total_delta_data_size`), preventing
    /// the decompressor from writing beyond this boundary and corrupting the delta instructions.
    ///
    /// ## About this test
    /// This test uses a specially crafted pack file (pack-regression-*.pack) with a large base
    /// object (52KB) and delta chains to exercise the buffer bounding code path. While this test
    /// currently does not fail when the fix is removed (because triggering the actual zlib-rs
    /// overshooting behavior requires very specific compression/decompression conditions found in
    /// repositories like chromium), it:
    ///
    /// 1. **Exercises the correct code path**: Tests the delta resolution logic where the buffer
    ///    bounding fix is applied
    /// 2. **Documents the fix**: Serves as in-code documentation of PR #2345 and why buffer bounding
    ///    is necessary
    /// 3. **Provides infrastructure**: If a reproducing pack file is obtained (e.g., from chromium),
    ///    it can be easily added here
    /// 4. **Validates correctness**: Ensures delta chains decode correctly with the fix in place
    ///
    /// The actual bug manifests when zlib-rs (or potentially other decompressors) write slightly
    /// beyond the decompressed size when given an unbounded buffer, corrupting the delta
    /// instructions that follow in memory. This is highly dependent on the specific compression
    /// ratios and internal zlib-rs behavior.
    #[test]
    fn regression_delta_decompression_buffer_bound() {
        const REGRESSION_PACK: &str = "objects/pack-regression-bd7158957832e5b7b85af809fc317508121192f1.pack";
        
        #[allow(clippy::ptr_arg)]
        fn resolve_with_panic(_oid: &gix_hash::oid, _out: &mut Vec<u8>) -> Option<ResolvedBase> {
            panic!("should not want to resolve an id here")
        }

        let p = pack_at(REGRESSION_PACK);
        
        // Test the base object at offset 730 (ed2a638b) - 52000 bytes uncompressed
        // This is the large base that the deltas reference
        let entry = p.entry(730).expect("valid object at offset 730");
        let mut buf = Vec::new();
        let result = p.decode_entry(
            entry,
            &mut buf,
            &mut Default::default(),
            &resolve_with_panic,
            &mut cache::Never,
        );
        
        assert!(
            result.is_ok(),
            "Base object should decode correctly"
        );
        assert_eq!(buf.len(), 52000, "Base object should be 52000 bytes");
        
        // Test delta objects with chain length = 1
        // These objects delta against the large base, exercising the critical code path
        // where the base object is decompressed with a bounded output buffer.
        
        // Object 7a035d07 at offset 1141 (delta chain length 1)
        let entry = p.entry(1141).expect("valid object at offset 1141");
        let mut buf = Vec::new();
        let result = p.decode_entry(
            entry,
            &mut buf,
            &mut Default::default(),
            &resolve_with_panic,
            &mut cache::Never,
        );
        
        assert!(
            result.is_ok(),
            "Delta with chain length 1 should decode correctly with bounded buffer. \
             Without the fix, buffer overflow would corrupt delta instructions causing decode to fail."
        );
        assert!(!buf.is_empty(), "Decoded object should not be empty");
        
        // Object e2ace3ae at offset 1222 (delta chain length 1)
        let entry = p.entry(1222).expect("valid object at offset 1222");
        let mut buf = Vec::new();
        let result = p.decode_entry(
            entry,
            &mut buf,
            &mut Default::default(),
            &resolve_with_panic,
            &mut cache::Never,
        );
        
        assert!(
            result.is_ok(),
            "Second delta should decode correctly with bounded buffer"
        );
        assert!(!buf.is_empty(), "Decoded object should not be empty");
        
        // Object 8f3fd104 at offset 1305 (delta chain length 1)
        let entry = p.entry(1305).expect("valid object at offset 1305");
        let mut buf = Vec::new();
        let result = p.decode_entry(
            entry,
            &mut buf,
            &mut Default::default(),
            &resolve_with_panic,
            &mut cache::Never,
        );
        
        assert!(
            result.is_ok(),
            "Third delta should decode correctly with bounded buffer"
        );
        assert!(!buf.is_empty(), "Decoded object should not be empty");
    }

    fn decode_entry_at_offset(offset: u64) -> Vec<u8> {
        #[allow(clippy::ptr_arg)]
        fn resolve_with_panic(_oid: &gix_hash::oid, _out: &mut Vec<u8>) -> Option<ResolvedBase> {
            panic!("should not want to resolve an id here")
        }

        let p = pack_at(SMALL_PACK);
        let entry = p.entry(offset).expect("valid object type");
        let mut buf = Vec::new();
        p.decode_entry(
            entry,
            &mut buf,
            &mut Default::default(),
            &resolve_with_panic,
            &mut cache::Never,
        )
        .expect("valid offset provides valid entry");
        buf
    }
}

/// All hardcoded offsets are obtained via `git pack-verify --verbose  tests/fixtures/packs/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx`
mod resolve_header {
    use crate::pack::{data::file::pack_at, SMALL_PACK};

    #[test]
    fn commit() {
        let out = resolve_header_at_offset(1968);
        assert_eq!(out.kind, gix_object::Kind::Commit);
        assert_eq!(out.object_size, 187);
        assert_eq!(out.num_deltas, 0);
    }

    #[test]
    fn blob_ofs_delta_two_links() {
        let out = resolve_header_at_offset(3033);
        assert_eq!(out.kind, gix_object::Kind::Blob);
        assert_eq!(out.object_size, 173);
        assert_eq!(out.num_deltas, 2);
    }

    #[test]
    fn blob_ofs_delta_single_link() {
        let out = resolve_header_at_offset(3569);
        assert_eq!(out.kind, gix_object::Kind::Blob);
        assert_eq!(out.object_size, 1163);
        assert_eq!(out.num_deltas, 1);
    }

    #[test]
    fn tree() {
        let out = resolve_header_at_offset(2097);
        assert_eq!(out.kind, gix_object::Kind::Tree);
        assert_eq!(out.object_size, 34);
        assert_eq!(out.num_deltas, 0);
    }

    fn resolve_header_at_offset(offset: u64) -> gix_pack::data::decode::header::Outcome {
        fn resolve_with_panic(_oid: &gix_hash::oid) -> Option<gix_pack::data::decode::header::ResolvedBase> {
            panic!("should not want to resolve an id here")
        }

        let p = pack_at(SMALL_PACK);
        let entry = p.entry(offset).expect("valid object type");
        p.decode_header(entry, &mut Default::default(), &resolve_with_panic)
            .expect("valid offset provides valid entry")
    }
}

mod decompress_entry {
    use gix_object::bstr::ByteSlice;

    use crate::pack::{data::file::pack_at, SMALL_PACK};

    #[test]
    fn commit() {
        let buf = decompress_entry_at_offset(1968);
        assert_eq!(buf.as_bstr(), b"tree e90926b07092bccb7bf7da445fae6ffdfacf3eae\nauthor Sebastian Thiel <byronimo@gmail.com> 1286529993 +0200\ncommitter Sebastian Thiel <byronimo@gmail.com> 1286529993 +0200\n\nInitial commit\n".as_bstr());
        assert_eq!(buf.len(), 187);
    }

    #[test]
    fn blob() {
        let buf = decompress_entry_at_offset(2142);
        assert_eq!(
            buf.as_bstr(),
            b"GitPython is a python library used to interact with Git repositories.\n\nHi there\n\nHello Other\n"
                .as_bstr()
        );
        assert_eq!(buf.len(), 93);
    }

    #[test]
    fn blob_with_two_chain_links() {
        let buf = decompress_entry_at_offset(3033);
        assert_eq!(buf.len(), 6, "it decompresses delta objects, but won't resolve them");
    }

    #[test]
    fn tree() {
        let buf = decompress_entry_at_offset(2097);
        assert_eq!(buf[..13].as_bstr(), b"100644 README".as_bstr());
        assert_eq!(buf.len(), 34);
        assert_eq!(
            buf.capacity(),
            34,
            "capacity must be controlled by the caller to be big enough"
        );
    }

    fn decompress_entry_at_offset(offset: u64) -> Vec<u8> {
        let p = pack_at(SMALL_PACK);
        let entry = p.entry(offset).expect("valid object type");

        let size = entry.decompressed_size as usize;
        let mut buf = vec![0; size];
        p.decompress_entry(&entry, &mut Default::default(), &mut buf)
            .expect("valid offset");

        buf.resize(entry.decompressed_size as usize, 0);
        buf
    }
}
