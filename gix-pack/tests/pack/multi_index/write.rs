use std::{
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use gix_features::progress;
use gix_testtools::fixture_path;

/// Writes a multi-index from the static SHA-1 pack indices, with pinned SHA-1 expectations.
/// The SHA-256 counterpart lives in [`from_a_hash_parameterized_pack`] below.
#[test]
fn from_paths() -> crate::Result {
    let pack_dir = fixture_path("objects/pack");
    let written = write_multi_index_from_pack_dir(&pack_dir, gix_hash::Kind::Sha1)?;
    assert_eq!(written.input_indices.len(), 3);

    assert_eq!(
        written.outcome.multi_index_checksum,
        "d34d327039a3554f8a644b29e07b903fa71ef269"
    );

    assert_eq!(written.file.num_indices(), 3);
    assert_eq!(
        written.file.index_names(),
        vec![
            PathBuf::from("pack-11fdfa9e156ab73caae3b6da867192221f2089c2.idx"),
            PathBuf::from("pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx"),
            PathBuf::from("pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx"),
        ]
    );
    assert_eq!(written.file.num_objects(), 139);
    assert_eq!(written.file.checksum(), written.outcome.multi_index_checksum);

    written.verify_integrity_with_referenced_packs()?;
    Ok(())
}

/// Like [`from_paths`], but sources its input index from the hash-parameterized fixture so the
/// writer runs under both SHA-1 and SHA-256. The fixture's gc leaves one pack, hence one index.
#[test]
fn from_a_hash_parameterized_pack() -> crate::Result {
    let object_hash = crate::object_hash();
    let pack_dir = crate::scripted_fixture_read_only("make_pack_gen_repo_multi_index.sh")?.join(".git/objects/pack");
    let written = write_multi_index_from_pack_dir(&pack_dir, object_hash)?;
    assert_eq!(written.input_indices.len(), 1, "the aggressive gc leaves a single pack");

    assert_eq!(
        written.file.object_hash(),
        object_hash,
        "the writer records the requested object hash"
    );
    assert_eq!(written.file.num_indices(), 1);
    assert_eq!(written.file.num_objects(), 868);
    assert_eq!(written.file.checksum(), written.outcome.multi_index_checksum);

    written.verify_integrity_with_referenced_packs()?;
    Ok(())
}

struct WrittenMultiIndex {
    file: gix_pack::multi_index::File,
    dir: gix_testtools::tempfile::TempDir,
    input_indices: Vec<PathBuf>,
    outcome: gix_pack::multi_index::write::Outcome,
}

impl WrittenMultiIndex {
    fn verify_integrity_with_referenced_packs(&self) -> crate::Result {
        // Place the referenced pack and index next to the multi-index so integrity can resolve them.
        for ro_index in &self.input_indices {
            std::fs::copy(
                ro_index,
                self.dir
                    .path()
                    .join(ro_index.file_name().expect("index paths have file names")),
            )?;
            let ro_pack = ro_index.with_extension("pack");
            std::fs::copy(
                &ro_pack,
                self.dir
                    .path()
                    .join(ro_pack.file_name().expect("pack paths have file names")),
            )?;
        }

        assert_eq!(
            self.file
                .verify_integrity(&mut progress::Discard, &AtomicBool::new(false), Default::default())?
                .actual_index_checksum,
            self.outcome.multi_index_checksum,
            "full integrity verification returns the written multi-index checksum"
        );
        assert_eq!(
            self.file
                .verify_integrity_fast(&mut progress::Discard, &AtomicBool::new(false))?,
            self.file.checksum(),
            "fast integrity verification returns the checksum read from the file"
        );
        Ok(())
    }
}

fn write_multi_index_from_pack_dir(pack_dir: &Path, object_hash: gix_hash::Kind) -> crate::Result<WrittenMultiIndex> {
    let input_indices = std::fs::read_dir(pack_dir)?
        .filter_map(|r| {
            let idx_path = r.ok()?.path();
            (idx_path.extension()? == "idx").then_some(idx_path)
        })
        .collect::<Vec<_>>();

    let dir = gix_testtools::tempfile::TempDir::new()?;
    let output_path = dir.path().join("multi-pack-index");
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output_path)?;
    let outcome = gix_pack::multi_index::write_from_index_paths(
        input_indices.clone(),
        &mut out,
        &mut progress::Discard,
        &AtomicBool::new(false),
        gix_pack::multi_index::write::Options { object_hash },
    )?;
    let file = gix_pack::multi_index::File::at(output_path, None)?;

    Ok(WrittenMultiIndex {
        file,
        dir,
        input_indices,
        outcome,
    })
}
