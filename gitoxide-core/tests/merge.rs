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
    bytes[bitmap..bitmap + 4].copy_from_slice(&3u32.to_be_bytes());
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
    update_head(&repo, "HEAD")?;

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
        2,
        "the rebuilt index retains its tracked entries"
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

#[test]
fn merge_indexes_remap_monitor_validity_and_keep_unaffected_listings_on_disk() -> TestResult {
    use gix::index::{entry::Flags, extension::fs_monitor::Token, write};

    let fixture = gix_testtools::scripted_fixture_writable("merge.sh")?;
    let repo = gix::open_opts(fixture.path(), gix::open::Options::isolated())?;
    let mut existing = repo.open_index()?;
    let cache = existing.set_untracked(None).expect("Git warmed the untracked cache");
    for entry in existing.entries_mut() {
        entry.flags.insert(Flags::FSMONITOR_VALID);
    }
    let token = Token::V2 {
        token: "before-merge".into(),
    };
    existing.set_fs_monitor_token(Some(token.clone()));
    existing.set_untracked(Some(cache));
    let options = write::Options {
        extensions: write::Extensions::AllUnconditionally,
        ..Default::default()
    };
    existing.write(options, repo.refs.shared_repository_permissions)?;

    let head_tree_id = repo.head_tree_id()?;
    let incoming_tree_id = repo.rev_parse_single("incoming^{tree}")?;
    let outcome = repo.merge_trees(
        head_tree_id,
        head_tree_id,
        incoming_tree_id,
        Default::default(),
        repo.tree_merge_options()?,
    )?;
    let mut planned = outcome.index_file(gix::merge::tree::TreatAsUnresolved::git(), Some(&existing))?;
    planned.set_path(fixture.path().join("planned-index"));
    planned.write(options, repo.refs.shared_repository_permissions)?;
    update_head(&repo, "incoming")?;
    for path in [planned.path(), &repo.index_path()] {
        let written = gix::index::File::at(path, repo.object_hash(), false, Default::default())?;
        let monitor = written.fs_monitor().expect("the rebuilt index retains its checkpoint");
        assert_eq!(monitor.token(), &token, "rebuilding does not advance the monitor");
        assert_eq!(
            monitor.valid_entries(written.entries().len()),
            Some(vec![false, false, true]),
            "a is new, file changed, and stable/kept moved from position one to two"
        );
        let cache = written.untracked().expect("the rebuilt index retains UNTR");
        assert!(
            cache.directories().iter().any(|dir| dir.name() == "stable"
                && dir.stat().is_some()
                && dir.untracked_entries().iter().any(|name| name == "visible")),
            "the unaffected listing survives serialization"
        );
    }
    Ok(())
}

fn update_head(repo: &gix::Repository, theirs: &str) -> gix::Result<()> {
    merge::tree(
        repo.clone(),
        &mut Vec::new(),
        &mut Vec::new(),
        "HEAD".into(),
        "HEAD".into(),
        theirs.into(),
        merge::tree::Options {
            format: OutputFormat::Human,
            file_favor: None,
            tree_favor: None,
            in_memory: false,
            debug: false,
            message: Some("rebuild the index".into()),
            update_head: true,
        },
    )
}
