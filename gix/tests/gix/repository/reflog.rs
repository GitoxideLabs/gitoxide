use std::{collections::BTreeMap, fs, path::Path, sync::atomic::AtomicBool};

use gix::repository::reflog::Options;
use gix_testtools::TestResult;

fn logs(root: &Path) -> TestResult<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<std::path::PathBuf, Vec<u8>>) -> TestResult {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                visit(root, &entry.path(), out)?;
            } else {
                out.insert(entry.path().strip_prefix(root)?.to_owned(), fs::read(entry.path())?);
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    visit(root, &root.join("logs"), &mut out)?;
    visit(root, &root.join("worktrees/linked/logs"), &mut out)?;
    Ok(out)
}

#[test]
fn configured_expiry_matches_git_in_all_worktrees() -> TestResult {
    let mut expected = None;
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable_with_args(
            "make_reflog_expiry.sh",
            std::iter::empty::<String>(),
            gix_testtools::Creation::Execute,
        )?;
        let path = fixture.path();
        let git_dir = path.join(".git");
        if use_git {
            gix_testtools::git(path, "reflog expire --all")?;
            expected = Some(logs(&git_dir)?);
        } else {
            let repo = gix::open_opts(path.join("linked"), crate::restricted())?;
            let original = logs(&git_dir)?;
            let dry_run = repo.expire_reflogs(
                Options {
                    dry_run: true,
                    ..Default::default()
                },
                &AtomicBool::default(),
            )?;
            assert!(dry_run.removed > 0);
            assert_eq!(logs(&git_dir)?, original, "dry run preserves every worktree log");
            let actual = repo.expire_reflogs(Default::default(), &AtomicBool::default())?;
            assert_eq!(actual, dry_run);
            for (name, actual) in logs(&git_dir)? {
                let expected = &expected.as_ref().expect("Git ran first")[&name];
                // Git normalizes missing message tabs when rewriting. Compare parsed entries;
                // Gitoxide deliberately preserves the original retained bytes.
                let parse = |bytes: &[u8]| -> TestResult<Vec<gix::refs::log::Line>> {
                    Ok(gix::refs::file::log::iter::forward(bytes)
                        .map(|line| line.map(|line| line.to_owned()))
                        .collect::<gix::Result<_>>()?)
                };
                assert_eq!(parse(&actual)?, parse(expected)?, "reflog {}", name.display());
            }
            gix_testtools::git(path, "fsck --strict")?;
        }
    }
    Ok(())
}

#[test]
fn explicit_expiry_overrides_patterns_and_stash_and_errors_are_safe() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_reflog_expiry.sh",
        std::iter::empty::<String>(),
        gix_testtools::Creation::Execute,
    )?;
    let path = fixture.path();
    let repo = gix::open_opts(path, crate::restricted())?;
    let original = logs(repo.git_dir())?;
    repo.expire_reflogs(Default::default(), &AtomicBool::new(true))
        .expect_err("interruption stops expiry");
    assert_eq!(logs(repo.git_dir())?, original);
    fs::write(repo.git_dir().join("HEAD.lock"), b"busy")?;
    repo.expire_reflogs(Default::default(), &AtomicBool::default())
        .expect_err("reference writers exclude expiry");
    assert_eq!(logs(repo.git_dir())?, original);
    fs::remove_file(repo.git_dir().join("HEAD.lock"))?;
    repo.expire_reflogs(
        Options {
            expire: Some(u64::MAX),
            expire_unreachable: Some(u64::MAX),
            ..Default::default()
        },
        &AtomicBool::default(),
    )?;
    assert!(
        logs(repo.git_dir())?.values().all(Vec::is_empty),
        "explicit expiry also empties stash logs"
    );
    Ok(())
}

#[test]
fn default_ages_use_the_provided_clock() -> TestResult {
    let now: gix::date::Zoned = "2002-03-02T00:00:00+00:00[UTC]".parse()?;
    let mut expected = None;
    for use_git in [true, false] {
        let fixture = gix_testtools::scripted_fixture_writable_with_args(
            "make_reflog_expiry.sh",
            ["defaults"],
            gix_testtools::Creation::Execute,
        )?;
        let path = fixture.path();
        if use_git {
            gix_testtools::git(
                path,
                &format!(
                    "reflog expire --all --expire=@{} --expire-unreachable=@{}",
                    now.timestamp().as_second() - 90 * 24 * 60 * 60,
                    now.timestamp().as_second() - 30 * 24 * 60 * 60,
                ),
            )?;
            expected = Some(logs(&path.join(".git"))?);
        } else {
            let repo = gix::open_opts(path, crate::restricted())?;
            repo.expire_reflogs(
                Options {
                    now: Some(now.clone()),
                    ..Default::default()
                },
                &AtomicBool::default(),
            )?;
            for (name, actual) in logs(repo.git_dir())? {
                let expected = &expected.as_ref().expect("Git ran first")[&name];
                let parse = |bytes: &[u8]| -> TestResult<Vec<gix::refs::log::Line>> {
                    Ok(gix::refs::file::log::iter::forward(bytes)
                        .map(|line| line.map(|line| line.to_owned()))
                        .collect::<gix::Result<_>>()?)
                };
                assert_eq!(parse(&actual)?, parse(expected)?, "reflog {}", name.display());
            }
        }
    }
    Ok(())
}
