use gix_diff::blob::{Algorithm, Driver};
use gix_testtools::TestResult;

use crate::util::named_repo;

#[test]
fn resource_cache() -> TestResult {
    let repo = named_repo("make_diff_repo.sh")?;
    let index = repo.index()?;
    let cache = gix::diff::resource_cache(
        &repo,
        gix::diff::blob::pipeline::Mode::ToWorktreeAndBinaryToText,
        repo.attributes_only(&index, gix_worktree::stack::state::attributes::Source::IdMapping)?
            .detach(),
        Default::default(),
    )?;
    assert_eq!(
        cache.filter.drivers(),
        &[
            Driver {
                name: "all-but-binary".into(),
                command: Some("command".into()),
                algorithm: Some(Algorithm::Histogram),
                binary_to_text_command: Some("textconv".into()),
                is_binary: None,
                hunk_header: None,
            },
            Driver {
                name: "binary-false".into(),
                is_binary: Some(false),
                ..Default::default()
            },
            Driver {
                name: "binary-true".into(),
                is_binary: Some(true),
                ..Default::default()
            }
        ]
    );
    assert_eq!(cache.options.algorithm, Some(Algorithm::Histogram));
    assert!(
        !cache.options.skip_internal_diff_if_external_is_configured,
        "pre-set to something that makes sense for most"
    );
    assert_eq!(
        cache.filter.options.large_file_threshold_bytes,
        512 * 1024 * 1024,
        "the default value unless it's configured"
    );
    Ok(())
}

#[test]
fn hunk_header_driver_configuration_preserves_order_and_defers_regex_errors() -> TestResult {
    use gix_diff::blob::hunk_header::Syntax;
    let repo = named_repo("make_hunk_header_drivers.sh")?;
    for (name, syntax) in [("basic-last", Syntax::Basic), ("extended-last", Syntax::Extended)] {
        let driver = repo.diff_driver(name)?.ok_or("configured driver")?;
        assert_eq!(
            driver
                .hunk_header
                .as_ref()
                .ok_or("configured hunk pattern")?
                .options
                .syntax,
            syntax,
            "the last of funcname and xfuncname wins in configuration order"
        );
        assert_eq!(
            driver.hunk_header()?.find(b"prefix declaration")?,
            Some(b"declaration".as_slice())
        );
    }
    let driver = repo
        .diff_driver("rust")?
        .ok_or("a configured built-in name is retained")?;
    assert!(
        driver.hunk_header.is_none(),
        "an unrelated override does not duplicate a built-in definition"
    );
    assert_eq!(
        driver.hunk_header()?.find(b"pub fn run() {")?,
        Some(b"pub fn run() {".as_slice()),
        "built-in language matching survives unrelated driver configuration"
    );
    let driver = repo.diff_driver("python")?.ok_or("unconfigured built-in driver")?;
    assert!(driver.hunk_header()?.find(b"def run():")?.is_some());
    let invalid = repo.diff_driver("invalid")?.ok_or("invalid pattern can be stored")?;
    assert!(
        invalid.hunk_header().is_err(),
        "only selecting the invalid pattern produces a regex error"
    );
    assert!(
        repo.diff_driver("unconfigured")?.is_none(),
        "unknown names have no configured driver"
    );
    Ok(())
}
