use gix_pack::mtimes::{self, Table};
use gix_testtools::TestResult;

#[test]
fn read_and_write_git_cruft_tables_for_both_hashes() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_cruft_mtimes.sh")?;
    for hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
        let repo = fixture.join(hash.to_string());
        let sidecar = std::fs::read_dir(repo.join("objects/pack"))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?
            .into_iter()
            .find(|path| path.extension().is_some_and(|ext| ext == "mtimes"))
            .expect("Git wrote a cruft pack");
        let bytes = std::fs::read(&sidecar)?;
        let index = gix_pack::index::File::at(sidecar.with_extension("idx"), hash)?;
        let table = Table::from_bytes(&bytes, index.num_objects(), &index.pack_checksum())?;
        assert_eq!(
            table.pack_checksum(),
            index.pack_checksum(),
            "table identifies its pack"
        );
        let old = std::fs::read_to_string(repo.join("old-id"))?;
        let new = std::fs::read_to_string(repo.join("new-id"))?;
        let old = gix_hash::ObjectId::from_hex(old.trim().as_bytes())?;
        let new = gix_hash::ObjectId::from_hex(new.trim().as_bytes())?;
        for (position, entry) in index.iter().enumerate() {
            let expected = if entry.oid == old {
                978_307_200
            } else {
                assert_eq!(entry.oid, new, "the other entry is the newer blob");
                1_009_843_200
            };
            assert_eq!(
                table.get(position as u32),
                Some(expected),
                "Git preserves each object's original time"
            );
        }
        assert_eq!(table.get(index.num_objects()), None, "out-of-range access is checked");
        let mut output = Vec::new();
        let checksum = mtimes::write(&table.iter().collect::<Vec<_>>(), &index.pack_checksum(), &mut output)?;
        assert_eq!(output, bytes, "native encoding matches Git byte for byte");
        assert_eq!(
            output.get(output.len() - hash.len_in_bytes()..),
            Some(checksum.as_slice()),
            "the checksum terminates the file"
        );
    }
    Ok(())
}

#[test]
fn malformed_or_mismatched_tables_are_rejected() -> TestResult {
    for hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
        let pack = hash.empty_tree();
        let mut bytes = Vec::new();
        mtimes::write(&[0, u32::MAX], &pack, &mut bytes)?;
        let table = Table::from_bytes(&bytes, 2, &pack)?;
        assert_eq!(
            table.iter().collect::<Vec<_>>(),
            [0, u32::MAX],
            "all u32 timestamps round trip"
        );
        for position in [0, 7, 11, 12, bytes.len() - 1] {
            let mut damaged = bytes.clone();
            damaged[position] ^= 1;
            Table::from_bytes(&damaged, 2, &pack).expect_err("damaged metadata must not influence object expiry");
        }
        for len in [0, 11, bytes.len() - 1] {
            Table::from_bytes(&bytes[..len], 2, &pack).expect_err("truncated metadata is rejected");
        }
        Table::from_bytes(&bytes, 1, &pack).expect_err("the object count must agree with the index");
        Table::from_bytes(&bytes, 2, &hash.empty_blob()).expect_err("another pack's timestamps must not be reused");
        let mut empty = Vec::new();
        mtimes::write(&[], &pack, &mut empty)?;
        assert_eq!(
            Table::from_bytes(&empty, 0, &pack)?.iter().len(),
            0,
            "empty tables still have valid headers and trailers"
        );
    }
    Ok(())
}
