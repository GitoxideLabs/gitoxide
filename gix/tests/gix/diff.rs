use crate::Result;
use gix_diff::blob::{Algorithm, Driver};

use crate::util::named_repo;

#[test]
fn resource_cache() -> Result {
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
                is_binary: None
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
