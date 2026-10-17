use gix_testtools::TestResult;

use crate::util::named_repo;

#[test]
fn global_paths_include_missing_files_without_loading_them() -> TestResult {
    let mut repo = named_repo("make_basic_repo.sh")?;
    assert!(
        repo.attribute_global_paths()?.is_empty(),
        "isolated repositories select no external attribute files by default"
    );
    repo.config_snapshot_mut().set_value(
        &gix::config::tree::Core::ATTRIBUTES_FILE,
        "definitely-missing-attributes",
    )?;
    let expected = repo.current_dir().join("definitely-missing-attributes");
    assert!(!expected.exists(), "the selected file is absent");
    assert_eq!(
        repo.attribute_global_paths()?,
        vec![expected],
        "relative configured paths resolve against the captured current directory, even when absent"
    );
    let index = repo.index_or_empty()?;
    repo.attributes_only(
        &index,
        gix_worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
    )?;
    Ok(())
}

#[test]
fn global_paths_preserve_strict_and_lenient_configuration_handling() -> TestResult {
    let mut repo = named_repo("make_basic_repo.sh")?;
    repo.config_snapshot_mut()
        .set_value(&gix::config::tree::Core::ATTRIBUTES_FILE, "")?;
    assert!(
        repo.attribute_global_paths()?.is_empty(),
        "lenient configuration ignores an empty attribute path"
    );
    let mut repo = gix::open_opts(repo.git_dir(), repo.open_options().clone().strict_config(true))?;
    repo.config_snapshot_mut()
        .set_value(&gix::config::tree::Core::ATTRIBUTES_FILE, "")?;
    assert!(
        repo.attribute_global_paths().is_err(),
        "strict configuration retains invalid attribute path errors"
    );
    Ok(())
}
