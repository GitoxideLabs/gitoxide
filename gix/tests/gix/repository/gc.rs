use std::{
    fs,
    path::Path,
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime},
};

use gix::{ObjectId, repository::gc::Options};
use gix_testtools::TestResult;

fn time(seconds: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)
}

fn ids(directory: &Path, name: &str) -> TestResult<Vec<ObjectId>> {
    Ok(fs::read_to_string(directory.join(name))?
        .lines()
        .map(|line| ObjectId::from_hex(line.as_bytes()))
        .collect::<gix::Result<_>>()?)
}

fn fixture() -> gix_testtools::Result<gix_testtools::tempfile::TempDir> {
    gix_testtools::scripted_fixture_writable_with_args(
        "make_object_gc.sh",
        std::iter::empty::<String>(),
        gix_testtools::Creation::Execute,
    )
}

fn assert_retention(path: &Path, recent: bool, expired: bool) -> TestResult {
    let repo = gix::open_opts(path, crate::restricted())?;
    for object_id in ids(path, "live-ids")? {
        assert!(
            repo.try_find_object(object_id)?.is_some(),
            "reachable objects are always retained"
        );
    }
    for object_id in ids(path, "recent-ids")? {
        assert_eq!(
            repo.try_find_object(object_id)?.is_some(),
            recent,
            "recent commit and older dependencies share retention"
        );
    }
    for object_id in ids(path, "expired-id")? {
        assert_eq!(
            repo.try_find_object(object_id)?.is_some(),
            expired,
            "unrelated old objects can expire"
        );
    }
    Ok(())
}

#[test]
fn retention_matches_git_and_cruft_repacking_preserves_ages() -> TestResult {
    for use_git in [true, false] {
        for storage in ["loose", "cruft"] {
            let fixture = fixture()?;
            let path = fixture.path().join(storage);
            if !use_git {
                let repo = gix::open_opts(&path, crate::restricted())?;
                let dry_run = repo.collect_garbage(
                    Options {
                        expire: time(1_009_843_200),
                        dry_run: true,
                    },
                    &AtomicBool::default(),
                )?;
                assert_eq!(dry_run.expired_objects, 1, "only the unrelated old blob can expire");
                assert_eq!(
                    dry_run.retained_unreachable_objects, 3,
                    "the recent commit protects its older tree and blob"
                );
                assert_eq!(dry_run.written_packs, 0);
                assert_retention(&path, true, true)?;
            }
            for (cutoff, recent) in [(1_009_843_200, true), (1_009_843_200, true), (1_072_915_200, false)] {
                if use_git {
                    gix_testtools::git(&path, &format!("gc --prune=@{cutoff}"))?;
                } else {
                    gix::open_opts(&path, crate::restricted())?.collect_garbage(
                        Options {
                            expire: time(cutoff),
                            dry_run: false,
                        },
                        &AtomicBool::default(),
                    )?;
                }
                assert_retention(&path, recent, false)?;
                gix_testtools::git(&path, "fsck --strict --no-reflogs")?;
            }
        }
    }
    Ok(())
}

#[test]
fn kept_packs_protect_their_loose_dependencies() -> TestResult {
    {
        let fixture = fixture()?;
        let path = fixture.path().join("loose");
        let recent = ids(&path, "recent-ids")?;
        let input = path.join("one-object");
        fs::write(&input, format!("{}\n", recent[2]))?;
        let output = gix_testtools::git_command(&path)
            .args(["pack-objects", "objects/pack/pack"])
            .stdin(fs::File::open(input)?)
            .output()?;
        assert!(output.status.success(), "Git writes a pack containing just the commit");
        let name = String::from_utf8(output.stdout)?;
        let pack_base = path.join("objects/pack").join(format!("pack-{}", name.trim()));
        let keep = pack_base.with_extension("keep");
        fs::write(&keep, b"owned by another caller")?;
        // A kept pack can contain only a commit. Protect its loose dependencies too,
        // even though Git currently assumes kept packs already contain a complete graph.
        {
            gix::open_opts(&path, crate::restricted())?.collect_garbage(
                Options {
                    expire: SystemTime::now(),
                    dry_run: false,
                },
                &AtomicBool::default(),
            )?;
        }
        assert_retention(&path, true, false)?;
        assert_eq!(
            fs::read(keep)?,
            b"owned by another caller",
            "other callers' keep files survive unchanged"
        );
    }
    Ok(())
}

