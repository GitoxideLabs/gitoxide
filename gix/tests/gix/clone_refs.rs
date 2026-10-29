#![cfg(all(feature = "worktree-mutation", feature = "blocking-network-client"))]

use std::sync::atomic::AtomicBool;

use gix_testtools::TestResult;

#[test]
fn reference_and_head_state_agrees_with_git_clone() -> TestResult {
    use baseline::{Case, state};

    // Isolate `git-upload-pack`, whose environment is isolated here.
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_clone_refs.sh")?;
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let mut cases = std::fs::read_dir(fixture.join("cases"))?.collect::<Result<Vec<_>, _>>()?;
    cases.sort_by_key(std::fs::DirEntry::file_name);
    let mut differences = Vec::new();
    for case in cases {
        let case = Case::from_dir_entry(case)?;
        let name = &case.name;
        let bare = name.ends_with("-bare");
        let mut prepare = gix::clone::PrepareFetch::new(
            fixture.join(&case.source),
            tmp.path().join(name),
            if bare {
                gix::create::Kind::Bare
            } else {
                gix::create::Kind::WithWorktree
            },
            Default::default(),
            crate::util::restricted().config_overrides([
                format!("init.defaultBranch={}", case.default_branch),
                "protocol.version=2".into(),
            ]),
        )?
        .with_remote_name(case.remote.as_str())?;
        if let Some(branch) = case.branch.as_deref() {
            prepare = prepare.with_ref_name(Some(branch))?;
        }
        if let Some(depth) = case.depth {
            prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(depth));
        }
        if let Some(revision) = case.revision.as_deref() {
            prepare = prepare.with_revision(Some(revision))?;
        }
        let extra = case.extra_refspecs;
        if !extra.is_empty() {
            // Git supplements configured specs with bare clone defaults without persisting the defaults.
            if bare {
                prepare = prepare.with_fetch_options(gix::remote::ref_map::Options {
                    extra_refspecs: vec![
                        gix::refspec::parse(
                            "+refs/heads/*:refs/heads/*".into(),
                            gix::refspec::parse::Operation::Fetch,
                        )?
                        .to_owned(),
                    ],
                    ..Default::default()
                });
            }
            prepare = prepare.configure_remote(move |mut remote| {
                let mut specs = extra.clone();
                let defaults = remote
                    .refspecs(gix::remote::Direction::Fetch)
                    .iter()
                    .map(|spec| spec.to_ref().to_bstring().to_string());
                for spec in defaults {
                    if !specs.contains(&spec) {
                        specs.push(spec);
                    }
                }
                remote.replace_refspecs(specs.iter().map(String::as_str), gix::remote::Direction::Fetch)?;
                Ok(remote)
            });
        }
        let actual = prepare.fetch_then_checkout(gix::progress::Discard, &AtomicBool::default());
        let success = case.success;
        match actual {
            Ok((mut checkout, _)) if success => {
                let repo = if !bare && !name.ends_with("-no-checkout") {
                    checkout
                        .main_worktree(gix::progress::Discard, &AtomicBool::default())?
                        .0
                } else {
                    checkout.persist()
                };
                let expected_repo = gix::open_opts(case.path.join("git"), crate::util::restricted())?;
                let actual = state(&repo)?;
                let expected = state(&expected_repo)?;
                if actual != expected {
                    differences.push(format!("{name}:\nactual: {actual:#?}\nGit: {expected:#?}"));
                }
                assert_eq!(
                    state(&gix::open_opts(repo.git_dir(), crate::util::restricted())?)?,
                    actual,
                    "{name}: reopening preserves references and configuration"
                );
                if let (Some(workdir), Some(expected_workdir)) = (repo.workdir(), expected_repo.workdir()) {
                    assert_eq!(
                        std::fs::read(workdir.join("file")).ok(),
                        std::fs::read(expected_workdir.join("file")).ok(),
                        "{name}: checkout contents agree with Git"
                    );
                }
                let actual_shallow = repo.shallow_commits()?.map(|ids| {
                    std::iter::once(ids.head)
                        .chain(ids.tail.iter().copied())
                        .collect::<Vec<_>>()
                });
                let expected_shallow = expected_repo.shallow_commits()?.map(|ids| {
                    std::iter::once(ids.head)
                        .chain(ids.tail.iter().copied())
                        .collect::<Vec<_>>()
                });
                assert_eq!(
                    actual_shallow, expected_shallow,
                    "{name}: shallow boundaries agree with Git"
                );
            }
            Err(error) if !success => {
                let diagnostics = std::fs::read_to_string(case.path.join("stderr"))?;
                assert!(!diagnostics.is_empty(), "{name}: Git records a failure diagnostic");
                assert!(
                    !error.to_string().is_empty(),
                    "{name}: gix reports a failure diagnostic"
                );
            }
            result => differences.push(format!("{name}: Git success={success}, gix={result:?}")),
        }
    }
    assert!(
        differences.is_empty(),
        "clone behavior differs from Git:\n{}",
        differences.join("\n")
    );
    Ok(())
}

