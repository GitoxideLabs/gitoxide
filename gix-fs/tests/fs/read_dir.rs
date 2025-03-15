#[test]
fn with_precomposed_unicode() -> crate::Result {
    let tmp = tempfile::tempdir()?;

    let decomposed = "a\u{308}";

    let root = tmp.path().join(decomposed);
    std::fs::create_dir(&root)?;
    std::fs::write(root.join(decomposed), [])?;

    let precomposed = "ä";
    for entry in gix_fs::read_dir(&root, true)? {
        let entry = entry?;
        assert_eq!(
            entry.file_name().to_str().unwrap(),
            precomposed,
            "precomposition is applied"
        );
        assert_eq!(
            entry.path().parent().unwrap().file_name().unwrap(),
            precomposed,
            "precomposition is applied for the whole path"
        );
    }

    for entry in gix_fs::read_dir(&root, false)? {
        let entry = entry?;
        assert_eq!(
            entry.file_name().to_str().unwrap(),
            decomposed,
            "by default, the path is unchanged"
        );
        assert_eq!(
            entry.path().parent().unwrap().file_name().unwrap(),
            decomposed,
            "the same is true for the parent path"
        );
    }

    Ok(())
}
