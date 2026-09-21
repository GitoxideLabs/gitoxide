use std::{
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, UNIX_EPOCH},
};

use gix::{
    bstr::BString,
    notify::{Changes, PathChange, Scope},
    status::{Item, WorktreeStatusHandle, monitor::Update},
};
use gix_error::TestResult;

fn fixture() -> TestResult<gix_testtools::tempfile::TempDir> {
    Ok(gix_testtools::scripted_fixture_writable(
        "make_status_monitor_index.sh",
    )?)
}

fn open(root: &Path) -> gix::Result<gix::Repository> {
    gix::open_opts(root, gix::open::Options::isolated())
}

fn refresh(monitor: &mut WorktreeStatusHandle, root: &Path) -> TestResult<Update> {
    Ok(monitor.refresh(&open(root)?, &AtomicBool::new(false))?)
}

fn git(root: &Path, args: &[&str]) -> TestResult {
    let output = gix_testtools::git_command(root).args(args).output()?;
    assert!(
        output.status.success(),
        "Git prepares or changes the disposable rules fixture with {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn scope(path: &str) -> Option<Scope> {
    Some(Scope::Paths(vec![PathChange {
        path: path.into(),
        kind: gix::notify::plumbing::EventKind::Any,
        path_kind: gix::notify::plumbing::PathKind::Any,
    }]))
}

fn invalidate_rule(monitor: &mut WorktreeStatusHandle, path: &str, attributes: bool) {
    monitor.invalidate_changes(&Changes {
        ignores: !attributes,
        attributes,
        worktree: scope(path),
        ..Default::default()
    });
}

fn invalidate_index(monitor: &mut WorktreeStatusHandle) {
    monitor.invalidate_changes(&Changes {
        index: true,
        ..Default::default()
    });
}

fn stable_file(path: impl AsRef<Path>, data: &str) -> TestResult {
    std::fs::write(&path, data)?;
    // An index rewrite must not move unrelated fixture entries out of the racy window.
    std::fs::File::options()
        .write(true)
        .open(path)?
        .set_modified(UNIX_EPOCH + Duration::from_secs(1_000_000_000))?;
    Ok(())
}

fn project(root: &Path, rule_name: &str, rules: &str) -> TestResult {
    std::fs::create_dir(root.join("project"))?;
    stable_file(root.join("project/tracked"), "line\n")?;
    stable_file(root.join("project").join(rule_name), rules)?;
    git(root, &["add", "project"])
}

fn change_indexed_rule(root: &Path, path: &str, rules: &str) -> TestResult {
    let repo = open(root)?;
    let blob_id = repo.write_blob(rules)?;
    let mut index = repo.open_index()?;
    index
        .entry_mut_by_path_and_stage(path.into(), gix::index::entry::Stage::Unconflicted)
        .expect("the rules fixture contains this indexed file")
        .id = blob_id.detach();
    index.write(Default::default(), repo.refs.shared_repository_permissions)?;
    Ok(())
}

fn assert_matches_fresh_status(monitor: &WorktreeStatusHandle, root: &Path) -> TestResult {
    let mut fresh = open(root)?
        .status(gix::progress::Discard)?
        .index_worktree_options_mut(|options| {
            options.sorting = Some(gix::status::plumbing::index_as_worktree_with_renames::Sorting::ByPathCaseSensitive);
        })
        .into_vec(None::<BString>, &AtomicBool::new(false))?;
    let rank = |item: &Item| match item {
        Item::TreeIndex(_) => 0,
        Item::IndexWorktree(_) => 1,
    };
    fresh.sort_by(|a, b| rank(a).cmp(&rank(b)).then_with(|| a.location().cmp(b.location())));
    assert_eq!(
        monitor.snapshot(),
        Some(fresh.as_slice()),
        "scoped rule refreshes agree with fresh status for every retained row"
    );
    Ok(())
}

#[test]
fn nested_worktree_ignore_changes_refresh_the_parent() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitignore", "out/\n")?;
    std::fs::create_dir(root.join("project/out"))?;
    std::fs::write(root.join("project/out/file"), "untracked\n")?;
    std::fs::write(root.join("z"), "unrelated change\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;

    std::fs::write(root.join("project/.gitignore"), "")?;
    invalidate_rule(&mut monitor, "project/.gitignore", false);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("project"),
        "nested ignore rules affect their parent subtree without rechecking other roots"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn nested_indexed_ignore_fallback_changes_refresh_the_parent() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitignore", "out/\n")?;
    git(root, &["update-index", "--skip-worktree", "project/.gitignore"])?;
    std::fs::remove_file(root.join("project/.gitignore"))?;
    std::fs::create_dir(root.join("project/out"))?;
    std::fs::write(root.join("project/out/file"), "untracked\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;

    change_indexed_rule(root, "project/.gitignore", "")?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("project"),
        "a changed skip-worktree ignore blob is the active rule source for its subtree"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn staging_an_existing_worktree_ignore_file_keeps_the_rule_policy() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitignore", "out/\n")?;
    std::fs::create_dir(root.join("project/out"))?;
    std::fs::write(root.join("project/out/file"), "untracked\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("project/.gitignore"), "")?;
    invalidate_rule(&mut monitor, "project/.gitignore", false);
    refresh(&mut monitor, root)?;

    git(root, &["add", "project/.gitignore"])?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(update.staged, "staging changes the rule file's index contents");
    assert_eq!(
        update.unstaged,
        scope(if cfg!(windows) { "project" } else { "project/.gitignore" }),
        "staging retains worktree ignore policy, with conservative Windows read-permission coverage"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn nested_worktree_attribute_changes_refresh_the_parent() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitattributes", "* -text\n")?;
    std::fs::write(root.join("project/tracked"), "line\r\n")?;
    std::fs::write(root.join("z"), "unrelated change\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;

    std::fs::write(root.join("project/.gitattributes"), "* text\n")?;
    invalidate_rule(&mut monitor, "project/.gitattributes", true);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("project"),
        "nested attributes can change normalized status throughout their parent subtree"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn nested_indexed_attribute_fallback_changes_refresh_the_parent() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitattributes", "* -text\n")?;
    std::fs::remove_file(root.join("project/.gitattributes"))?;
    std::fs::write(root.join("project/tracked"), "line\r\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;

    change_indexed_rule(root, "project/.gitattributes", "* text\n")?;
    invalidate_index(&mut monitor);
    assert_eq!(
        refresh(&mut monitor, root)?.unstaged,
        scope("project"),
        "attributes fall back to their index blob when the worktree rule is absent"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn staging_an_existing_worktree_attribute_file_keeps_the_rule_policy() -> TestResult {
    let fixture = fixture()?;
    let root = fixture.path();
    project(root, ".gitattributes", "* -text\n")?;
    std::fs::write(root.join("project/tracked"), "line\r\n")?;
    let (_project, mut monitor) = super::monitor(root)?;
    refresh(&mut monitor, root)?;
    std::fs::write(root.join("project/.gitattributes"), "* text\n")?;
    invalidate_rule(&mut monitor, "project/.gitattributes", true);
    refresh(&mut monitor, root)?;

    git(root, &["add", "project/.gitattributes"])?;
    invalidate_index(&mut monitor);
    let update = refresh(&mut monitor, root)?;
    assert!(update.staged, "staging changes the attribute file's index contents");
    assert_eq!(
        update.unstaged,
        scope(if cfg!(windows) {
            "project"
        } else {
            "project/.gitattributes"
        }),
        "staging retains worktree attributes, with conservative Windows read-permission coverage"
    );
    assert_matches_fresh_status(&monitor, root)
}

#[test]
fn gitmodules_changes_refresh_submodule_rows_with_all_worktree_coverage() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_status_monitor_submodules.sh")?;
    let root = fixture.path().join("root");
    let repo = open(&root)?;
    let project = repo.monitor(gix::notify::Options {
        worktree_coverage: Some(gix::notify::WorktreeCoverage::All),
        ..Default::default()
    })?;
    let mut monitor = project.worktree_status(&repo, Default::default())?;
    refresh(&mut monitor, &root)?;
    assert!(
        monitor
            .snapshot()
            .expect("the initial status baseline exists")
            .iter()
            .any(|item| matches!(item, Item::IndexWorktree(_) if item.location() == "outer")),
        "the fixture's dirty nested checkout is initially visible at its root gitlink"
    );

    git(
        &root,
        &["config", "--file", ".gitmodules", "submodule.outer.ignore", "all"],
    )?;
    // All coverage reports the rule path, leaving the status monitor to expand its effects.
    invalidate_rule(&mut monitor, ".gitmodules", true);
    assert_eq!(
        refresh(&mut monitor, &root)?.unstaged,
        Some(Scope::All),
        "submodule ignore changes refresh gitlink rows outside the rule path"
    );
    assert!(
        monitor
            .snapshot()
            .expect("the refreshed status baseline exists")
            .iter()
            .all(|item| !matches!(item, Item::IndexWorktree(_) if item.location() == "outer")),
        "ignore=all removes the old dirty submodule row from the owned snapshot"
    );
    assert_matches_fresh_status(&monitor, &root)
}

#[test]
fn root_and_global_rule_changes_keep_full_fallbacks() -> TestResult {
    for (path, attributes, global) in [
        (".gitignore", false, false),
        (".gitattributes", true, false),
        (".git/info/exclude", false, true),
        (".git/info/attributes", true, true),
    ] {
        let fixture = fixture()?;
        let root = fixture.path();
        let before = if attributes { "* -text\n" } else { "untracked\n" };
        stable_file(root.join(path), before)?;
        std::fs::write(root.join("tracked"), "initial\r\n")?;
        std::fs::write(root.join("untracked"), "new\n")?;
        let (_project, mut monitor) = super::monitor(root)?;
        refresh(&mut monitor, root)?;

        std::fs::write(root.join(path), if attributes { "* text\n" } else { "" })?;
        if global {
            monitor.invalidate_changes(&Changes {
                ignores: !attributes,
                attributes,
                ..Default::default()
            });
        } else {
            invalidate_rule(&mut monitor, path, attributes);
        }
        assert_eq!(
            refresh(&mut monitor, root)?.unstaged,
            Some(Scope::All),
            "{path} can affect status throughout the checkout"
        );
        assert_matches_fresh_status(&monitor, root)?;
    }
    Ok(())
}