#[test]
fn bare_clone_persists_an_explicit_default_mapping() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_clone_refs.sh")?;
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    // Git's clone adds this mapping twice and rejects it; gix accepts replacement-only refspecs.
    let (repo, _) = gix::clone::PrepareFetch::new(
        fixture.join("symbolic"),
        tmp.path(),
        gix::create::Kind::Bare,
        Default::default(),
        crate::util::restricted(),
    )?
    .configure_remote(|mut remote| {
        remote.replace_refspecs(["+refs/heads/*:refs/heads/*"], gix::remote::Direction::Fetch)?;
        Ok(remote)
    })
    .fetch_only(gix::progress::Discard, &AtomicBool::default())?;
    let config = repo.config_snapshot();
    assert_eq!(
        config.string("remote.origin.fetch"),
        Some(gix::bstr::BString::from("+refs/heads/*:refs/heads/*")),
        "a user-supplied mapping is persisted even when identical to the initial bare mapping"
    );
    Ok(())
}

mod baseline {
    use std::{num::NonZeroU32, path::PathBuf};

    use gix_testtools::TestResult;

    pub(super) struct Case {
        pub name: String,
        pub path: PathBuf,
        pub source: String,
        pub branch: Option<String>,
        pub depth: Option<NonZeroU32>,
        pub default_branch: String,
        pub remote: String,
        pub revision: Option<String>,
        pub extra_refspecs: Vec<String>,
        pub success: bool,
    }

    impl Case {
        pub fn from_dir_entry(entry: std::fs::DirEntry) -> TestResult<Self> {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            let input = std::fs::read_to_string(path.join("inputs"))?;
            let [source, branch, depth, default_branch, remote, revision]: [&str; 6] = input
                .lines()
                .collect::<Vec<_>>()
                .try_into()
                .map_err(|_| gix::error::message!("{name}: expected six fixture input fields"))?;
            let extra_refspecs = std::fs::read_to_string(path.join("extra-refspecs"))?
                .lines()
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect();
            let success = std::fs::read_to_string(path.join("status"))?.trim() == "0";
            Ok(Self {
                name,
                source: source.into(),
                branch: (branch != "-").then(|| branch.to_owned()),
                depth: if depth == "-" { None } else { Some(depth.parse()?) },
                default_branch: default_branch.into(),
                remote: remote.into(),
                revision: (revision != "-").then(|| revision.to_owned()),
                extra_refspecs,
                success,
                path,
            })
        }
    }

    #[derive(Debug, PartialEq, Eq)]
    pub(super) struct State {
        refs: Vec<(String, String)>,
        config: Vec<(String, Vec<String>)>,
        pub logs: Vec<(String, Vec<(gix::ObjectId, gix::ObjectId)>)>,
    }

    fn log(repo: &gix::Repository, name: &str) -> TestResult<Vec<(gix::ObjectId, gix::ObjectId)>> {
        let mut values = Vec::new();
        if let Some(entries) = repo.find_reference(name)?.log_iter().all()? {
            for entry in entries {
                let entry = entry?;
                values.push((entry.previous_oid(), entry.new_oid()));
            }
        }
        Ok(values)
    }

    pub(super) fn state(repo: &gix::Repository) -> TestResult<State> {
        let mut refs = vec![(
            "HEAD".into(),
            std::fs::read_to_string(repo.git_dir().join("HEAD"))?.trim().into(),
        )];
        let mut logs = vec![("HEAD".into(), log(repo, "HEAD")?)];
        for reference in repo.references()?.all()? {
            let reference = reference?;
            refs.push((
                reference.name().to_string(),
                reference.target().into_owned().to_string(),
            ));
            logs.push((reference.name().to_string(), log(repo, &reference.name().to_string())?));
        }
        refs.sort();
        logs.sort();
        let config = repo.config_snapshot();
        let mut values = Vec::new();
        for remote in ["origin", "team/origin"] {
            for key in ["fetch", "tagOpt", "mirror"] {
                let key = format!("remote.{remote}.{key}");
                values.push((
                    key.clone(),
                    config
                        .plumbing()
                        .raw_values(key.as_str())
                        .unwrap_or_default()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                ));
            }
        }
        for branch in [
            "main",
            "other",
            "alpha",
            "preferred",
            "master",
            "unborn",
            "absent",
            "HEAD",
        ] {
            for key in ["remote", "merge"] {
                let key = format!("branch.{branch}.{key}");
                values.push((
                    key.clone(),
                    config
                        .plumbing()
                        .raw_values(key.as_str())
                        .unwrap_or_default()
                        .iter()
                        .map(ToString::to_string)
                        .collect(),
                ));
            }
        }
        Ok(State {
            refs,
            config: values,
            logs,
        })
    }
}