#[test]
fn retain_all_repacking_preserves_objects_and_cruft_ages() -> TestResult {
    for storage in ["loose", "cruft"] {
        let fixture = fixture()?;
        let path = fixture.path().join(storage);
        let repo = gix::open_opts(&path, crate::restricted())?;
        let outcome = repo.repack_objects(&AtomicBool::default())?;
        assert_eq!(outcome.objects, 7, "even unreachable expired objects are repacked");
        assert_retention(&path, true, true)?;
        assert_eq!(
            gix::odb::loose::Store::at(path.join("objects"), repo.object_hash())
                .iter()
                .count(),
            0
        );
        gix_testtools::git(&path, "fsck --strict --no-reflogs")?;
        repo.collect_garbage(
            Options {
                expire: time(1_009_843_200),
                dry_run: false,
            },
            &AtomicBool::default(),
        )?;
        assert_retention(&path, true, false)?;
    }
    Ok(())
}

#[test]
fn retain_all_repacking_preserves_promisor_boundaries_and_hydrated_objects() -> TestResult {
    let fixture = fixture()?;
    let path = fixture.path().join("loose");
    let repo = gix::open_opts(&path, crate::restricted())?;
    let live = ids(&path, "live-ids")?;
    let recent = ids(&path, "recent-ids")?;
    let input = path.join("promised-objects");
    fs::write(&input, format!("{}\n{}\n", live[1], live[2]))?;
    let output = gix_testtools::git_command(&path)
        .args(["pack-objects", "objects/pack/pack"])
        .stdin(fs::File::open(input)?)
        .output()?;
    assert!(output.status.success());
    let pack_name = String::from_utf8(output.stdout)?;
    let promisor = path
        .join("objects/pack")
        .join(format!("pack-{}.promisor", pack_name.trim()));
    fs::write(&promisor, b"origin")?;
    for object_id in &live {
        let hex = object_id.to_string();
        fs::remove_file(path.join("objects").join(&hex[..2]).join(&hex[2..]))?;
    }
    gix_testtools::git(&path, "config remote.origin.promisor true")?;
    let outcome = repo.repack_objects(&AtomicBool::default())?;
    assert_eq!(
        outcome.objects, 6,
        "missing promised objects need not be hydrated to repack"
    );
    let repo = gix::open_opts(&path, crate::restricted())?;
    assert!(repo.try_find_object(live[0])?.is_none());
    assert!(
        repo.try_find_object(recent[0])?.is_some(),
        "hydrated objects outside the available graph survive"
    );
    for entry in fs::read_dir(path.join("objects/pack"))? {
        let path = entry?.path();
        if path.extension().is_some_and(|ext| ext == "idx") {
            let index = gix::odb::pack::index::File::at(&path, repo.object_hash())?;
            if path.with_extension("promisor").exists() {
                assert_eq!(index.num_objects(), 2, "local objects must not acquire promisor status");
            } else {
                assert_eq!(index.num_objects(), 4);
            }
        }
    }
    gix_testtools::git(&path, "fsck --strict --no-reflogs")?;
    Ok(())
}

#[test]
fn failures_and_interrupts_do_not_remove_objects() -> TestResult {
    for failure in [
        "interrupt",
        "pid-lock",
        "pid",
        "corrupt-index",
        "corrupt-times",
        "precious",
        "partial-clone",
        "promisor-remote",
    ] {
        let fixture = fixture()?;
        let path = fixture.path().join("cruft");
        let pack_directory = path.join("objects/pack");
        match failure {
            "pid-lock" => fs::write(path.join("gc.pid.lock"), b"another writer")?,
            "pid" => fs::write(path.join("gc.pid"), b"1 another-host")?,
            "corrupt-index" | "corrupt-times" => {
                let extension = if failure == "corrupt-index" { "idx" } else { "mtimes" };
                let target = fs::read_dir(&pack_directory)?
                    .map(|entry| entry.map(|entry| entry.path()))
                    .collect::<std::io::Result<Vec<_>>>()?
                    .into_iter()
                    .find(|path| path.extension().is_some_and(|ext| ext == extension))
                    .expect("fixture has metadata");
                let mut bytes = fs::read(&target)?;
                let last = bytes.len() - 1;
                bytes[last] ^= 1;
                fs::remove_file(&target)?;
                fs::write(target, bytes)?;
            }
            "precious" => {
                gix_testtools::git(&path, "config extensions.preciousObjects true")?;
            }
            "partial-clone" => {
                gix_testtools::git(&path, "config extensions.partialClone origin")?;
            }
            "promisor-remote" => {
                gix_testtools::git(&path, "config remote.origin.promisor true")?;
            }
            _ => {}
        }
        let before = fs::read_dir(&pack_directory)?.count();
        let repo = gix::open_opts(&path, crate::restricted())?;
        repo.collect_garbage(
            Options {
                expire: SystemTime::now(),
                dry_run: false,
            },
            &AtomicBool::new(failure == "interrupt"),
        )
        .expect_err("invalid or concurrently maintained state fails before deletion");
        if !matches!(failure, "partial-clone" | "promisor-remote") {
            repo.repack_objects(&AtomicBool::new(failure == "interrupt"))
                .expect_err("repacking has the same validation and collector exclusion");
        }
        assert_eq!(
            fs::read_dir(&pack_directory)?.count(),
            before,
            "no pack files were removed for {failure}"
        );
    }
    Ok(())
}

