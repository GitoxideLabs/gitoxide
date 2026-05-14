mod metadata {
    use gix_index::fs::Metadata;

    #[test]
    fn from_path_no_follow() -> crate::Result {
        let root = crate::scripted_fixture_read_only("file_metadata.sh")?;

        // For now, don't assert on the values of the metadata as these depends on the filesystem,
        // which might truncate it, or fail entirely.
        for filename in ["future", "past"] {
            let meta = Metadata::from_path_no_follow(&root.join(filename))?;
            assert!(meta.created().is_some());
            assert!(meta.modified().is_some());
            assert_eq!(meta.len(), 0);
        }
        Ok(())
    }
}
