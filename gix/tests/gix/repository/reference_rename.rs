use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use gix::bstr::ByteSlice;
use gix::refs::{Target, transaction::Change};
use gix_testtools::{Creation, TestResult, scripted_fixture_read_only_with_args, scripted_fixture_writable_with_args};

const SCRIPT: &str = "make_reference_rename.sh";

#[cfg(unix)]
#[test]
fn non_utf8_names_are_preserved_in_reflogs_and_configuration() -> TestResult {
    // The fixture copy helper rejects non-UTF-8 paths, so create these names directly in the temporary directory.
    let fixture = scripted_fixture_writable_with_args(SCRIPT, ["non-utf8", "setup"], Creation::Execute)?;
    let unsupported = fixture.path().join("unsupported");
    if unsupported.is_file() {
        eprintln!(
            "skipping non-UTF-8 rename: {}",
            std::fs::read_to_string(unsupported)?.trim()
        );
        return Ok(());
    }
    let baseline = scripted_fixture_read_only_with_args(SCRIPT, ["non-utf8", "git"])?;
    let input = std::fs::read(fixture.path().join("input"))?;
    let mut lines = input.lines();
    let source = lines.next().expect("fixture source name").as_bstr();
    let destination = lines.next().expect("fixture destination name").as_bstr();
    let mut repo = open(&fixture.path().join("main"))?;
    let target = repo.find_reference(source)?.target().into_owned();

    repo.rename_reference(source, destination, false)?;
    assert!(
        repo.try_find_reference(source)?.is_none(),
        "remove the original byte-oriented name"
    );
    assert_eq!(
        repo.find_reference(destination)?.target(),
        target.to_ref(),
        "preserve the source target under the exact destination bytes"
    );
    assert_eq!(
        std::fs::read_to_string(baseline.join("status"))?.trim(),
        "0",
        "Git accepts these byte-oriented branch names"
    );
    assert_eq!(
        observe(fixture.path())?,
        observe(&baseline)?,
        "preserve every reference-name, reflog, HEAD, and configuration byte like Git"
    );
    Ok(())
}

#[test]
fn local_branches_preserve_git_history_and_configuration() -> TestResult {
    for scenario in [
        "plain",
        "same",
        "descend",
        "ascend",
        "force-destination",
        "broken-destination",
        "cyclic-destination",
        "malformed-target-destination",
        "apply-source",
        "bare",
        "bare-destination",
    ] {
        assert_matches_git(scenario, true)?;
    }
    Ok(())
}

#[test]
fn every_matching_worktree_head_is_retargeted_with_git_reflogs() -> TestResult {
    for scenario in ["worktrees", "linked", "symbolic-head"] {
        assert_matches_git(scenario, true)?;
    }
    Ok(())
}

