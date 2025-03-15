use gix_fs::SharedFileSnapshotMut;
use std::path::Path;

#[test]
fn journey() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempfile::tempdir().unwrap();
    if !has_nanosecond_times(tmp.path())? {
        return Ok(());
    }

    let file_path = tmp.path().join("content");
    let smut = SharedFileSnapshotMut::<String>::new();

    let check = || file_path.metadata().ok()?.modified().ok();
    let open = || {
        Ok(match std::fs::read_to_string(&file_path) {
            Ok(s) => Some(s),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(err),
        })
    };
    let snap = smut.recent_snapshot(check, open)?;
    assert!(snap.is_none());

    std::fs::write(&file_path, "content")?;
    let snap = smut.recent_snapshot(check, open)?.expect("content read");
    assert_eq!(&**snap, "content", "it read the file for the first time");

    std::fs::write(&file_path, "change")?;
    let snap = smut.recent_snapshot(check, open)?.expect("content read");
    assert_eq!(&**snap, "change", "it picks up the change");

    std::fs::remove_file(&file_path)?;
    let snap = smut.recent_snapshot(check, open)?;
    assert!(snap.is_none(), "file deleted, nothing to see here");

    std::fs::write(&file_path, "new")?;
    let snap = smut.recent_snapshot(check, open)?.expect("content read again");
    let owned: String = snap.into_owned_or_cloned();
    assert_eq!(owned, "new", "owned versions are possible easily and efficiently");
    Ok(())
}

fn has_nanosecond_times(root: &Path) -> std::io::Result<bool> {
    let test_file = root.join("nanosecond-test");

    std::fs::write(&test_file, "a")?;
    let first_time = test_file.metadata()?.modified()?;

    std::fs::write(&test_file, "b")?;
    let second_time = test_file.metadata()?.modified()?;

    Ok(first_time != second_time)
}
