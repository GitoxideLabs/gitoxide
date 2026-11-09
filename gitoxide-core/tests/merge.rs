use gitoxide_core::{OutputFormat, repository::merge};
use gix_testtools::TestResult;

#[test]
fn update_head_discards_malformed_optional_fsmonitor_data() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("merge.sh")?;
    let repo = gix::open_opts(fixture.path(), gix::open::Options::isolated())?;
    let commit_id = repo.head_id()?;
    let tree_id = repo.head_commit()?.tree_id()?;
    let mut index = repo.open_index()?;
    let blob_id = index.entries()[0].id;
    index.set_fs_monitor_token(Some(gix::index::extension::fs_monitor::Token::V1 {
        nanos_since_1970: 1,
    }));
    let mut bytes = Vec::new();
    index.write_to(&mut bytes, Default::default())?;
    let extension = bytes
        .windows(4)
        .position(|window| window == b"FSMN")
        .expect("the monitor extension was written");
    let bitmap = extension + 8 + 4 + 8 + 4;
    bytes[bitmap..bitmap + 4].copy_from_slice(&2u32.to_be_bytes());
    let checksum_offset = bytes.len() - repo.object_hash().len_in_bytes();
    let mut hasher = gix::hash::hasher(repo.object_hash());
    hasher.update(&bytes[..checksum_offset]);
    bytes[checksum_offset..].copy_from_slice(hasher.try_finalize()?.as_bytes());
    std::fs::write(repo.index_path(), bytes)?;

    let previous = repo.open_index()?;
    assert!(
        previous
            .fs_monitor()
            .expect("the optional extension can be decoded")
            .valid_entries(previous.entries().len())
            .is_none(),
        "the invalid bitmap, not index decoding or its checksum, triggers the fallback"
    );
    merge::tree(
        repo.clone(),
        &mut Vec::new(),
        &mut Vec::new(),
        "HEAD".into(),
        "HEAD".into(),
        "HEAD".into(),
        merge::tree::Options {
            format: OutputFormat::Human,
            file_favor: None,
            tree_favor: None,
            in_memory: false,
            debug: false,
            message: Some("rebuild the index".into()),
            update_head: true,
        },
    )?;

    assert_ne!(
        repo.head_id()?,
        commit_id,
        "optional cache corruption cannot prevent updating HEAD"
    );
    assert_eq!(
        repo.head_commit()?.tree_id()?,
        tree_id,
        "the merged tree remains correct"
    );
    let written = repo.open_index()?;
    assert_eq!(
        written.entries().len(),
        1,
        "the rebuilt index retains its tracked entry"
    );
    assert_eq!(
        written.entries()[0].id,
        blob_id,
        "the rebuilt entry retains its content"
    );
    assert!(
        written.fs_monitor().is_none(),
        "the unusable optional extension is discarded"
    );
    Ok(())
}
