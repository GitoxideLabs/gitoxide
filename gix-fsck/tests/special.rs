use gix_fsck::objects::{MessageId, Options, Severity, SpecialBlobs};
use gix_testtools::TestResult;

#[test]
fn special_blob_diagnostics_match_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_special_object_validation.sh")?;
    let mut cases = 0;
    for case in std::fs::read_dir(&fixture)? {
        let case = case?.path();
        if !case.join("tree").is_file() {
            continue;
        }
        cases += 1;
        let tree_id = read_id(&case.join("tree-id"))?;
        let objects = gix_odb::at(case.join("repo/objects"), tree_id.kind())?;
        let mut options = Options::default();
        options.strict = true;
        if case.join("skiplist").is_file() {
            options.extend_skip_list(&std::fs::read(case.join("skiplist"))?, tree_id.kind())?;
        }
        let mut special = SpecialBlobs::default();
        let mut actual = special.observe_tree(tree_id, &std::fs::read(case.join("tree"))?, &options)?;
        actual.extend(special.validate(&objects, &options)?);
        let report = std::fs::read_to_string(case.join("report"))?;
        let mut expected = Vec::new();
        for line in report.lines() {
            let Some(severity) = (if line.starts_with("error") {
                Some(Severity::Error)
            } else if line.starts_with("warning") {
                Some(Severity::Warning)
            } else {
                None
            }) else {
                continue;
            };
            for token in line.split_ascii_whitespace() {
                if let Some(name) = token.strip_suffix(':')
                    && (name.starts_with("gitmodules")
                        || name.starts_with("gitattributes")
                        || name == "gitignoreSymlink"
                        || name == "mailmapSymlink")
                {
                    expected.push((name.parse::<MessageId>()?, severity));
                }
            }
        }
        let mut actual: Vec<_> = actual.into_iter().map(|d| (d.id, d.severity)).collect();
        actual.sort();
        expected.sort();
        expected.dedup();
        assert_eq!(actual, expected, "{}: {report}", case.display());
    }
    assert!(cases > 50, "the complete special-file and URL corpus ran");
    Ok(())
}

#[test]
fn skip_lists_severity_and_repeated_references() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_special_object_validation.sh")?;
    let case = fixture.join("command-update");
    let tree_id = read_id(&case.join("tree-id"))?;
    let blob_id = read_id(&case.join("blob-id"))?;
    let objects = gix_odb::at(case.join("repo/objects"), tree_id.kind())?;
    let tree = std::fs::read(case.join("tree"))?;
    let mut options = Options::default();
    let mut special = SpecialBlobs::default();
    special.observe_tree(tree_id, &tree, &options)?;
    special.observe_tree(tree_id, &tree, &options)?;
    let actual = special.validate(&objects, &options)?;
    assert_eq!(actual.len(), 1, "shared blobs are checked once");
    assert_eq!(actual[0].object_id, blob_id, "content diagnostics identify the blob");
    options.set_message("gitmodulesUpdate", "warn")?;
    assert_eq!(special.validate(&objects, &options)?[0].severity, Severity::Warning);
    options.skip_object(blob_id);
    assert!(
        special.validate(&objects, &options)?.is_empty(),
        "skipped blobs are exempt"
    );
    let mut special = SpecialBlobs::default();
    options.skip_object(tree_id);
    special.observe_tree(tree_id, &tree, &options)?;
    assert_eq!(
        special.validate(&objects, &Options::default())?[0].id,
        MessageId::GitModulesUpdate,
        "skipped trees still discover special blobs; skipping the blob is independent"
    );
    let case = fixture.join("modules-symlink");
    let tree_id = read_id(&case.join("tree-id"))?;
    let actual = special.observe_tree(tree_id, &std::fs::read(case.join("tree"))?, &Options::default())?;
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].id, MessageId::GitModulesSymlink);
    assert_eq!(
        actual[0].object_id, tree_id,
        "symlink diagnostics identify the containing tree"
    );
    Ok(())
}

fn read_id(path: &std::path::Path) -> gix_testtools::Result<gix_hash::ObjectId> {
    Ok(gix_hash::ObjectId::from_hex(
        std::fs::read_to_string(path)?.trim().as_bytes(),
    )?)
}

#[test]
fn object_store_failures_keep_their_cause() -> TestResult {
    use gix_error::ErrorExt;

    struct Unreadable;
    impl gix_object::Find for Unreadable {
        fn try_find<'a>(
            &self,
            _id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            Err(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "test object access denied").raise())
        }
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_special_object_validation.sh")?;
    let case = fixture.join("valid-modules");
    let tree_id = read_id(&case.join("tree-id"))?;
    let blob_id = read_id(&case.join("blob-id"))?;
    let options = Options::default();
    let mut special = SpecialBlobs::default();
    special.observe_tree(tree_id, &std::fs::read(case.join("tree"))?, &options)?;
    let error = special
        .validate(&Unreadable, &options)
        .expect_err("lookup failures propagate");
    let diagnostic = format!("{error:?}");
    assert!(
        diagnostic.contains("test object access denied"),
        "the IO cause is retained"
    );
    assert!(
        diagnostic.contains(&blob_id.to_string()),
        "lookup context identifies the blob"
    );
    Ok(())
}