#[test]
fn recent_objects_hooks_protect_graphs_and_propagate_failures() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for use_git in [true, false] {
        let fixture = fixture()?;
        let path = fixture.path().join("loose");
        let commit_id = ids(&path, "recent-ids")?[2];
        gix_testtools::git_command(&path)
            .args(["config", "gc.recentObjectsHook", &format!("printf '%s\\n' {commit_id}")])
            .output()?;
        if use_git {
            gix_testtools::git(&path, "gc --prune=now")?;
        } else {
            gix::open_opts(&path, crate::restricted())?.collect_garbage(
                Options {
                    expire: SystemTime::now(),
                    dry_run: false,
                },
                &AtomicBool::default(),
            )?;
        }
        assert_retention(&path, true, false)?;
    }
    for suffix in ["\\r\\n", ""] {
        let fixture = fixture()?;
        let path = fixture.path().join("loose");
        let commit_id = ids(&path, "recent-ids")?[2];
        let output = gix_testtools::git_command(&path)
            .args([
                "config",
                "gc.recentObjectsHook",
                &format!("printf '{commit_id}{suffix}'"),
            ])
            .output()?;
        assert!(output.status.success());
        gix::open_opts(&path, crate::restricted())?.collect_garbage(
            Options {
                expire: SystemTime::now(),
                dry_run: false,
            },
            &AtomicBool::default(),
        )?;
        assert_retention(&path, true, false)?;
    }
    for command in ["exit 1", "printf invalid", "printf '%02000d' 0"] {
        let fixture = fixture()?;
        let path = fixture.path().join("loose");
        let output = gix_testtools::git_command(&path)
            .args(["config", "gc.recentObjectsHook", command])
            .output()?;
        assert!(output.status.success());
        gix::open_opts(&path, crate::restricted())?
            .collect_garbage(
                Options {
                    expire: SystemTime::now(),
                    dry_run: false,
                },
                &AtomicBool::default(),
            )
            .expect_err("hook errors stop collection");
        assert_retention(&path, true, true)?;
    }
    Ok(())
}

#[test]
fn recent_objects_hook_without_output_can_be_cancelled() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let path = fixture.path().join("loose");
    let output = gix_testtools::git_command(&path)
        .args(["config", "gc.recentObjectsHook", "while :; do :; done"])
        .output()?;
    assert!(output.status.success());
    let repo = gix::open_opts(&path, crate::restricted())?;
    let interrupt = AtomicBool::default();
    let result = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(100));
            interrupt.store(true, std::sync::atomic::Ordering::Relaxed);
        });
        repo.collect_garbage(
            Options {
                expire: SystemTime::now(),
                dry_run: false,
            },
            &interrupt,
        )
    });
    assert!(
        result
            .expect_err("a silent hook must not block cancellation")
            .is_cancelled()
    );
    assert_retention(&path, true, true)?;
    Ok(())
}

