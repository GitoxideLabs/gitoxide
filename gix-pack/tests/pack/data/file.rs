use gix_odb::pack;

use crate::fixture_path;

fn pack_at(at: &str) -> pack::data::File {
    pack::data::File::at(fixture_path(at).as_path(), gix_hash::Kind::Sha1).expect("valid pack file")
}

#[test]
fn unresolved_delta_base_is_not_found() {
    use gix_error::{ErrorExt, message};

    let base_id = gix_hash::ObjectId::empty_blob(gix_hash::Kind::Sha1);
    let err = pack::data::decode::DeltaBaseUnresolved(base_id).and_raise(message("Could not decode object"));
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "an unresolved delta base is a missing object", @"
    Could not decode object

    Caused by:
        0: A delta chain could not be followed as the ref base with id Oid(1) could not be found
    ");
    assert!(err.is_not_found(), "an unresolved delta base is a missing object");
    assert!(
        err.probable_cause().is::<pack::data::decode::DeltaBaseUnresolved>(),
        "the missing delta base, not its classification marker or outer context, is the probable cause"
    );
    assert_eq!(
        err.downcast_any_ref::<pack::data::decode::DeltaBaseUnresolved>()
            .expect("the missing delta base is retained")
            .0,
        base_id,
        "the decode failure retains the missing object ID"
    );
}

mod method {
    use std::sync::atomic::AtomicBool;

    use gix_utils::progress;

    use crate::{SMALL_PACK, data::file::pack_at, pack_from_memory_at};

    #[test]
    fn checksum() {
        let p = pack_at(SMALL_PACK);
        assert_eq!(p.checksum(), "0f3ea84cd1bba10c2a03d736a460635082833e59");
    }

    #[test]
    fn verify_checksum() -> gix_testtools::TestResult {
        let p = pack_at(SMALL_PACK);
        assert_eq!(
            p.verify_checksum(&mut progress::Discard, &AtomicBool::new(false))?,
            p.checksum()
        );
        Ok(())
    }

    #[test]
    fn interrupted_checksum_preserves_cancellation_and_io_source() {
        let p = pack_at(SMALL_PACK);
        let err = p
            .verify_checksum(&mut progress::Discard, &AtomicBool::new(true))
            .expect_err("the interrupt flag cancels hashing the pack");
        assert!(err.is_cancelled(), "verification preserves the hashing cancellation");
        assert!(
            !err.is_retryable(),
            "verification must not reclassify cancellation as retryable"
        );
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(std::io::ErrorKind::Interrupted),
            "the original hashing I/O interruption survives"
        );
    }

    #[test]
    fn verify_checksum_from_memory() -> gix_testtools::TestResult {
        let p = pack_from_memory_at(SMALL_PACK);
        assert_eq!(
            p.verify_checksum(&mut progress::Discard, &AtomicBool::new(false))?,
            p.checksum()
        );
        Ok(())
    }

    #[test]
    fn iter() -> gix_testtools::TestResult {
        let pack = pack_at(SMALL_PACK);
        let it = pack.streaming_iter()?;
        assert_eq!(it.count(), pack.num_objects() as usize);
        Ok(())
    }
}

/// All hardcoded offsets are obtained via `git pack-verify --verbose  tests/fixtures/packs/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx`
mod decode_entry {
    use bstr::ByteSlice;
    use gix_error::Result;
    use gix_pack::{cache, data::decode::entry::ResolvedBase};

    use crate::{SMALL_PACK, data::file::pack_at, fixture_path, fixup};

    const BLOB_DELTA_OFFSET: u64 = 3569;

    fn content_of(path: &str) -> Vec<u8> {
        fixup(std::fs::read(fixture_path(path)).expect("valid fixture"))
    }

    #[test]
    fn cache_keys_are_stable_only_for_the_open_pack() -> gix_testtools::TestResult {
        #[derive(Default)]
        struct RecordKey(Option<cache::Id>);

        impl cache::DecodeEntry for RecordKey {
            fn put(&mut self, pack_id: cache::Id, _offset: u64, _data: &[u8], _kind: gix_object::Kind, _size: usize) {
                assert_eq!(self.0, Some(pack_id), "cache reads and writes use the same key");
            }

            fn get(
                &mut self,
                pack_id: cache::Id,
                _offset: u64,
                _out: &mut Vec<u8>,
            ) -> Option<(gix_object::Kind, usize)> {
                self.0 = Some(pack_id);
                None
            }
        }

        let key = |pack: &gix_pack::data::File| -> gix_testtools::TestResult<cache::Id> {
            let mut cache = RecordKey::default();
            pack.decode_entry(
                pack.entry(BLOB_DELTA_OFFSET)?,
                &mut Vec::new(),
                &mut Default::default(),
                &|_, _| Ok(None),
                &mut cache,
            )?;
            Ok(cache.0.expect("the fixture delta was looked up in the cache"))
        };

        let pack = pack_at(SMALL_PACK);
        let pack_id = pack.id;
        let first = key(&pack)?;
        assert_eq!(key(&pack)?, first, "one open pack reuses its cache entries");
        drop(pack);

        let reopened = pack_at(SMALL_PACK);
        assert_eq!(
            reopened.id, pack_id,
            "the public location ID still identifies the same path"
        );
        assert_ne!(
            key(&reopened)?,
            first,
            "reopening a pack cannot reuse a stale cache key"
        );
        Ok(())
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
        let buf = decode_entry_at_offset(BLOB_DELTA_OFFSET);
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

    fn decode_entry_at_offset(offset: u64) -> Vec<u8> {
        fn resolve_with_panic(_oid: &gix_hash::oid, _out: &mut Vec<u8>) -> Result<Option<ResolvedBase>> {
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
    use gix_error::Result;

    use crate::{SMALL_PACK, data::file::pack_at};

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
        fn resolve_with_panic(_oid: &gix_hash::oid) -> Result<Option<gix_pack::data::decode::header::ResolvedBase>> {
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

    use crate::{SMALL_PACK, data::file::pack_at};

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

    #[test]
    fn decompress_into_caller_provided_buffer() {
        let p = pack_at(SMALL_PACK);
        let entry = p.entry(1968).expect("valid object type");
        let mut buf = vec![0; entry.decompressed_size as usize];

        p.decompress_entry(&entry, &mut Default::default(), &mut buf)
            .expect("the caller provided enough space for the decompressed object");

        assert_eq!(
            buf.len(),
            187,
            "the buffer is larger than the alloc limit and that's alright"
        );
    }

    #[test]
    fn caller_provided_buffer_must_be_large_enough() {
        let p = pack_at(SMALL_PACK);
        let entry = p.entry(1968).expect("valid object type");
        let mut buf = vec![0; entry.decompressed_size as usize - 1];

        let err = p
            .decompress_entry(&entry, &mut Default::default(), &mut buf)
            .expect_err("an undersized caller-provided buffer is invalid input");
        insta::assert_debug_snapshot!(err, "caller provided buffer must be large enough", @"Output buffer is too small for the decompressed entry");
        assert!(err.is_validation());
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
