use gix_odb::pack;
use gix_testtools::size_ok;

#[test]
fn size_of_entry() {
    let actual = std::mem::size_of::<pack::data::input::Entry>();
    let sha1 = 104;
    let sha256_extra = 32;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "let's keep the size in check as we have many of them: {actual} <~ {expected}"
    );
}

mod new_from_header {
    use std::fs;

    use gix_odb::{
        pack,
        pack::data::input::{EntryDataMode, Mode},
    };

    use crate::{
        fixture_path,
        pack::{SMALL_PACK, V2_PACKS_AND_INDICES},
    };

    #[test]
    fn header_encode() -> Result<(), Box<dyn std::error::Error>> {
        for (_, data_file) in V2_PACKS_AND_INDICES {
            let data = fs::read(fixture_path(data_file))?;
            for entry in pack::data::input::BytesToEntriesIter::new_from_header(
                std::io::BufReader::new(data.as_slice()),
                Mode::AsIs,
                EntryDataMode::Ignore,
                gix_hash::Kind::Sha1,
            )? {
                let entry = entry?;

                let mut buf = Vec::<u8>::new();
                entry.header.write_to(entry.decompressed_size, &mut buf)?;
                let new_entry =
                    pack::data::Entry::from_bytes(&buf, entry.pack_offset, gix_hash::Kind::Sha1.len_in_bytes())?;

                assert_eq!(
                    new_entry.header_size(),
                    buf.len(),
                    "it should consume all provided bytes"
                );
                assert_eq!(
                    new_entry.decompressed_size, entry.decompressed_size,
                    "decoded size must match"
                );
                assert_eq!(new_entry.header, entry.header, "headers match after roundtrip");
            }
        }
        Ok(())
    }

    #[test]
    fn generic_iteration() -> Result<(), Box<dyn std::error::Error>> {
        for compression_mode in &[
            EntryDataMode::Ignore,
            EntryDataMode::Keep,
            EntryDataMode::Crc32,
            EntryDataMode::KeepAndCrc32,
        ] {
            for trailer_mode in &[Mode::AsIs, Mode::Verify, Mode::Restore] {
                let mut iter = pack::data::input::BytesToEntriesIter::new_from_header(
                    std::io::BufReader::new(fs::File::open(fixture_path(SMALL_PACK))?),
                    *trailer_mode,
                    *compression_mode,
                    gix_hash::Kind::Sha1,
                )?;

                let num_objects = iter.len();
                assert_eq!(iter.version(), pack::data::Version::V2);
                assert_eq!(num_objects, 42);
                assert_eq!(iter.by_ref().take(42 - 1).count(), num_objects - 1);
                assert_eq!(iter.len(), 1);
                let entry = iter.next().expect("last object")?;
                assert_eq!(
                    entry.trailer.expect("trailer id"),
                    pack::data::File::at(fixture_path(SMALL_PACK), gix_hash::Kind::Sha1)?.checksum(),
                    "last object contains the trailer - a hash over all bytes in the pack"
                );
                assert_eq!(iter.len(), 0);

                assert_eq!(entry.crc32.is_none(), !compression_mode.crc32());
                assert_eq!(entry.compressed.is_none(), !compression_mode.keep());
                if compression_mode.crc32() {
                    assert_eq!(entry.crc32.expect("crc32 computed"), 3627824263);
                }
                if compression_mode.keep() {
                    assert_eq!(
                        entry
                            .compressed
                            .expect("bytes present when keeping compressed bytes")
                            .len(),
                        46
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn restore_missing_trailer() -> Result<(), Box<dyn std::error::Error>> {
        let pack = fs::read(fixture_path(SMALL_PACK))?;
        let mut iter = pack::data::input::BytesToEntriesIter::new_from_header(
            std::io::BufReader::new(&pack[..pack.len() - 20]),
            Mode::Restore,
            EntryDataMode::Ignore,
            gix_hash::Kind::Sha1,
        )?;
        let num_objects = iter.len();
        assert_eq!(iter.by_ref().take(42 - 1).count(), num_objects - 1);
        assert_eq!(
            iter.next().expect("last object")?.trailer.expect("trailer id"),
            pack::data::File::at(fixture_path(SMALL_PACK), gix_hash::Kind::Sha1)?.checksum(),
            "the correct checksum should be restored"
        );
        Ok(())
    }

    #[test]
    fn restore_partial_pack() -> Result<(), Box<dyn std::error::Error>> {
        let pack = fs::read(fixture_path(SMALL_PACK))?;
        let mut iter = pack::data::input::BytesToEntriesIter::new_from_header(
            std::io::BufReader::new(&pack[..pack.len() / 2]),
            Mode::Restore,
            EntryDataMode::Ignore,
            gix_hash::Kind::Sha1,
        )?;
        let mut num_objects = 0;
        for entry in iter.by_ref() {
            let entry = entry?;
            num_objects += 1;
            assert!(
                entry.trailer.is_some(),
                "every entry has a trailer as we don't know when an object will fail - thus we never fail"
            );
        }
        assert_eq!(num_objects, 12);
        assert_eq!(
            iter.len(),
            0,
            "it will never return any more objects (right now), so nothing is left"
        );
        Ok(())
    }
}
