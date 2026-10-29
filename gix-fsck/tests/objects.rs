use gix_fsck::objects::{MessageId, Options, Severity};
use gix_testtools::TestResult;

#[test]
fn raw_structure_diagnostics_match_git() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_raw_object_validation.sh")?;
    for case in std::fs::read_dir(fixture)? {
        let case = case?.path();
        if !case.join("object").is_file() {
            continue;
        }
        let object_id = gix_hash::ObjectId::from_hex(std::fs::read_to_string(case.join("id"))?.trim().as_bytes())?;
        let kind = gix_object::Kind::from_bytes(&std::fs::read(case.join("kind"))?)?;
        let bytes = std::fs::read(case.join("object"))?;
        let options = Options::strict();
        let actual = gix_fsck::objects::validate(
            object_id,
            gix_object::Data::new(&bytes, kind, object_id.kind()),
            &options,
        )?;
        let report = std::fs::read_to_string(case.join("report"))?;
        let mut expected = Vec::new();
        for line in report.lines() {
            let prefix = if line.starts_with("error in ") {
                Some(Severity::Error)
            } else if line.starts_with("warning in ") {
                Some(Severity::Warning)
            } else {
                None
            };
            if let Some(severity) = prefix
                && let Some((_, message)) = line.split_once(&format!("{object_id}: "))
                && let Some((name, _)) = message.split_once(':')
            {
                expected.push((name.parse::<MessageId>()?, severity));
            }
        }
        if expected.is_empty() && report.contains("object could not be parsed") {
            assert!(
                actual.iter().any(|diagnostic| diagnostic.severity == Severity::Error),
                "objects rejected before Git's fsck callback must also be rejected: {}: {report}",
                case.display()
            );
            continue;
        }
        let mut actual: Vec<_> = actual
            .into_iter()
            .map(|diagnostic| (diagnostic.id, diagnostic.severity))
            .collect();
        actual.sort();
        expected.sort();
        assert_eq!(actual, expected, "{}: {report}", case.display());
    }
    Ok(())
}

#[test]
fn severity_overrides_skip_lists_and_fatal_checks() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_raw_object_validation.sh")?;
    let bytes = std::fs::read(fixture.join("dot-name/object"))?;
    let object_id =
        gix_hash::ObjectId::from_hex(std::fs::read_to_string(fixture.join("dot-name/id"))?.trim().as_bytes())?;
    let data = gix_object::Data::new(&bytes, gix_object::Kind::Tree, object_id.kind());
    let mut options = Options::default();
    assert_eq!(
        gix_fsck::objects::validate(object_id, data, &options)?[0].severity,
        Severity::Warning
    );
    options.strict = true;
    assert_eq!(
        gix_fsck::objects::validate(object_id, data, &options)?[0].severity,
        Severity::Error
    );
    options.set_message("hasDot", "warn")?;
    assert_eq!(
        gix_fsck::objects::validate(object_id, data, &options)?[0].severity,
        Severity::Warning
    );
    options.set_message("hasDot", "ignore")?;
    assert!(gix_fsck::objects::validate(object_id, data, &options)?.is_empty());
    assert!(options.set_message("nulInHeader", "ignore").is_err());
    assert!(options.set_message("unterminatedHeader", "warn").is_err());
    assert!(options.set_message("unknownFutureMessage", "ignore").is_err());
    options.set_message("largePathname", "error:123")?;
    assert_eq!(options.max_tree_entry_length, 123);
    options.set_message("hasDot", "error")?;
    options.extend_skip_list(
        format!("# old object\n  {object_id}  # grandfathered\n").as_bytes(),
        object_id.kind(),
    )?;
    assert!(gix_fsck::objects::validate(object_id, data, &options)?.is_empty());
    assert!(options.extend_skip_list(b"invalid\n", object_id.kind()).is_err());
    Ok(())
}
