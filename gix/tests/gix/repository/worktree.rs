use gix_ref::bstr;

#[cfg(feature = "worktree-mutation")]
mod create {
    use std::sync::atomic::AtomicBool;

    use gix::refs::{FullName, transaction::PreviousValue};

    fn branch(name: &str) -> FullName {
        format!("refs/heads/{name}").try_into().expect("valid test branch name")
    }

    #[test]
    fn attached_and_detached_worktrees_are_checked_out_and_recognized_by_git() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let commit_id = repo.head_id()?.detach();
        let topic = branch("topic");
        repo.reference(
            topic.clone(),
            commit_id,
            PreviousValue::MustNotExist,
            "create worktree test branch",
        )?;

        let attached_path = destinations.path().join("attached");
        let (attached, attached_outcome) = repo.create_worktree(
            &attached_path,
            gix::worktree::create::Head::Attached(topic.clone()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert_eq!(attached.head_name()?, Some(topic));
        assert_eq!(attached.head_id()?.detach(), commit_id);
        assert!(attached_outcome.files_updated > 0, "the target tree was checked out");
        assert!(attached_path.join("this").is_file(), "tracked files are present");
        assert!(!attached.index()?.entries().is_empty(), "the linked index was written");
        assert_eq!(
            gix_testtools::git(&attached_path, "status --porcelain")?,
            "",
            "the checkout and its index agree"
        );

        let detached_path = destinations.path().join("detached");
        let (detached, _) = repo.create_worktree(
            &detached_path,
            gix::worktree::create::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert_eq!(detached.head_name()?, None);
        assert_eq!(detached.head_id()?.detach(), commit_id);

        let listing = gix_testtools::git(repo.workdir().expect("non-bare fixture"), "worktree list --porcelain")?;
        assert!(
            listing.contains(attached_path.to_str().expect("test paths are UTF-8")),
            "Git recognizes the attached worktree"
        );
        assert!(
            listing.contains(detached_path.to_str().expect("test paths are UTF-8")),
            "Git recognizes the detached worktree"
        );
        Ok(())
    }

    #[test]
    fn creates_a_worktree_from_a_bare_parent() -> crate::Result {
        let Some(fixture) = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            "make_worktree_repo.sh",
            ["bare"],
            gix_testtools::Creation::CopyFromReadOnly,
            |version| version >= (2, 31, 0),
        )?
        else {
            return Ok(());
        };
        let repo = gix::open_opts(fixture.path().join("repo.git"), crate::restricted())?;
        assert!(repo.is_bare(), "the creating repository has no main worktree");
        let destination = fixture.path().join("created-from-bare");
        let main = branch("main");

        let (worktree, _) = repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Attached(main.clone()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;

        assert_eq!(worktree.head_name()?, Some(main));
        assert_eq!(worktree.workdir(), Some(gix_path::realpath(&destination)?.as_path()));
        assert_eq!(gix_testtools::git(&destination, "status --porcelain")?, "");
        Ok(())
    }

    #[test]
    fn validation_failures_leave_the_destination_absent() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("rejected");
        let main = branch("main");

        let err = repo
            .create_worktree(
                &destination,
                gix::worktree::create::Head::Attached(main.clone()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("the main branch is already checked out");
        assert!(
            matches!(err, gix::worktree::create::Error::CheckedOut { name, .. } if name == main),
            "the checked-out branch is identified"
        );
        assert!(!destination.exists(), "validation happens before creating files");

        let interrupted = AtomicBool::new(true);
        let err = repo
            .create_worktree(
                &destination,
                gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &interrupted,
            )
            .expect_err("an already-interrupted operation does no work");
        assert!(matches!(err, gix::worktree::create::Error::Interrupted));
        assert!(!destination.exists(), "interruption leaves no destination behind");

        std::fs::create_dir(&destination)?;
        std::fs::write(destination.join("keep"), b"user data")?;
        let err = repo
            .create_worktree(
                &destination,
                gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("a non-empty destination is rejected");
        assert!(matches!(err, gix::worktree::create::Error::Prepare(_)));
        assert_eq!(
            std::fs::read(destination.join("keep"))?,
            b"user data",
            "failed creation preserves existing destination contents"
        );
        Ok(())
    }

    #[test]
    fn registered_destinations_are_rejected_even_when_missing_or_empty() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("registered");
        let commit_id = repo.head_id()?.detach();
        repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;

        std::fs::remove_dir_all(&destination)?;
        for exists in [false, true] {
            if exists {
                std::fs::create_dir(&destination)?;
            }
            let err = repo
                .create_worktree(
                    &destination,
                    gix::worktree::create::Head::Detached(commit_id),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("registered destinations cannot be reused");
            assert!(
                matches!(err, gix::worktree::create::Error::DestinationRegistered { destination: actual } if actual == destination),
                "the registered destination is identified"
            );
        }

        let case_variant = destination.with_file_name("REGISTERED");
        if case_variant.exists() {
            let err = repo
                .create_worktree(
                    &case_variant,
                    gix::worktree::create::Head::Detached(commit_id),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("filesystem-equivalent casing cannot bypass registration");
            assert!(
                matches!(err, gix::worktree::create::Error::DestinationRegistered { destination } if destination == case_variant),
                "the caller's case variant is identified"
            );
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn registered_destinations_are_matched_through_symlinked_parents() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let actual_parent = destinations.path().join("actual");
        let linked_parent = destinations.path().join("linked");
        std::fs::create_dir(&actual_parent)?;
        std::os::unix::fs::symlink(&actual_parent, &linked_parent)?;
        let destination = actual_parent.join("registered");
        let commit_id = repo.head_id()?.detach();
        repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        std::fs::remove_dir_all(&destination)?;

        let alias = linked_parent.join("registered");
        let err = repo
            .create_worktree(
                &alias,
                gix::worktree::create::Head::Detached(commit_id),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("registered destinations are compared by their real paths");
        assert!(
            matches!(err, gix::worktree::create::Error::DestinationRegistered { destination } if destination == alias),
            "the alias supplied by the caller is identified"
        );
        Ok(())
    }
}

#[cfg(feature = "worktree-mutation")]
mod remove {
    use std::sync::atomic::AtomicBool;

    use gix::{
        refs::{FullName, transaction::PreviousValue},
        worktree::remove::Force,
    };

    fn branch(name: &str) -> FullName {
        format!("refs/heads/{name}").try_into().expect("valid test branch name")
    }

    #[test]
    fn removes_a_clean_worktree_by_suffix_without_deleting_its_branch() -> crate::Result {
        let (mut repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("nested/topic-checkout");
        let topic = branch("remove-topic");
        repo.reference(
            topic.clone(),
            repo.head_id()?.detach(),
            PreviousValue::MustNotExist,
            "create worktree removal test branch",
        )?;
        let (linked, _) = repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Attached(topic.clone()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = linked.git_dir().to_owned();
        drop(linked);
        let malformed = repo.common_dir().join("worktrees/malformed");
        std::fs::create_dir(&malformed)?;
        std::fs::write(malformed.join("gitdir"), b"not a gitdir\n")?;
        repo.config_snapshot_mut()
            .set_value(&gix::config::tree::Core::IGNORE_CASE, "true")?;

        let target = repo.prepare_remove_worktree("TOPIC-CHECKOUT")?;
        assert_eq!(target.base(), gix_path::realpath(&destination)?);
        assert_eq!(
            target.repository()?.head_name()?,
            Some(topic.clone()),
            "the resolved worktree can be inspected before removal"
        );
        target.remove(Force::Never, gix::progress::Discard)?;

        assert!(!destination.exists(), "the checkout is removed");
        assert!(!private_git_dir.exists(), "the registration is removed");
        assert!(
            repo.try_find_reference(topic.as_ref())?.is_some(),
            "core worktree removal leaves the attached branch untouched"
        );
        Ok(())
    }

    #[test]
    fn permits_removing_the_current_linked_worktree_but_not_the_main_worktree() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("current");
        let (linked, _) = repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = linked.git_dir().to_owned();

        linked.remove_worktree(&destination, Force::Never, gix::progress::Discard)?;
        assert!(!destination.exists(), "the current linked checkout is removed");
        assert!(!private_git_dir.exists(), "its registration is removed");

        let main_path = repo.workdir().expect("non-bare fixture").to_owned();
        let err = repo
            .remove_worktree(&main_path, Force::OverrideLock, gix::progress::Discard)
            .expect_err("the main worktree is never removable");
        assert!(matches!(err, gix::worktree::remove::Error::MainWorktree { path } if path == main_path));
        Ok(())
    }

    #[test]
    fn dirty_and_locked_worktrees_require_the_corresponding_force_level() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let dirty_path = gix_path::realpath(destinations.path().join("dirty"))?;
        let (dirty, _) = repo.create_worktree(
            &dirty_path,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        std::fs::write(dirty_path.join("untracked"), b"changes")?;
        drop(dirty);
        let err = repo
            .remove_worktree(&dirty_path, Force::Never, gix::progress::Discard)
            .expect_err("an untracked file makes the worktree dirty");
        assert!(
            matches!(err, gix::worktree::remove::Error::Dirty { ref path } if path == &dirty_path),
            "an untracked file is rejected as dirty, got {err:?}"
        );
        repo.remove_worktree(&dirty_path, Force::DiscardChanges, gix::progress::Discard)?;

        let locked_path = gix_path::realpath(destinations.path().join("locked"))?;
        let (locked, _) = repo.create_worktree(
            &locked_path,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = locked.git_dir().to_owned();
        std::fs::write(private_git_dir.join("locked"), b"on external storage\n")?;
        drop(locked);
        let err = repo
            .remove_worktree(&locked_path, Force::DiscardChanges, gix::progress::Discard)
            .expect_err("one force does not override a lock");
        assert!(
            matches!(err, gix::worktree::remove::Error::Locked { path, reason: Some(reason) }
                if path == locked_path && reason == "on external storage"),
            "the lock and its reason are reported"
        );
        repo.remove_worktree(&locked_path, Force::OverrideLock, gix::progress::Discard)?;
        Ok(())
    }

    #[test]
    fn initialized_submodules_require_force() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("submodules");
        let (linked, _) = repo.create_worktree(
            &destination,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        std::fs::create_dir(linked.git_dir().join("modules"))?;
        drop(linked);

        let err = repo
            .remove_worktree(&destination, Force::Never, gix::progress::Discard)
            .expect_err("initialized submodules prevent an unforced removal");
        assert!(matches!(err, gix::worktree::remove::Error::ContainsSubmodule { .. }));
        repo.remove_worktree(&destination, Force::DiscardChanges, gix::progress::Discard)?;
        Ok(())
    }

    #[test]
    fn backlink_validation_is_never_forced_and_missing_checkouts_are_unregistered() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let invalid_path = destinations.path().join("invalid-backlink");
        let (invalid, _) = repo.create_worktree(
            &invalid_path,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = invalid.git_dir().to_owned();
        std::fs::write(invalid_path.join(".git"), "gitdir: ../elsewhere\n")?;
        drop(invalid);
        let err = repo
            .remove_worktree(&invalid_path, Force::OverrideLock, gix::progress::Discard)
            .expect_err("force cannot bypass backlink validation");
        assert!(matches!(err, gix::worktree::remove::Error::BacklinkMismatch { .. }));
        assert!(invalid_path.exists(), "an invalid checkout is retained");
        assert!(private_git_dir.exists(), "an invalid registration is retained");

        let missing_path = destinations.path().join("missing");
        let (missing, _) = repo.create_worktree(
            &missing_path,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = missing.git_dir().to_owned();
        drop(missing);
        std::fs::remove_dir_all(&missing_path)?;
        let target = repo.prepare_remove_worktree(&missing_path)?;
        assert_eq!(
            target.repository()?.head_id()?.detach(),
            repo.head_id()?.detach(),
            "private metadata remains inspectable without the checkout"
        );
        target.remove(Force::Never, gix::progress::Discard)?;
        assert!(!private_git_dir.exists(), "a missing checkout is unregistered");

        let blocked_parent = destinations.path().join("non-directory");
        let blocked_path = blocked_parent.join("missing");
        let (blocked, _) = repo.create_worktree(
            &blocked_path,
            gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let private_git_dir = blocked.git_dir().to_owned();
        drop(blocked);
        std::fs::remove_dir_all(&blocked_parent)?;
        std::fs::write(&blocked_parent, b"not a directory")?;
        repo.remove_worktree(&blocked_path, Force::Never, gix::progress::Discard)?;
        assert!(
            !private_git_dir.exists(),
            "a checkout hidden behind a non-directory ancestor is unregistered"
        );
        Ok(())
    }

    #[test]
    fn ambiguous_suffixes_can_be_disambiguated_with_an_exact_path() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let first_path = destinations.path().join("one/shared");
        let second_path = destinations.path().join("two/shared");
        for destination in [&first_path, &second_path] {
            repo.create_worktree(
                destination,
                gix::worktree::create::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )?;
        }

        let err = repo
            .remove_worktree("shared", Force::Never, gix::progress::Discard)
            .expect_err("a non-unique suffix is ambiguous");
        assert!(
            matches!(err, gix::worktree::remove::Error::Ambiguous { candidates, .. } if candidates.len() == 2),
            "all suffix matches are reported"
        );
        repo.remove_worktree(&first_path, Force::Never, gix::progress::Discard)?;
        assert!(!first_path.exists(), "the exact match was removed");
        assert!(second_path.exists(), "the other suffix match remains");

        let missing = destinations.path().join("does-not-exist");
        let err = repo
            .remove_worktree(&missing, Force::Never, gix::progress::Discard)
            .expect_err("an unknown path is reported as such");
        assert!(matches!(err, gix::worktree::remove::Error::NotFound { target } if target == missing));
        Ok(())
    }
}

/// The buffer length for SHA1 archives.
#[cfg(target_pointer_width = "64")]
#[cfg(feature = "worktree-stream")]
const EXPECTED_BUFFER_LENGTH: usize = 102;
/// The buffer length for SHA1 archives on 32bit machines.
#[cfg(target_pointer_width = "32")]
#[cfg(feature = "worktree-stream")]
const EXPECTED_BUFFER_LENGTH: usize = 86;

#[cfg(feature = "worktree-stream")]
fn expected_buffer_length(repo: &gix::Repository) -> usize {
    EXPECTED_BUFFER_LENGTH + repo.object_hash().len_in_hex() - gix::hash::Kind::Sha1.len_in_hex()
}

#[test]
#[cfg(feature = "worktree-stream")]
fn stream() -> crate::Result {
    let repo = crate::named_repo("make_packed_and_loose.sh")?;
    let mut stream = repo.worktree_stream(repo.head_commit()?.tree_id()?)?.0.into_read();
    assert_eq!(
        std::io::copy(&mut stream, &mut std::io::sink())?,
        expected_buffer_length(&repo) as u64,
        "there is some content in the stream, it works"
    );
    Ok(())
}

#[test]
#[cfg(feature = "worktree-archive")]
fn archive() -> crate::Result {
    let repo = crate::named_repo("make_packed_and_loose.sh")?;
    let (stream, _index) = repo.worktree_stream(repo.head_commit()?.tree_id()?)?;
    let mut buf = Vec::<u8>::new();

    repo.worktree_archive(
        stream,
        std::io::Cursor::new(&mut buf),
        gix_features::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
        Default::default(),
    )?;
    assert_eq!(buf.len(), expected_buffer_length(&repo), "default format is internal");
    Ok(())
}

mod with_core_worktree_config {
    use std::io::BufRead;

    #[test]
    #[cfg(feature = "index")]
    fn relative() -> crate::Result {
        for (name, is_relative) in [("absolute-worktree", false), ("relative-worktree", true)] {
            let repo = repo(name);

            if is_relative {
                assert_eq!(
                    repo.workdir().unwrap(),
                    repo.git_dir().parent().unwrap().parent().unwrap().join("worktree"),
                    "{name}|{is_relative}: work_dir is set to core.worktree config value, relative paths are appended to `git_dir() and made absolute`"
                );
            } else {
                assert_eq!(
                    repo.workdir().unwrap(),
                    gix_path::realpath(repo.git_dir().parent().unwrap().parent().unwrap().join("worktree"))?,
                    "absolute workdirs are left untouched"
                );
            }

            assert_eq!(
                repo.worktree().expect("present").base(),
                repo.workdir().unwrap(),
                "current worktree is based on work-tree dir"
            );

            let baseline = crate::repository::worktree::Baseline::collect(repo.git_dir())?;
            assert_eq!(baseline.len(), 1, "git lists the main worktree");
            assert_eq!(
                baseline[0].root,
                gix_path::realpath(repo.git_dir().parent().unwrap())?,
                "git lists the original worktree, to which we have no access anymore"
            );
            assert_eq!(
                repo.worktrees()?.len(),
                0,
                "we only list linked worktrees, and there are none"
            );
            assert_eq!(
                repo.index()?.entries().len(),
                count_deleted(repo.git_dir()),
                "git considers all worktree entries missing as the overridden worktree is an empty dir"
            );
            assert_eq!(repo.index()?.entries().len(), 3, "just to be sure");
        }
        Ok(())
    }

    #[test]
    fn non_existing_relative() {
        let repo = repo("relative-nonexisting-worktree");
        assert_eq!(
            count_deleted(repo.git_dir()),
            0,
            "git can't chdir into missing worktrees, has no error handling there"
        );

        assert!(
            !repo.workdir().expect("configured").exists(),
            "non-existing or invalid worktrees (this one is a file) are taken verbatim and \
            may lead to errors later - just like in `git` and we explicitly do not try to be smart about it"
        );
    }

    #[test]
    fn relative_file() {
        let repo = repo("relative-worktree-file");
        assert_eq!(count_deleted(repo.git_dir()), 0, "git can't chdir into a file");

        assert!(
            repo.workdir().expect("configured").is_file(),
            "non-existing or invalid worktrees (this one is a file) are taken verbatim and \
            may lead to errors later - just like in `git` and we explicitly do not try to be smart about it"
        );
    }

    #[test]
    #[cfg(feature = "index")]
    fn bare_relative() -> crate::Result {
        let repo = repo("bare-relative-worktree");

        assert_eq!(
            count_deleted(repo.git_dir()),
            0,
            "git refuses to mix bare with core.worktree"
        );
        assert!(
            repo.workdir().is_none(),
            "we simply don't load core.worktree in bare repos either to match this behaviour"
        );
        assert!(repo.try_index()?.is_none());
        assert!(repo.index_or_empty()?.entries().is_empty());
        Ok(())
    }

    #[test]
    #[cfg(unix)] // symlinks are used here, let's not try our luck on Windows.
    fn relative_through_symlinked_ancestor_keeps_callers_path_namespace() -> crate::Result {
        let link = gix_testtools::scripted_fixture_read_only("make_core_worktree_repo.sh")?.join("symlinked-ancestor");

        let repo = gix::open_opts(link.join("relative-worktree"), crate::restricted())?;
        assert_eq!(
            repo.workdir(),
            Some(link.join("worktree").as_path()),
            "if a symlink in an ancestor changes nothing about how the relative worktree resolves, \
             the caller's path namespace is kept instead of jumping to the canonicalized one"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)] // symlinks are used here, let's not try our luck on Windows.
    fn relative_from_symlinked_git_dir() -> crate::Result {
        let fixture = gix_testtools::scripted_fixture_read_only("make_core_worktree_repo.sh")?;
        let root = fixture.join("linked-git-dir-detached-worktree");
        let repo = gix::open_opts(root.join("home"), crate::restricted())?;
        let git_worktree = std::fs::read_to_string(root.join("worktree.baseline"))?;

        assert_eq!(
            gix_path::realpath(repo.workdir().expect("core.worktree is configured"))?,
            gix_path::realpath(git_worktree.trim_end())?,
            "relative core.worktree values from repository config are resolved against the real git dir"
        );
        Ok(())
    }

    fn repo(name: &str) -> gix::Repository {
        let dir = gix_testtools::scripted_fixture_read_only("make_core_worktree_repo.sh").unwrap();
        gix::open_opts(dir.join(name), crate::restricted()).unwrap()
    }

    fn count_deleted(git_dir: &std::path::Path) -> usize {
        std::fs::read(git_dir.join("status.baseline"))
            .unwrap()
            .lines()
            .map_while(Result::ok)
            .filter(|line| line.contains(" D "))
            .count()
    }
}

struct Baseline<'a> {
    lines: bstr::Lines<'a>,
}

mod baseline {
    use std::{
        borrow::Cow,
        path::{Path, PathBuf},
    };

    use gix::bstr::{BString, ByteSlice};
    use gix_object::bstr::BStr;

    use super::Baseline;

    impl Baseline<'_> {
        pub fn collect(dir: impl AsRef<Path>) -> std::io::Result<Vec<Worktree>> {
            let content = std::fs::read(dir.as_ref().join("worktree-list.baseline"))?;
            Ok(Baseline { lines: content.lines() }.collect())
        }
    }

    pub type Reason = BString;

    #[derive(Debug)]
    pub struct Worktree {
        pub root: PathBuf,
        pub bare: bool,
        pub locked: Option<Reason>,
        pub peeled: gix_hash::ObjectId,
        pub branch: Option<BString>,
        pub prunable: Option<Reason>,
    }

    impl Iterator for Baseline<'_> {
        type Item = Worktree;

        fn next(&mut self) -> Option<Self::Item> {
            let root = gix_path::from_bstr(Cow::Borrowed(fields(self.lines.next()?).1)).into_owned();
            let mut bare = false;
            let mut branch = None;
            let mut peeled = gix_hash::ObjectId::null(gix_hash::Kind::Sha1);
            let mut locked = None;
            let mut prunable = None;
            for line in self.lines.by_ref() {
                if line.is_empty() {
                    break;
                }
                if line == b"bare" {
                    bare = true;
                    continue;
                } else if line == b"detached" {
                    continue;
                }
                let (field, value) = fields(line);
                match field {
                    f if f == "HEAD" => peeled = gix_hash::ObjectId::from_hex(value).expect("valid hash"),
                    f if f == "branch" => branch = Some(value.to_owned()),
                    f if f == "locked" => locked = Some(value.to_owned()),
                    f if f == "prunable" => prunable = Some(value.to_owned()),
                    _ => unreachable!("unknown field: {}", field),
                }
            }
            Some(Worktree {
                root,
                bare,
                locked,
                peeled,
                branch,
                prunable,
            })
        }
    }

    fn fields(line: &[u8]) -> (&BStr, &BStr) {
        let (a, b) = line.split_at(line.find_byte(b' ').expect("at least a space"));
        (a.as_bstr(), b[1..].as_bstr())
    }
}

#[test]
fn from_bare_parent_repo() {
    let Some(dir) = gix_testtools::scripted_fixture_read_only_with_args_with_git_version(
        "make_worktree_repo.sh",
        ["bare"],
        |version| version >= (2, 31, 0),
    )
    .unwrap() else {
        return;
    };
    let repo = gix::open_opts(dir.join("repo.git"), crate::restricted()).expect("fixture repository opens");

    run_assertions(repo, true /* bare */);
}

#[test]
fn from_nonbare_parent_repo() {
    let Some(dir) = gix_testtools::scripted_fixture_read_only_with_git_version("make_worktree_repo.sh", |version| {
        version >= (2, 31, 0)
    })
    .unwrap() else {
        return;
    };
    let repo = gix::open_opts(dir.join("repo"), crate::restricted()).expect("fixture repository opens");

    run_assertions(repo, false /* bare */);
}

#[test]
fn linked_worktree_proxy_base_with_relative_linking_files() -> crate::Result {
    let fixture = gix_testtools::scripted_fixture_read_only_needs_archive("make_worktree_relative_linking.sh")?;
    let main = fixture.join("main");
    let linked = fixture.join("linked");
    let private_git_dir = main.join(".git/worktrees/linked");
    let repo = gix::open_opts(&main, crate::restricted())?;
    let worktrees = repo.worktrees()?;
    assert_eq!(worktrees.len(), 1, "the relative-path fixture has one linked worktree");
    let proxy = worktrees.into_iter().next().expect("one worktree");

    assert_eq!(
        gix_path::realpath(proxy.base()?)?,
        gix_path::realpath(&linked)?,
        "proxy bases resolve relative worktrees/<id>/gitdir paths against the private git dir"
    );
    let linked_repo = proxy.into_repo()?;
    assert_eq!(
        linked_repo.workdir().map(gix_path::realpath).transpose()?,
        Some(gix_path::realpath(&linked)?)
    );
    assert_eq!(linked_repo.git_dir(), private_git_dir);

    Ok(())
}

#[test]
#[cfg(unix)]
fn linked_worktree_proxy_base_with_symlinked_main_repo() -> crate::Result {
    let fixture = gix_testtools::scripted_fixture_read_only_needs_archive("make_worktree_relative_linking.sh")?;
    let linked = fixture.join("actual/linked");
    let main_symlink = fixture.join("main-symlink");

    let repo = gix::open_opts(&main_symlink, crate::restricted())?;
    let worktrees = repo.worktrees()?;
    assert_eq!(worktrees.len(), 1, "the relative-path fixture has one linked worktree");
    let proxy = worktrees.into_iter().next().expect("one worktree");

    assert_eq!(
        gix_path::realpath(proxy.base()?)?,
        gix_path::realpath(&linked)?,
        "proxy bases preserve symlink semantics when resolving relative worktrees/<id>/gitdir paths"
    );
    let repo = proxy.into_repo()?;
    assert_eq!(
        repo.workdir().map(gix_path::realpath).transpose()?,
        Some(gix_path::realpath(&linked)?)
    );

    Ok(())
}

#[test]
fn from_nonbare_parent_repo_set_workdir() -> gix_testtools::Result {
    let Some(dir) = gix_testtools::scripted_fixture_read_only_with_git_version("make_worktree_repo.sh", |version| {
        version >= (2, 31, 0)
    })?
    else {
        return Ok(());
    };
    let mut repo = gix::open_opts(dir.join("repo"), crate::restricted()).expect("fixture repository opens");

    assert!(repo.worktree().is_some_and(|wt| wt.is_main()), "we have main worktree");

    let worktrees = repo.worktrees()?;
    assert_eq!(worktrees.len(), 6);

    let linked_wt_dir = worktrees.first().unwrap().base().expect("this linked worktree exists");
    repo.set_workdir(linked_wt_dir).expect("works as the dir exists");

    assert!(
        repo.worktree().is_some_and(|wt| wt.is_main()),
        "it's still the main worktree as that depends on the git_dir"
    );

    let mut wt_repo = repo.worktrees()?.first().unwrap().clone().into_repo()?;
    assert!(
        wt_repo.worktree().is_some_and(|wt| !wt.is_main()),
        "linked worktrees are never main"
    );

    wt_repo.set_workdir(Some(repo.workdir().unwrap().to_owned()))?;
    assert!(
        wt_repo.worktree().is_some_and(|wt| !wt.is_main()),
        "it's still the linked worktree as that depends on the git_dir"
    );

    Ok(())
}

fn run_assertions(main_repo: gix::Repository, should_be_bare: bool) {
    assert_eq!(main_repo.is_bare(), should_be_bare);
    assert_eq!(main_repo.kind(), gix::repository::Kind::Common);
    let mut baseline = Baseline::collect(
        main_repo
            .workdir()
            .map_or_else(|| main_repo.git_dir().parent(), std::path::Path::parent)
            .expect("a temp dir as parent"),
    )
    .unwrap();
    let expected_main = baseline.remove(0);
    assert_eq!(expected_main.bare, should_be_bare);

    if should_be_bare {
        assert!(main_repo.worktree().is_none());
    } else {
        assert_eq!(
            main_repo.workdir().expect("non-bare").canonicalize().unwrap(),
            expected_main.root.canonicalize().unwrap()
        );
        assert_eq!(main_repo.head_id().unwrap(), expected_main.peeled);
        assert_eq!(
            main_repo.head_name().unwrap().expect("no detached head"),
            expected_main.branch.unwrap()
        );
        let worktree = main_repo.worktree().expect("not bare");
        assert!(
            worktree.lock_reason().is_none(),
            "main worktrees, bare or not, are never locked"
        );
        assert!(!worktree.is_locked());
        assert!(worktree.is_main());
    }
    assert_eq!(main_repo.main_repo().unwrap(), main_repo, "main repo stays main repo");

    let actual = main_repo.worktrees().unwrap();
    assert_eq!(actual.len(), baseline.len());

    for actual in actual {
        let base = actual.base().unwrap();
        let expected = baseline
            .iter()
            .find(|exp| exp.root == base)
            .expect("we get the same root and it matches");
        assert!(
            !expected.bare,
            "only the main worktree can be bare, and we don't see it in this loop"
        );
        let proxy_lock_reason = actual.lock_reason();
        assert_eq!(proxy_lock_reason, expected.locked);
        let proxy_is_locked = actual.is_locked();
        assert_eq!(proxy_is_locked, proxy_lock_reason.is_some());
        // TODO: check id of expected worktree, but need access to .gitdir from worktree base
        let proxy_id = actual.id().to_owned();
        assert_eq!(
            base.is_dir(),
            expected.prunable.is_none(),
            "in our case prunable repos have no worktree base"
        );

        assert_eq!(
            main_repo.worktree_proxy_by_id(actual.id()).expect("exists").git_dir(),
            actual.git_dir(),
            "we can basically get the same proxy by its ID explicitly"
        );

        let repo = if base.is_dir() {
            let repo = actual.clone().into_repo().unwrap();
            assert_eq!(
                &gix::open_opts(base, crate::restricted()).expect("linked worktree repository opens"),
                &repo,
                "repos are considered the same no matter if opened from worktree or from git dir"
            );
            repo
        } else {
            assert!(
                matches!(
                    actual.clone().into_repo(),
                    Err(gix::worktree::proxy::into_repo::Error::MissingWorktree { .. })
                ),
                "missing bases are detected"
            );
            actual.clone().into_repo_with_possibly_inaccessible_worktree().unwrap()
        };
        let worktree = repo.worktree().expect("linked worktrees have at least a base path");
        assert!(!worktree.is_main());
        assert_eq!(worktree.lock_reason(), proxy_lock_reason);
        assert_eq!(worktree.is_locked(), proxy_is_locked);
        assert_eq!(worktree.id(), Some(proxy_id.as_ref()));
        assert_eq!(
            repo.main_repo().unwrap(),
            main_repo,
            "main repo from worktree repo is the actual main repo"
        );

        let proxy_by_id = repo
            .worktree_proxy_by_id(actual.id())
            .expect("can get the proxy from a linked repo as well");
        assert_ne!(
            proxy_by_id.git_dir(),
            actual.git_dir(),
            "The git directories might not look the same…"
        );
        assert_eq!(
            gix_path::realpath(proxy_by_id.git_dir()).ok(),
            gix_path::realpath(actual.git_dir()).ok(),
            "…but they are the same effectively"
        );
    }
}
