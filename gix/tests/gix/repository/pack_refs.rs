use std::{fs, sync::atomic::AtomicBool};

use gix::repository::pack_refs::Options;
use gix_testtools::TestResult;

#[test]
fn packing_shared_refs_matches_git_without_changing_logs() -> TestResult {
    for all in [false, true] {
        let mut expected = None;
        for use_git in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable_with_args(
                "make_pack_references.sh",
                std::iter::empty::<String>(),
                gix_testtools::Creation::Execute,
            )?;
            let path = fixture.path();
            if use_git {
                gix_testtools::git(path, if all { "pack-refs --all" } else { "pack-refs" })?;
                expected = Some(fs::read(path.join(".git/packed-refs"))?);
            } else {
                let mut repo = gix::open_opts(path.join("linked"), crate::restricted())?;
                repo.set_namespace("unrelated")?;
                let before = fs::read(repo.common_dir().join("packed-refs"))?;
                let reflog = fs::read(repo.common_dir().join("logs/refs/heads/main"))?;
                let options = Options {
                    all,
                    ..Default::default()
                };
                let dry_run = repo.pack_references(
                    Options {
                        dry_run: true,
                        ..options
                    },
                    &AtomicBool::default(),
                )?;
                assert!(dry_run.packed > 0);
                assert_eq!(fs::read(repo.common_dir().join("packed-refs"))?, before);
                let outcome = repo.pack_references(options, &AtomicBool::default())?;
                assert_eq!(outcome, dry_run);
                assert_eq!(
                    fs::read(repo.common_dir().join("packed-refs"))?,
                    expected.as_deref().expect("Git ran first")
                );
                assert_eq!(fs::read(repo.common_dir().join("logs/refs/heads/main"))?, reflog);
                assert!(repo.common_dir().join("refs/heads/alias").is_file());
                assert!(repo.common_dir().join("refs/bisect/good").is_file());
                assert!(repo.common_dir().join("refs/worktree/private").is_file());
                assert!(repo.git_dir().join("refs/worktree/private").is_file());
                assert_eq!(repo.common_dir().join("refs/heads/main").is_file(), !all);
                assert!(!repo.common_dir().join("refs/tags/annotated-two").exists());
                gix_testtools::git(path, "fsck --strict")?;
            }
        }
    }
    Ok(())
}

#[test]
fn packing_honors_locks_and_can_keep_loose_refs() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_pack_references.sh",
        std::iter::empty::<String>(),
        gix_testtools::Creation::Execute,
    )?;
    let repo = gix::open_opts(fixture.path(), crate::restricted())?;
    let before = fs::read(repo.common_dir().join("packed-refs"))?;
    for lock in ["packed-refs.lock", "refs/heads/main.lock"] {
        fs::write(repo.common_dir().join(lock), b"busy")?;
        repo.pack_references(
            Options {
                all: true,
                ..Default::default()
            },
            &AtomicBool::default(),
        )
        .expect_err("other writers exclude packing");
        assert_eq!(fs::read(repo.common_dir().join("packed-refs"))?, before);
        assert!(repo.common_dir().join("refs/heads/main").is_file());
        fs::remove_file(repo.common_dir().join(lock))?;
    }
    repo.pack_references(
        Options {
            all: true,
            prune: false,
            ..Default::default()
        },
        &AtomicBool::default(),
    )?;
    assert!(repo.common_dir().join("refs/heads/main").is_file());
    assert!(repo.common_dir().join("refs/tags/annotated-two").is_file());
    Ok(())
}