#[test]
fn performed_head_edits_identify_each_worktree_and_its_original_symbolic_target() -> TestResult {
    for (scenario, names) in [
        (
            "worktrees",
            &["main-worktree/HEAD", "worktrees/one/HEAD", "worktrees/two/HEAD"][..],
        ),
        (
            "linked",
            &["main-worktree/HEAD", "worktrees/one/HEAD", "worktrees/two/HEAD"][..],
        ),
        ("symbolic-head", &["main-worktree/HEAD"][..]),
        ("unborn-worktrees", &["main-worktree/HEAD", "worktrees/one/HEAD"][..]),
    ] {
        let fixture = writable(scenario)?;
        let input = std::fs::read_to_string(fixture.path().join("input"))?;
        let (source, destination, force, invocation) = inputs(&input);
        let mut repo = open(&fixture.path().join(invocation))?;
        let source_exists = repo.try_find_reference(source)?.is_some();
        let previous = names
            .iter()
            .map(|name| {
                repo.find_reference(*name)
                    .map(|reference| reference.target().into_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let edits = repo.rename_reference(source, destination, force)?;
        let reference_edits = if source_exists { 2 } else { 0 };
        assert_eq!(
            edits.len(),
            reference_edits + names.len(),
            "report exactly the renamed refs and matching worktree HEADs"
        );
        for ((name, previous), edit) in names.iter().zip(previous).zip(&edits[reference_edits..]) {
            assert_eq!(
                edit.name.as_bstr(),
                *name,
                "qualify each performed HEAD edit with its worktree"
            );
            assert!(!edit.deref, "HEAD retargeting edits the symbolic reference itself");
            let Change::Update { new, expected, .. } = &edit.change else {
                panic!("HEAD retargeting must be reported as an update: {edit:?}");
            };
            assert_eq!(
                new,
                &Target::Symbolic(destination.try_into()?),
                "report the new symbolic target"
            );
            assert_eq!(
                expected,
                &gix::refs::transaction::PreviousValue::MustExistAndMatch(previous),
                "report the original raw target, including an intermediate alias"
            );
        }
    }
    Ok(())
}

#[test]
fn unborn_branches_rename_heads_and_configuration_without_creating_refs() -> TestResult {
    for scenario in ["unborn", "unborn-worktrees", "unborn-existing", "unborn-prefix"] {
        assert_matches_git(scenario, true)?;
    }
    Ok(())
}

#[test]
fn existing_and_reserved_destinations_and_active_sources_are_protected() -> TestResult {
    for scenario in [
        "reject-destination",
        "symbolic-destination",
        "destination-used-main",
        "destination-used-linked",
        "rebase-source",
        "bisect-source",
        "rebase-destination",
        "bisect-destination",
        "pending-destination",
    ] {
        assert_matches_git(scenario, false)?;
    }
    Ok(())
}

#[test]
fn later_failures_report_the_partially_completed_rename() -> TestResult {
    assert_matches_git("head-lock", false)?;
    Ok(())
}

#[test]
fn configuration_lock_failure_precedes_reference_changes() -> TestResult {
    let fixture = writable("config-lock")?;
    let mut repo = open(&fixture.path().join("main"))?;
    let before = observe(fixture.path())?;
    let error = repo
        .rename_reference("refs/heads/topic", "refs/heads/renamed", false)
        .expect_err("configuration must be locked before renaming the branch");
    assert!(
        format!("{error:?}").contains("configuration"),
        "identify the blocked configuration update: {error:?}"
    );
    assert_eq!(
        observe(fixture.path())?,
        before,
        "a configuration lock failure preserves every ref, log, HEAD, and config"
    );
    Ok(())
}

#[test]
fn tags_and_remote_refs_keep_branch_configuration_and_worktree_heads() -> TestResult {
    for scenario in ["tag", "remote"] {
        let fixture = writable(scenario)?;
        let input = std::fs::read_to_string(fixture.path().join("input"))?;
        let (source, destination, force, invocation) = inputs(&input);
        let mut repo = open(&fixture.path().join(invocation))?;
        let before = observe(fixture.path())?;
        let target = repo.find_reference(source)?.target().into_owned();
        let old_log = history(&repo, source)?;

        let edits = repo.rename_reference(source, destination, force)?;
        assert!(repo.try_find_reference(source)?.is_none(), "remove the source ref");
        assert_eq!(
            repo.find_reference(destination)?.target(),
            target.to_ref(),
            "preserve the source target"
        );
        assert_performed(&edits, source, destination, Some(&target));
        let after = observe(fixture.path())?;
        for (path, content) in &before.files {
            if !path.starts_with("main/.git/logs/refs") {
                assert_eq!(after.files.get(path), Some(content), "{scenario}: preserve {path:?}");
            }
        }
        let new_log = history(&repo, destination)?;
        if scenario == "tag" {
            assert!(old_log.is_empty() && new_log.is_empty(), "do not create a tag reflog");
        } else {
            assert_eq!(new_log.len(), old_log.len() + 1, "append one rename entry");
            assert_eq!(
                &new_log[..old_log.len()],
                old_log,
                "preserve remote reflog history in order"
            );
            let last = new_log.last().expect("rename appends an entry");
            assert_eq!(
                last.previous_oid,
                target.id().to_owned(),
                "rename starts at the source target"
            );
            assert_eq!(last.new_oid, target.id().to_owned(), "rename keeps the source target");
        }
    }
    Ok(())
}

#[test]
fn pseudorefs_and_crossing_the_local_branch_boundary_are_rejected() -> TestResult {
    for (source, destination) in [
        ("HEAD", "refs/heads/renamed"),
        ("refs/heads/topic", "HEAD"),
        ("refs/heads/topic", "refs/tags/renamed"),
        ("refs/tags/topic", "refs/heads/renamed"),
        ("topic", "refs/heads/renamed"),
    ] {
        let fixture = writable("tag")?;
        let mut repo = open(&fixture.path().join("main"))?;
        let before = observe(fixture.path())?;
        assert!(
            repo.rename_reference(source, destination, false).is_err(),
            "reject unsupported rename {source} -> {destination}"
        );
        assert_eq!(observe(fixture.path())?, before, "validation precedes all changes");
    }
    Ok(())
}

fn assert_matches_git(scenario: &str, success: bool) -> TestResult {
    let fixture = writable(scenario)?;
    let baseline = scripted_fixture_read_only_with_args(SCRIPT, [scenario, "git"])?;
    let input = std::fs::read_to_string(fixture.path().join("input"))?;
    let (source, destination, force, invocation) = inputs(&input);
    let mut repo = open(&fixture.path().join(invocation))?;
    let source_target = repo
        .try_find_reference(source)?
        .map(|reference| reference.target().into_owned());
    let outcome = repo.rename_reference(source, destination, force);
    assert_eq!(
        std::fs::read_to_string(baseline.join("status"))?.trim() == "0",
        success,
        "{scenario}: the fixture records the expected Git outcome"
    );
    assert_eq!(outcome.is_ok(), success, "{scenario}: {outcome:?}");
    if let Ok(edits) = outcome {
        assert_performed(&edits, source, destination, source_target.as_ref());
        assert_reloaded_configuration(&repo, source, destination)?;
    }
    assert_eq!(
        observe(fixture.path())?,
        observe(&baseline)?,
        "{scenario}: refs, complete reflogs, every HEAD, and all config files match Git"
    );
    Ok(())
}

fn assert_reloaded_configuration(repo: &gix::Repository, source: &str, destination: &str) -> TestResult {
    let source = source.strip_prefix("refs/heads/").expect("local branch source");
    let destination = destination
        .strip_prefix("refs/heads/")
        .expect("local branch destination");
    let config = repo.config_snapshot();
    assert_eq!(
        config
            .string(&format!("branch.{destination}.marker"))
            .expect("renamed branch configuration"),
        "last",
        "the same repository sees the last renamed source section"
    );
    if source != destination {
        assert!(
            config.string(&format!("branch.{source}.marker")).is_none(),
            "the same repository discards renamed source values"
        );
    }
    for key in ["included", "private"] {
        assert_eq!(
            config
                .string(&format!("branch.{source}.{key}"))
                .expect("unmodified configuration source"),
            "unchanged",
            "included and per-worktree branch sections remain available under the source name"
        );
    }
    assert_eq!(
        repo.committer().expect("configured committer")?,
        gix::actor::SignatureRef {
            name: "committer".into(),
            email: "committer@example.com".into(),
            time: "946771200 +0000",
        },
        "refreshing renamed configuration preserves caller-supplied identity and time"
    );
    Ok(())
}

fn assert_performed(
    edits: &[gix::refs::transaction::RefEdit],
    source: &str,
    destination: &str,
    target: Option<&Target>,
) {
    if let Some(target) = target {
        assert!(
            edits
                .iter()
                .any(|edit| edit.name.as_bstr() == source && matches!(edit.change, Change::Delete { .. })),
            "report the source deletion"
        );
        assert!(
            edits.iter().any(|edit| edit.name.as_bstr() == destination
                && matches!(&edit.change, Change::Update { new, .. } if new == target)),
            "report the destination's preserved source target"
        );
    } else {
        assert!(
            !edits
                .iter()
                .any(|edit| edit.name.as_bstr() == source && matches!(edit.change, Change::Delete { .. })),
            "an unborn branch has no source reference to delete"
        );
    }
}

fn writable(scenario: &str) -> TestResult<gix_testtools::tempfile::TempDir> {
    Ok(scripted_fixture_writable_with_args(
        SCRIPT,
        [scenario, "setup"],
        Creation::CopyFromReadOnly,
    )?)
}

fn inputs(input: &str) -> (&str, &str, bool, &str) {
    let mut lines = input.lines();
    (
        lines.next().expect("fixture source name"),
        lines.next().expect("fixture destination name"),
        lines.next().expect("fixture force flag") == "true",
        lines.next().expect("fixture invocation worktree"),
    )
}

fn open(path: &Path) -> TestResult<gix::Repository> {
    let mut options = crate::restricted();
    options.permissions.config.includes = true;
    Ok(gix::open_opts(
        path,
        options.config_overrides([
            "committer.name=committer",
            "committer.email=committer@example.com",
            "gitoxide.commit.committerDate=2000-01-02 00:00:00 +0000",
            "core.filesRefLockTimeout=0",
            "core.packedRefsTimeout=0",
            "core.configLockTimeout=0",
        ]),
    )?)
}

fn history(repo: &gix::Repository, name: &str) -> TestResult<Vec<gix::refs::log::Line>> {
    let mut entries = Vec::new();
    if let Some(lines) = repo.find_reference(name)?.log_iter().all()? {
        for line in lines {
            entries.push(line?.into());
        }
    }
    Ok(entries)
}

#[derive(Debug, PartialEq, Eq)]
struct State {
    refs: BTreeMap<gix::bstr::BString, Target>,
    files: BTreeMap<PathBuf, Vec<u8>>,
}

fn observe(root: &Path) -> TestResult<State> {
    let repo = open(&root.join("main"))?;
    let refs = repo
        .references()?
        .all()?
        .map(|reference| {
            reference.map(|reference| (reference.name().as_bstr().to_owned(), reference.target().into_owned()))
        })
        .collect::<Result<_, _>>()?;
    let mut files = BTreeMap::new();
    let common = Path::new("main/.git");
    for name in ["HEAD", "config", "included.cfg", "config.worktree"] {
        let relative = common.join(name);
        files.insert(relative.clone(), std::fs::read(root.join(relative))?);
    }
    let broken = common.join("BROKEN_REF");
    if root.join(&broken).is_file() {
        files.insert(broken.clone(), std::fs::read(root.join(broken))?);
    }
    collect_files(root, &common.join("logs"), &mut files)?;
    let worktrees = root.join(common).join("worktrees");
    if worktrees.is_dir() {
        for entry in std::fs::read_dir(worktrees)? {
            let relative = common.join("worktrees").join(entry?.file_name());
            for name in ["HEAD", "config.worktree"] {
                let path = relative.join(name);
                files.insert(path.clone(), std::fs::read(root.join(path))?);
            }
            collect_files(root, &relative.join("logs"), &mut files)?;
        }
    }
    Ok(State { refs, files })
}

fn collect_files(root: &Path, relative: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> TestResult {
    if !root.join(relative).is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(root.join(relative))? {
        let entry = entry?;
        let path = relative.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            collect_files(root, &path, files)?;
        } else {
            files.insert(path.clone(), std::fs::read(root.join(path))?);
        }
    }
    Ok(())
}