#[test]
fn alternates_are_read_only_and_collection_does_not_require_git() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let source = fixture.path().join("loose");
    let destination = fixture.path().join("borrower");
    fs::create_dir(&destination)?;
    gix_testtools::git(&destination, "init --bare")?;
    let alternate = source.join("objects").canonicalize()?;
    fs::write(
        destination.join("objects/info/alternates"),
        format!("{}\n", alternate.display()),
    )?;
    let live_commit_id = ids(&source, "live-ids")?[2];
    gix_testtools::git(&destination, &format!("update-ref refs/heads/main {live_commit_id}"))?;
    let repo = gix::open_opts(&destination, crate::restricted())?;
    let expired_blob_id = repo.write_blob(b"local expired object")?.detach();
    let old_path =
        gix::odb::loose::Store::at(destination.join("objects"), repo.object_hash()).object_path(&expired_blob_id);
    fs::File::open(old_path)?.set_times(fs::FileTimes::new().set_modified(time(978_307_200)))?;
    let empty_path = fixture.path().join("no-programs");
    fs::create_dir(&empty_path)?;
    let guard = gix_testtools::Env::new().set("PATH", empty_path.to_string_lossy());
    let outcome = repo.collect_garbage(
        Options {
            expire: time(1_009_843_200),
            dry_run: false,
        },
        &AtomicBool::default(),
    )?;
    assert_eq!(
        outcome.objects_before, 1,
        "alternate objects do not enter the local inventory"
    );
    assert_eq!(outcome.expired_objects, 1);
    assert!(repo.try_find_object(expired_blob_id)?.is_none());
    assert!(repo.try_find_object(live_commit_id)?.is_some());
    assert_retention(&source, true, true)?;
    drop(guard);
    gix_testtools::git(&destination, "fsck --strict --no-reflogs")?;
    Ok(())
}

#[test]
fn hooks_can_introduce_new_objects_that_protect_older_dependencies() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = fixture()?;
    let path = fixture.path().join("loose");
    let repo = gix::open_opts(&path, crate::restricted())?;
    let recent = ids(&path, "recent-ids")?;
    let mut commit = repo.find_commit(recent[2])?.decode()?.to_owned()?;
    commit.message = "introduced during collection".into();
    let commit_id = repo.write_object(&commit)?.detach();
    let object_path = gix::odb::loose::Store::at(path.join("objects"), repo.object_hash()).object_path(&commit_id);
    fs::rename(&object_path, path.join("hook-object"))?;
    let hex = commit_id.to_string();
    let command = format!(
        "cp hook-object objects/{}/{} && printf '%s\\n' {commit_id}",
        &hex[..2],
        &hex[2..]
    );
    let result = gix_testtools::git_command(&path)
        .args(["config", "gc.recentObjectsHook", &command])
        .output()?;
    assert!(result.status.success());
    let repo = gix::open_opts(&path, crate::restricted())?;
    repo.collect_garbage(
        Options {
            expire: SystemTime::now(),
            dry_run: false,
        },
        &AtomicBool::default(),
    )?;
    let repo = gix::open_opts(&path, crate::restricted())?;
    for object_id in [commit_id, recent[0], recent[1]] {
        assert!(
            repo.try_find_object(object_id)?.is_some(),
            "a new hook tip protects its old dependencies"
        );
    }
    assert!(
        repo.try_find_object(recent[2])?.is_none(),
        "the old unrelated commit can expire"
    );
    Ok(())
}

#[test]
fn full_gc_expires_logs_before_objects_and_honors_reference_packing_configuration() -> TestResult {
    for kind in ["bare", "worktree"] {
        for pack_refs in ["true", "false", "notbare"] {
            for use_git in [true, false] {
                let fixture = gix_testtools::scripted_fixture_writable("make_full_gc.sh")?;
                let path = fixture.path().join(kind);
                gix_testtools::git(&path, &format!("config gc.packRefs {pack_refs}"))?;
                let repo = gix::open_opts(&path, crate::restricted())?;
                let old_id = ids(&path, "old-id")?[0];
                if use_git {
                    gix_testtools::git(&path, "gc --prune=now")?;
                } else {
                    let options = Options {
                        expire: SystemTime::now() + Duration::from_secs(1),
                        dry_run: true,
                    };
                    let outcome = repo.gc(options, &AtomicBool::default())?;
                    assert!(outcome.reflogs.removed > 0);
                    assert!(repo.try_find_object(old_id)?.is_some(), "dry run preserves objects");
                    assert!(repo.common_dir().join("refs/heads/main").is_file());
                    repo.gc(
                        Options {
                            dry_run: false,
                            ..options
                        },
                        &AtomicBool::default(),
                    )?;
                }
                let repo = gix::open_opts(&path, crate::restricted())?;
                assert!(
                    repo.try_find_object(old_id)?.is_none(),
                    "expired logs no longer protect old commits"
                );
                assert!(repo.head_commit().is_ok(), "the live branch remains valid");
                let packed = pack_refs == "true" || (pack_refs == "notbare" && kind != "bare");
                assert_eq!(repo.common_dir().join("refs/heads/main").is_file(), !packed);
                gix_testtools::git(&path, "fsck --strict")?;
            }
        }
    }
    Ok(())
}
