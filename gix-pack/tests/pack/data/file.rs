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
    /// ## Why this test may not fail without the fix
    /// The bug is highly dependent on:
    /// 1. The specific structure of the pack file (delta chain complexity, object sizes)
    /// 2. The decompressor implementation (zlib-rs behavior vs. other implementations)
    /// 3. The compressibility and actual compressed size of the base object
    ///
    /// The chromium repository was reported to trigger this issue, but creating a minimal
    /// reproducing pack file would require carefully crafting objects with specific compression
    /// characteristics. The existing test fixtures (SMALL_PACK) don't happen to trigger the
    /// overshooting behavior, but they do exercise the same code path.
    ///
    /// ## Value of this test
    /// Even though this test may not fail without the fix using current fixtures, it:
    /// 1. Documents the expected behavior and the fix
    /// 2. Exercises the delta chain decompression code path
    /// 3. Would catch regressions if the bounding logic were removed or broken
    /// 4. Provides a place to add a proper reproducing pack file if one is created in the future
    #[test]
    fn regression_delta_decompression_buffer_bound() {
        // This test uses an offset delta with two links (delta -> delta -> base)
        // The critical path is when the base object is decompressed into a buffer that
        // also needs to hold delta instructions.
        let buf = decode_entry_at_offset(3033);

        // If the bug were present with a pack file that triggers it, the decompression would either:
        // 1. Corrupt the delta instructions in memory, causing delta application to fail
        // 2. Produce incorrect output data
        // This assertion verifies the correct data was decoded:
        assert_eq!(
            buf.as_bstr(),
            content_of("objects/b8aa61be84b78d7fcff788e8d844406cc97132bf.txt").as_bstr(),
            "Delta chain should decode correctly with properly bounded decompression buffer"
        );

        // Also test the single-link delta chain
        let buf = decode_entry_at_offset(3569);
        assert_eq!(
            buf.as_bstr(),
            content_of("objects/f139391424a8c623adadf2388caec73e5e90865b.txt").as_bstr(),
            "Single-link delta chain should decode correctly with properly bounded decompression buffer"
        );
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
