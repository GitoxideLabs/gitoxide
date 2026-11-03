use filetime::FileTime;
use gix_index::{File, State, extension::fs_monitor::Token};
use gix_testtools::TestResult;

#[test]
fn git_untracked_caches_survive_index_writes() -> TestResult {
    for name in [
        "untracked_cache_empty",
        "untracked_cache_populated",
        "untracked_cache_nested",
    ] {
        let index = fixture(name)?;
        let expected = index.untracked().expect("Git produced an untracked cache");
        let mut bytes = Vec::new();
        index.write_to(&mut bytes, Default::default())?;
        let (written, _) = State::from_bytes(&bytes, FileTime::now(), index.object_hash(), Default::default())?;
        let actual = written.untracked().expect("writing retains the untracked cache");
        assert_eq!(
            format!("{actual:?}"),
            format!("{expected:?}"),
            "{name}: cache data and directory order survive serialization"
        );

        let output = gix_testtools::tempfile::TempDir::new()?;
        let index_path = output.path().join("index");
        std::fs::write(&index_path, bytes)?;
        let root = index
            .path()
            .parent()
            .and_then(std::path::Path::parent)
            .expect("fixture index is in .git");
        let result = gix_testtools::git_command(root)
            .env("GIT_INDEX_FILE", &index_path)
            .args(["update-index", "--index-version=2"])
            .output()?;
        assert!(
            result.status.success(),
            "Git can rewrite the index: {:?}",
            result.stderr
        );
        let from_git = File::at(&index_path, index.object_hash(), false, Default::default())?;
        assert_eq!(
            format!("{:?}", from_git.untracked()),
            format!("{:?}", Some(expected)),
            "{name}: Git preserves the cache written by gix"
        );
    }
    Ok(())
}

#[test]
fn directory_children_must_have_unique_sorted_names() -> TestResult {
    let index = fixture("untracked_cache_nested")?;
    let mut bytes = Vec::new();
    index
        .untracked()
        .expect("nested fixture has sibling directories")
        .write_to(&mut bytes, index.object_hash())?;
    let name = b"untracked-dir-2\0";
    let position = bytes
        .windows(name.len())
        .position(|window| window == name)
        .expect("the directory name occurs in the serialized cache");
    let accepted: Vec<_> = b"34"
        .iter()
        .map(|&suffix| {
            let mut bytes = bytes.clone();
            bytes[position + name.len() - 2] = suffix;
            gix_index::extension::untracked_cache::decode(&bytes[8..], index.object_hash(), None).is_some()
        })
        .collect();
    assert_eq!(
        accepted,
        [false, false],
        "duplicate and unsorted sibling names must be rejected before binary lookup can trust their order"
    );
    Ok(())
}

#[test]
fn invalidation_covers_directory_descendants_and_ancestors() -> TestResult {
    let index = fixture("untracked_cache_nested")?;
    let original = index.untracked().expect("nested fixture has cached directories");
    for path in ["tracked-dir-with-ignore", "tracked-dir-with-ignore/"] {
        let mut cache = original.clone();
        cache.invalidate_path(path.into());
        for directory in cache.directories() {
            if [
                "",
                "tracked-dir-with-ignore",
                "nested-untracked-dir",
                "deep-untracked-dir",
            ]
            .iter()
            .any(|name| directory.name() == *name)
            {
                assert!(
                    directory.stat().is_none(),
                    "the affected subtree and its ancestors need scanning"
                );
                assert!(
                    directory.untracked_entries().is_empty(),
                    "invalidated listings cannot be reused"
                );
            } else {
                let before = original
                    .directories()
                    .iter()
                    .find(|before| before.name() == directory.name())
                    .expect("same directory graph");
                assert_eq!(
                    format!("{directory:?}"),
                    format!("{before:?}"),
                    "siblings keep their cache data"
                );
            }
        }
    }
    let mut cache = original.clone();
    cache.invalidate_path("tracked-dir-with-ignore/visible-untracked-file".into());
    let nested = cache
        .directories()
        .iter()
        .find(|dir| dir.name() == "nested-untracked-dir")
        .expect("nested directory exists");
    assert!(
        nested.stat().is_some(),
        "a file event does not invalidate unrelated descendant directories"
    );
    for path in ["/", "../outside", "tracked-dir-with-ignore//file", ""] {
        let mut cache = original.clone();
        cache.invalidate_path(path.into());
        assert!(
            cache
                .directories()
                .iter()
                .all(|dir| dir.stat().is_none() && dir.untracked_entries().is_empty()),
            "{path:?}: uncertain paths invalidate every directory"
        );
    }
    Ok(())
}

#[test]
fn mutation_and_monitor_advancement_discard_unvalidated_cache_data() -> TestResult {
    let mut index = fixture("untracked_cache_populated")?;
    let original = index.set_untracked(None).expect("fixture cache is present");
    assert!(
        index.untracked().is_none(),
        "taking the cache removes it from the index"
    );
    assert!(
        index.set_untracked(Some(original.clone())).is_none(),
        "restoring replaces the previous cache"
    );
    index.entries_mut();
    assert!(
        index.untracked().is_none(),
        "entry edits invalidate cached tracked/untracked membership"
    );

    index.set_untracked(Some(original.clone()));
    index.set_fs_monitor_token(Some(Token::V2 { token: "first".into() }));
    assert!(
        index.untracked().is_none(),
        "a new monitor checkpoint cannot retain unchecked cache data"
    );
    index.set_untracked(Some(original.clone()));
    index.set_fs_monitor_token(Some(Token::V2 { token: "first".into() }));
    assert!(
        index.untracked().is_some(),
        "the same checkpoint leaves cache validity unchanged"
    );
    index.set_fs_monitor_token(Some(Token::V2 { token: "second".into() }));
    assert!(
        index.untracked().is_none(),
        "advancing the checkpoint requires explicit cache validation"
    );
    index.set_untracked(Some(original));
    index.set_fs_monitor_token(None);
    assert!(
        index.untracked().is_none(),
        "removing the monitor also discards its unchecked cache"
    );
    Ok(())
}

#[test]
fn serialization_rejects_mismatched_hashes_and_truncation_is_not_a_cache() -> TestResult {
    let index = fixture("untracked_cache_nested")?;
    let cache = index.untracked().expect("nested fixture has exclusion object ids");
    let other_hash = match index.object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::Kind::Sha256,
        _ => gix_hash::Kind::Sha1,
    };
    let mut bytes = Vec::new();
    assert!(
        cache.write_to(&mut bytes, other_hash).is_err(),
        "mixed object-id lengths would corrupt UNTR"
    );
    assert!(
        bytes.is_empty(),
        "invalid cache data is rejected before any extension is emitted"
    );
    cache.write_to(&mut bytes, index.object_hash())?;
    let body = &bytes[8..];
    for len in 0..body.len() {
        assert!(
            gix_index::extension::untracked_cache::decode(&body[..len], index.object_hash(), None).is_none(),
            "truncated cache length {len} cannot be used"
        );
    }
    Ok(())
}

fn fixture(name: &str) -> TestResult<File> {
    Ok(File::at(
        crate::fixture_index_path_needs_archive(name),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?)
}
