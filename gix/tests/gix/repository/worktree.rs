use gix_ref::bstr;

#[cfg(feature = "worktree-mutation")]
mod add {
    use std::sync::atomic::AtomicBool;

    use gix::bstr::ByteSlice;
    use gix::refs::{FullName, transaction::PreviousValue};
    use gix_path::{into_bstr, to_unix_separators_on_windows};

    fn branch(name: &str) -> FullName {
        format!("refs/heads/{name}").try_into().expect("valid test branch name")
    }

    #[test]
    #[cfg(unix)]
    fn shared_repository_permissions_match_git_with_shared_linking_files() -> crate::Result {
        use std::{fs, os::unix::fs::PermissionsExt};

        let mode =
            |path: &std::path::Path| -> std::io::Result<u32> { Ok(fs::metadata(path)?.permissions().mode() & 0o7777) };
        for (shared, worktree_config) in [("group", false), ("0640", false), ("false", false), ("0640", true)] {
            let (mut source, _fixture) = crate::basic_rw_repo()?;
            let work_dir = source.workdir().expect("source checkout").to_owned();
            gix_testtools::git(&work_dir, "add some-with-file")?;
            gix_testtools::git(
                &work_dir,
                "commit -m 'track nested files for checkout permission checks'",
            )?;
            if worktree_config {
                gix_testtools::git(&work_dir, "config extensions.worktreeConfig true")?;
                gix_testtools::git(&work_dir, "config core.sharedRepository group")?;
                gix_testtools::git(&work_dir, &format!("config --worktree core.sharedRepository {shared}"))?;
            } else {
                gix_testtools::git(&work_dir, &format!("config core.sharedRepository {shared}"))?;
            }
            source.reload()?;
            for attached in [true, false] {
                let suffix = if attached { "attached" } else { "detached" };
                let commit_id = source.head_id()?.detach();
                let head = if attached {
                    let topic = branch("gix-topic");
                    source.reference(topic.clone(), commit_id, PreviousValue::MustNotExist, "create topic")?;
                    gix::worktree::add::Head::Attached(topic)
                } else {
                    gix::worktree::add::Head::Detached(commit_id)
                };
                let parent = work_dir.join(format!("gix-{suffix}"));
                let destination = parent.join("worktree");
                let (created, _) =
                    source.add_worktree(&destination, head, gix::progress::Discard, &AtomicBool::default())?;
                gix_testtools::git(
                    &work_dir,
                    &format!(
                        "worktree add {} git-{suffix}/worktree HEAD",
                        if attached { "-b git-topic" } else { "--detach" }
                    ),
                )?;
                let git_parent = work_dir.join(format!("git-{suffix}"));
                let git_destination = git_parent.join("worktree");
                let git_repo = gix::open_opts(&git_destination, crate::restricted())?;
                for (actual, expected) in [
                    (source.common_dir().join("worktrees"), git_parent.clone()),
                    (parent, git_parent),
                    (destination.clone(), git_destination.clone()),
                    (created.git_dir().to_owned(), git_repo.git_dir().to_owned()),
                ] {
                    assert_eq!(
                        mode(&actual)?,
                        mode(&expected)?,
                        "shared={shared}: worktree directories follow Git's directory policy"
                    );
                }
                for name in ["HEAD", "logs", "logs/HEAD", "index"] {
                    assert_eq!(
                        mode(&created.git_dir().join(name))?,
                        mode(&git_repo.git_dir().join(name))?,
                        "shared={shared}: {name} follows Git's metadata policy"
                    );
                }
                for path in [
                    destination.join(".git"),
                    created.git_dir().join("gitdir"),
                    created.git_dir().join("commondir"),
                ] {
                    assert_eq!(
                        mode(&path)?,
                        mode(&git_repo.git_dir().join("HEAD"))?,
                        "shared={shared}: linking files deliberately use the repository metadata policy"
                    );
                }
                for name in ["this", "some-with-file"] {
                    assert_eq!(
                        mode(&destination.join(name))?,
                        mode(&git_destination.join(name))?,
                        "shared={shared}: checked-out {name} retains Git's ordinary filesystem permissions"
                    );
                }
                assert_eq!(
                    gix_testtools::git(&destination, "status --porcelain")?,
                    "",
                    "permissions do not change checkout contents"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn initial_head_reflog_respects_configuration_like_git() -> crate::Result {
        for (log_all_ref_updates, worktree_config) in [
            (None, false),
            (Some("true"), false),
            (Some("false"), false),
            (Some("always"), false),
            (Some("true"), true),
            (Some("false"), true),
        ] {
            let (mut source, _fixture) = crate::basic_rw_repo()?;
            let work_dir = source.workdir().expect("source checkout").to_owned();
            let commit_id = source.head_id()?.detach();
            let topic = branch("topic");
            source.reference(topic.clone(), commit_id, PreviousValue::MustNotExist, "create topic")?;
            let logging_enabled = log_all_ref_updates != Some("false");
            let mut config = source.config_file_mut(source.common_dir().join("config"))?;
            if worktree_config {
                config.set_raw_value("extensions.worktreeConfig", "true")?;
                config.set_raw_value("core.logAllRefUpdates", if logging_enabled { "false" } else { "true" })?;
                config.commit()?;
                config = source.config_file_mut(source.git_dir().join("config.worktree"))?;
            }
            if let Some(value) = log_all_ref_updates {
                config.set_raw_value("core.logAllRefUpdates", value)?;
            } else {
                config.raw_values_mut("core.logAllRefUpdates")?.delete_all();
            }
            config.commit()?;
            source.reload()?;

            for attached in [true, false] {
                gix_testtools::git(
                    &work_dir,
                    if attached {
                        "worktree add git-created topic"
                    } else {
                        "worktree add --detach git-created HEAD"
                    },
                )?;
                let git_log_path = source.common_dir().join("worktrees/git-created/logs/HEAD");
                assert_eq!(
                    git_log_path.is_file(),
                    logging_enabled,
                    "Git honors core.logAllRefUpdates={log_all_ref_updates:?} with worktreeConfig={worktree_config}"
                );
                let git_log = logging_enabled.then(|| std::fs::read(git_log_path)).transpose()?;
                gix_testtools::git(&work_dir, "worktree remove git-created")?;
                let source_log = std::fs::read(source.git_dir().join("logs/HEAD"))?;
                let branch_log = std::fs::read(source.common_dir().join("logs/refs/heads/topic"))?;
                let destination = work_dir.join(if attached { "gix-attached" } else { "gix-detached" });
                let (created, _) = source.add_worktree(
                    &destination,
                    if attached {
                        gix::worktree::add::Head::Attached(topic.clone())
                    } else {
                        gix::worktree::add::Head::Detached(commit_id)
                    },
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )?;
                assert_eq!(
                    created.head_id()?,
                    commit_id,
                    "initialization preserves the target commit"
                );
                assert_eq!(
                    created.head_name()?,
                    attached.then_some(topic.clone()),
                    "HEAD retains its attachment"
                );
                let head = created.find_reference("HEAD")?;
                assert_eq!(
                    head.log_exists(),
                    logging_enabled,
                    "new HEAD logging matches Git for attached={attached}, core.logAllRefUpdates={log_all_ref_updates:?}, worktreeConfig={worktree_config}"
                );
                if let Some(git_log) = git_log {
                    let actual_log = std::fs::read(created.git_dir().join("logs/HEAD"))?;
                    let initial = gix_ref::file::log::LineRef::from_bytes(&actual_log)?;
                    let git_initial = gix_ref::file::log::LineRef::from_bytes(&git_log)?;
                    assert_eq!(
                        initial.previous_oid(),
                        source.object_hash().null(),
                        "initial HEAD has no predecessor"
                    );
                    assert_eq!(
                        initial.new_oid(),
                        commit_id,
                        "the initial entry records the checked-out commit"
                    );
                    assert_eq!(
                        (initial.previous_oid, initial.new_oid, initial.message),
                        (git_initial.previous_oid, git_initial.new_oid, git_initial.message),
                        "the initial reflog transition matches Git"
                    );
                    assert_eq!(
                        initial.signature.name, "gitoxide",
                        "the configured committer writes the reflog"
                    );
                    assert_eq!(
                        gix_testtools::git(&destination, "rev-parse --verify 'HEAD@{0}'")?.trim(),
                        commit_id.to_string(),
                        "Git can resolve the newly initialized HEAD reflog"
                    );
                }
                assert_eq!(
                    std::fs::read(source.git_dir().join("logs/HEAD"))?,
                    source_log,
                    "the source HEAD reflog is unchanged"
                );
                assert_eq!(
                    std::fs::read(source.common_dir().join("logs/refs/heads/topic"))?,
                    branch_log,
                    "initializing HEAD does not update the branch reflog"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn bisect_and_rebase_reserve_branches_in_main_and_linked_worktrees() -> crate::Result {
        for operation in ["bisect", "rebase-merge", "rebase-apply", "rebase-interactive"] {
            for occupied in ["main", "linked"] {
                let fixture = gix_testtools::scripted_fixture_writable_with_args(
                    "make_worktree_reserved_branches.sh",
                    [operation, occupied],
                    gix_testtools::Creation::Execute,
                )?;
                let occupied_path = fixture.path().join(occupied);
                let branch_name = if occupied == "main" { "main" } else { "topic" };
                let name = branch(branch_name);
                let occupied_repo = gix::open_opts(&occupied_path, crate::restricted())?;
                assert!(
                    occupied_repo.head()?.is_detached(),
                    "{operation} temporarily detaches HEAD"
                );

                for caller in ["main", "linked"] {
                    let mut repo = gix::open_opts(fixture.path().join(caller), crate::restricted())?;
                    let destination = fixture.path().join("rejected");
                    let git_error = gix_testtools::git(
                        repo.workdir().expect("fixture has a checkout"),
                        &format!("worktree add ../rejected {branch_name}"),
                    )
                    .expect_err("Git reserves the original branch during the operation");
                    assert!(
                        git_error.to_string().contains("already"),
                        "Git rejects branch occupancy: {git_error}"
                    );
                    let err = repo
                        .add_worktree(
                            &destination,
                            gix::worktree::add::Head::Attached(name.clone()),
                            gix::progress::Discard,
                            &AtomicBool::default(),
                        )
                        .expect_err("the operation's original branch remains reserved");
                    match err {
                        gix::worktree::add::Error::CheckedOut {
                            name: actual,
                            worktree_dirs,
                        } => {
                            assert_eq!(actual, name);
                            assert_eq!(worktree_dirs.len(), 1, "each occupied worktree is reported once");
                            assert_eq!(
                                worktree_dirs[0].canonicalize()?,
                                occupied_path.canonicalize()?,
                                "the occupied directory is the same even when Windows uses a short path"
                            );
                        }
                        err => panic!("{operation} in {occupied}, called from {caller}: {err:?}"),
                    }
                    assert!(
                        !destination.exists(),
                        "occupancy is checked before creating the destination"
                    );
                    assert!(
                        matches!(
                            repo.delete_local_branches([name.clone()]),
                            Err(gix::repository::branch::delete::Error::CheckedOut { .. })
                        ),
                        "the shared occupancy check protects branch deletion as well"
                    );
                }

                let repo = gix::open_opts(fixture.path().join("main"), crate::restricted())?;
                repo.add_worktree(
                    fixture.path().join("allowed"),
                    gix::worktree::add::Head::Attached(branch("available")),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )?;
            }
        }
        Ok(())
    }

    #[test]
    fn mailbox_application_does_not_reserve_a_rebase_branch() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        std::fs::write(repo.git_dir().join("HEAD"), format!("{}\n", repo.head_id()?))?;
        let state = repo.git_dir().join("rebase-apply");
        std::fs::create_dir(&state)?;
        std::fs::write(state.join("applying"), b"")?;
        std::fs::write(state.join("head-name"), b"refs/heads/main\n")?;

        repo.add_worktree(
            destinations.path().join("allowed"),
            gix::worktree::add::Head::Attached(branch("main")),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        Ok(())
    }

    #[test]
    fn worktree_config_is_inherited_before_checkout() -> crate::Result {
        // MSYS sed's text mode would discard CRLF produced before smudge filtering.
        let sed = if cfg!(windows) { "sed -b" } else { "sed" };
        let (mut source, _fixture) = crate::basic_rw_repo()?;
        let work_dir = source.workdir().expect("non-bare fixture");
        std::fs::write(work_dir.join(".gitattributes"), b"filtered filter=inherit\n")?;
        std::fs::write(work_dir.join("filtered"), b"hello\n")?;
        gix_testtools::git(work_dir, "add .gitattributes filtered")?;
        gix_testtools::git(work_dir, "commit -m 'record filter attributes'")?;
        let mut config = source.config_file_mut(source.common_dir().join("config"))?;
        config.set_raw_value("extensions.worktreeConfig", "true")?;
        config.set_raw_value("core.autocrlf", "false")?;
        config.commit()?;

        for (name, autocrlf, expected_eol, expected_filter) in [
            ("main", "true", b"hello\r\n".as_slice(), b"main\r\n".as_slice()),
            ("linked", "false", b"hello\n".as_slice(), b"linked\n".as_slice()),
        ] {
            let work_dir = source.workdir().expect("source checkout").to_owned();
            let config_path = source.git_dir().join("config.worktree");
            let mut config = source.config_file_mut(&config_path)?;
            config.set_raw_value("core.autocrlf", autocrlf)?;
            config.set_raw_value("core.bare", "false")?;
            config.set_raw_value("core.worktree", to_unix_separators_on_windows(into_bstr(&work_dir)))?;
            config.set_raw_value("filter.inherit.smudge", format!("{sed} s/hello/{name}/"))?;
            config.set_raw_value("filter.inherit.clean", format!("{sed} s/{name}/hello/"))?;
            config.set_raw_value("filter.inherit.required", "true")?;
            config.commit()?;
            source.reload()?;
            let original_config = std::fs::read(&config_path)?;

            gix_testtools::git(&work_dir, "worktree add --detach git-created HEAD")?;
            let (created, _) = source.add_worktree(
                work_dir.join("gix-created"),
                gix::worktree::add::Head::Detached(source.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )?;
            for (file, expected) in [("this", expected_eol), ("filtered", expected_filter)] {
                let git_contents = std::fs::read(work_dir.join("git-created").join(file))?;
                assert_eq!(git_contents, expected, "Git applies {name}'s worktree-local settings");
                assert_eq!(
                    std::fs::read(created.workdir().expect("new checkout").join(file))?,
                    git_contents,
                    "checkout inherits {name}'s line endings and filter commands before writing {file}"
                );
            }
            let copied = gix_config::File::from_path_no_includes(
                created.git_dir().join("config.worktree"),
                gix_config::Source::Worktree,
            )?;
            assert!(
                copied.raw_value("core.worktree").is_err(),
                "the source checkout path is excluded"
            );
            assert_eq!(
                copied.boolean("core.bare")?,
                Some(false),
                "an explicit non-bare setting is retained"
            );
            assert_eq!(
                std::fs::read(&config_path)?,
                original_config,
                "copying configuration leaves the source file unchanged"
            );
            assert_eq!(
                gix_testtools::git(created.workdir().expect("new checkout"), "status --porcelain")?,
                "",
                "Git agrees with the checkout and index after applying inherited filters"
            );
            source = created;
        }
        Ok(())
    }

    #[test]
    fn common_attributes_are_applied_during_checkout() -> crate::Result {
        // Preserve the checkout's line endings when the filter runs under MSYS.
        let sed = if cfg!(windows) { "sed -b" } else { "sed" };
        let (mut source, _fixture) = crate::basic_rw_repo()?;
        let mut config = source.config_file_mut(source.common_dir().join("config"))?;
        config.set_raw_value("core.autocrlf", "false")?;
        config.set_raw_value("filter.shared.smudge", format!("{sed} s/hello/shared/"))?;
        config.set_raw_value("filter.shared.clean", format!("{sed} s/shared/hello/"))?;
        config.set_raw_value("filter.shared.required", "true")?;
        config.commit()?;
        source.reload()?;
        std::fs::write(
            source.common_dir().join("info/attributes"),
            b"this text eol=crlf filter=shared\n",
        )?;

        for caller in ["main", "linked"] {
            let work_dir = source.workdir().expect("source checkout").to_owned();
            gix_testtools::git(&work_dir, "worktree add --detach git-created HEAD")?;
            let (created, _) = source.add_worktree(
                work_dir.join("gix-created"),
                gix::worktree::add::Head::Detached(source.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )?;
            let git_contents = std::fs::read(work_dir.join("git-created/this"))?;
            assert_eq!(
                git_contents, b"shared\r\n",
                "Git applies the common attributes for CRLF conversion and smudge filtering"
            );
            assert_eq!(
                std::fs::read(created.workdir().expect("new checkout").join("this"))?,
                git_contents,
                "checkout from the {caller} worktree uses the common info/attributes like Git"
            );
            assert_eq!(
                gix_testtools::git(created.workdir().expect("new checkout"), "status --porcelain")?,
                "",
                "Git agrees with the checkout and index after applying the common attributes"
            );
            source = created;
        }
        Ok(())
    }

    #[test]
    fn worktree_config_is_optional_but_copy_errors_roll_back() -> crate::Result {
        let (mut repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let source_config = repo.git_dir().join("config.worktree");
        std::fs::write(&source_config, b"[core]\n\tautocrlf = true\n")?;
        let commit_id = repo.head_id()?.detach();
        let (disabled, _) = repo.add_worktree(
            destinations.path().join("disabled"),
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert!(
            !disabled.git_dir().join("config.worktree").exists(),
            "a dormant config.worktree file is not copied"
        );

        let mut config = repo.config_file_mut(repo.common_dir().join("config"))?;
        config.set_raw_value("extensions.worktreeConfig", "true")?;
        config.commit()?;
        std::fs::remove_file(source_config)?;
        repo.reload()?;
        let (linked, _) = repo.add_worktree(
            destinations.path().join("missing"),
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert!(
            !linked.git_dir().join("config.worktree").exists(),
            "an enabled extension does not require a worktree configuration file"
        );

        std::fs::write(linked.git_dir().join("config.worktree"), b"[invalid\n")?;
        let destination = destinations.path().join("rejected");
        let err = linked
            .add_worktree(
                &destination,
                gix::worktree::add::Head::Detached(commit_id),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("invalid source configuration must not be silently dropped");
        assert!(
            matches!(err, gix::worktree::add::Error::ReadWorktreeConfig(_)),
            "parse failures identify the source worktree configuration"
        );
        assert!(
            !destination.exists(),
            "a copy failure removes the new checkout directory"
        );
        assert_eq!(
            repo.worktrees()?.len(),
            2,
            "a copy failure removes the private Git directory"
        );
        Ok(())
    }

    #[test]
    fn relative_links_survive_moving_the_repository_and_worktrees_together() -> crate::Result {
        use std::fs;

        let (repo, _fixture) = crate::basic_rw_repo()?;
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let group = tmp.path().join("group");
        fs::create_dir(&group)?;
        let main_path = repo.workdir().expect("non-bare fixture").to_owned();
        drop(repo);
        fs::rename(main_path, group.join("main"))?;
        let mut repo = gix::open_opts(group.join("main"), crate::restricted())?;
        let mut config = repo.config_file_mut(repo.common_dir().join("config"))?;
        config.set_raw_value("worktree.useRelativePaths", "true")?;
        config.commit()?;
        repo.reload()?;
        let commit_id = repo.head_id()?.detach();
        let (linked, _) = repo.add_worktree(
            group.join("linked"),
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert_eq!(
            fs::read_to_string(linked.git_dir().join("gitdir"))?,
            "../../../../linked/.git\n",
            "the backlink is relative to the private Git directory"
        );
        assert_eq!(
            fs::read_to_string(group.join("linked/.git"))?,
            "gitdir: ../main/.git/worktrees/linked\n",
            "the forward link is relative to the worktree"
        );
        assert_eq!(
            linked.config_snapshot().integer("core.repositoryFormatVersion"),
            Some(1)
        );
        assert_eq!(
            linked.config_snapshot().boolean("extensions.relativeWorktrees"),
            Some(true)
        );

        // Adding from a linked worktree must update/use the shared configuration too.
        linked.add_worktree(
            group.join("another"),
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        drop(linked);
        drop(repo);
        let moved = tmp.path().join("moved");
        fs::rename(&group, &moved)?;
        let repo = gix::open_opts(moved.join("main"), crate::restricted())?;
        assert_eq!(
            repo.worktrees()?.len(),
            2,
            "both worktrees remain registered after moving"
        );
        for proxy in repo.worktrees()? {
            assert!(
                !proxy.is_prunable(),
                "relative backlinks still locate the moved worktrees"
            );
            let linked = proxy.into_repo()?;
            assert_eq!(linked.head_id()?.detach(), commit_id);
            assert_eq!(
                gix::open(linked.workdir().expect("linked checkout"))?
                    .head_id()?
                    .detach(),
                commit_id
            );
            // Container CI can have older Git too; always exercise gix above.
            if *gix_testtools::GIT_VERSION >= (2, 48, 0) {
                assert_eq!(
                    gix_testtools::git(linked.workdir().expect("linked checkout"), "status --porcelain")?,
                    "",
                    "Git can use the moved checkout and index without repair"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn relative_worktrees_extension_does_not_enable_relative_links_by_itself() -> crate::Result {
        let (mut repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let mut config = repo.config_file_mut(repo.common_dir().join("config"))?;
        config.set_raw_value("core.repositoryFormatVersion", "1")?;
        config.set_raw_value("extensions.relativeWorktrees", "true")?;
        config.commit()?;
        repo.reload()?;
        for (name, setting) in [("unset", None), ("false", Some("false"))] {
            if let Some(setting) = setting {
                repo.config_snapshot_mut()
                    .set_raw_value("worktree.useRelativePaths", setting)?;
            }
            let (linked, _) = repo.add_worktree(
                destinations.path().join(name),
                gix::worktree::add::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )?;
            assert_eq!(
                std::fs::read_to_string(linked.git_dir().join("gitdir"))?,
                format!(
                    "{}\n",
                    to_unix_separators_on_windows(into_bstr(linked.workdir().expect("linked checkout").join(".git")))
                ),
                "the extension records compatibility; the worktree setting controls new links"
            );
        }
        Ok(())
    }

    #[test]
    fn failed_relative_config_update_rolls_back_worktree_addition() -> crate::Result {
        for extension in [None, Some("futureExtension")] {
            let fixture = gix_testtools::tempfile::TempDir::new()?;
            // Only SHA-1 can need a v0 upgrade; lock failure also covers the selected fixture hash.
            gix_testtools::git(
                fixture.path(),
                if extension.is_some() {
                    "init --object-format=sha1"
                } else {
                    "init"
                },
            )?;
            gix_testtools::git(fixture.path(), "commit --allow-empty -m initial")?;
            let mut repo = gix::open_opts(fixture.path(), crate::restricted())?;
            let destinations = gix_testtools::tempfile::TempDir::new()?;
            let config_path = repo.common_dir().join("config");
            if let Some(extension) = extension {
                let mut config = repo.config_file_mut(&config_path)?;
                config.set_raw_value(&format!("extensions.{extension}"), "true")?;
                config.commit()?;
            }
            let before = std::fs::read(&config_path)?;
            repo.config_snapshot_mut()
                .set_raw_value("worktree.useRelativePaths", "true")?;
            repo.config_snapshot_mut()
                .set_raw_value("core.configLockTimeout", "0")?;
            let lock = extension
                .is_none()
                .then(|| repo.config_file_mut(&config_path))
                .transpose()?;
            let destination = destinations.path().join("rejected");
            let err = repo
                .add_worktree(
                    &destination,
                    gix::worktree::add::Head::Detached(repo.head_id()?.detach()),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("the shared config cannot safely be updated");
            assert!(
                matches!(
                    err,
                    gix::worktree::add::Error::Configure(_) | gix::worktree::add::Error::UnsupportedExtension { .. }
                ),
                "the config failure is reported: {err:?}"
            );
            assert!(!destination.exists(), "failed configuration removes the new worktree");
            assert!(
                repo.worktrees()?.is_empty(),
                "failed configuration removes the private Git directory"
            );
            assert_eq!(
                std::fs::read(&config_path)?,
                before,
                "the original config remains intact"
            );
            drop(lock);
        }
        Ok(())
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
        let (attached, attached_outcome) = repo.add_worktree(
            &attached_path,
            gix::worktree::add::Head::Attached(topic.clone()),
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
        let (detached, _) = repo.add_worktree(
            &detached_path,
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert_eq!(detached.head_name()?, None);
        assert_eq!(detached.head_id()?.detach(), commit_id);

        let listing = gix_testtools::git(repo.workdir().expect("non-bare fixture"), "worktree list --porcelain")?;
        assert!(
            listing.contains(to_unix_separators_on_windows(into_bstr(&attached_path)).to_str()?),
            "Git recognizes the attached worktree"
        );
        assert!(
            listing.contains(to_unix_separators_on_windows(into_bstr(&detached_path)).to_str()?),
            "Git recognizes the detached worktree"
        );
        Ok(())
    }

    #[test]
    fn linked_worktree_cannot_check_out_a_branch_occupied_in_any_worktree() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        assert_eq!(
            repo.worktrees_including_main()?.collect::<Result<Vec<_>, _>>()?,
            vec![repo.clone()],
            "the main repository is listed even without linked worktrees"
        );
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let topic = branch("topic");
        repo.reference(
            topic.clone(),
            repo.head_id()?,
            PreviousValue::MustNotExist,
            "create worktree test branch",
        )?;
        let (linked, _) = repo.add_worktree(
            destinations.path().join("linked"),
            gix::worktree::add::Head::Attached(topic),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let linked_dir = linked.workdir().expect("linked checkout");
        let destination = destinations.path().join("rejected");

        for (name, worktree_dir) in [
            ("main", repo.workdir().expect("non-bare fixture")),
            ("topic", linked_dir),
        ] {
            let git_err = gix_testtools::git(linked_dir, &format!("worktree add ../rejected {name}"))
                .expect_err("Git rejects a branch already checked out in either worktree");
            assert!(
                git_err.to_string().contains("already"),
                "Git reports that the branch is occupied: {git_err}"
            );
            let expected_dirs = vec![gix_path::realpath(worktree_dir)?];
            let name = branch(name);
            let err = linked
                .add_worktree(
                    &destination,
                    gix::worktree::add::Head::Attached(name.clone()),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("branches checked out in the main or linked worktree are protected");
            assert!(
                matches!(&err, gix::worktree::add::Error::CheckedOut { name: actual, worktree_dirs }
                    if actual == &name && worktree_dirs == &expected_dirs),
                "the occupied branch and its single checkout are reported: {err:?}"
            );
            let err = linked
                .clone()
                .delete_local_branches([name.clone()])
                .expect_err("branch deletion uses the same complete worktree listing");
            assert!(
                matches!(&err, gix::repository::branch::delete::Error::CheckedOut { name: actual, worktree_dirs }
                    if actual == &name && worktree_dirs == &expected_dirs),
                "branch deletion protects the same single checkout: {err:?}"
            );
            assert!(!destination.exists(), "branch validation leaves the destination absent");
        }
        assert_eq!(repo.worktrees()?.len(), 1, "no extra worktree was registered");
        Ok(())
    }

    #[test]
    fn lock_suffix_only_destination_names_match_git() -> crate::Result {
        let (source, _fixture) = crate::basic_rw_repo()?;
        let work_dir = source.workdir().expect("source checkout");
        gix_testtools::git(work_dir, "worktree add --detach git/.lock.lock HEAD")?;
        let git_work_dir = work_dir.join("git/.lock.lock");
        let git_contents = std::fs::read(git_work_dir.join("this"))?;
        let git_repo = gix::open_opts(&git_work_dir, crate::restricted())?;
        assert_eq!(
            git_repo.git_dir().file_name(),
            Some("-lock".as_ref()),
            "Git rewrites the leading dot before stripping repeated lock suffixes"
        );
        drop(git_repo);
        gix_testtools::git(work_dir, "worktree remove git/.lock.lock")?;

        let destination = work_dir.join("gix/.lock.lock");
        let (created, _) = source.add_worktree(
            &destination,
            gix::worktree::add::Head::Detached(source.head_id()?.detach()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        assert_eq!(
            created.git_dir().file_name(),
            Some("-lock".as_ref()),
            "the private Git directory uses the same non-empty ID as Git"
        );
        assert_eq!(
            std::fs::read(destination.join("this"))?,
            git_contents,
            "checkout succeeds at the requested destination"
        );
        assert_eq!(
            gix_testtools::git(&destination, "status --porcelain")?,
            "",
            "Git agrees with the checkout and index"
        );
        Ok(())
    }

    #[test]
    fn attached_worktrees_ignore_reference_namespaces_like_git() -> crate::Result {
        for (namespace, from_config) in [("foo", false), ("foo/bar", true)] {
            let (mut source, _fixture) = crate::basic_rw_repo()?;
            let work_dir = source.workdir().expect("source checkout").to_owned();
            let commit_id = source.head_id()?.detach();
            let namespaced_commit_id = source
                .head_commit()?
                .parent_ids()
                .next()
                .expect("the fixture has two commits with different trees")
                .detach();
            let topic = branch("topic");
            source.reference(topic.clone(), commit_id, PreviousValue::MustNotExist, "create topic")?;
            source.set_namespace(namespace)?;
            for name in ["main", "topic", "only-namespaced"] {
                source.reference(
                    branch(name),
                    namespaced_commit_id,
                    PreviousValue::MustNotExist,
                    "create namespaced branch",
                )?;
            }
            if from_config {
                let mut config = source.config_file_mut(source.common_dir().join("config"))?;
                config.set_raw_value("gitoxide.core.refsNamespace", namespace)?;
                config.commit()?;
                source.reload()?;
            }

            gix_testtools::git(
                &work_dir,
                &format!("--namespace={namespace} worktree add git-created topic"),
            )?;
            let git_work_dir = work_dir.join("git-created");
            let git_contents = std::fs::read(git_work_dir.join("this"))?;
            assert_eq!(
                gix_testtools::git(&git_work_dir, "rev-parse HEAD")?.trim(),
                commit_id.to_string(),
                "Git selects the default namespace even when the namespaced branch has another commit"
            );
            gix_testtools::git(&work_dir, "worktree remove git-created")?;

            let (created, _) = source.add_worktree(
                work_dir.join("gix-created"),
                gix::worktree::add::Head::Attached(topic.clone()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )?;
            assert_eq!(
                std::fs::read(created.workdir().expect("new checkout").join("this"))?,
                git_contents,
                "the checkout uses the same default-namespace tree as Git"
            );
            assert!(
                created.namespace().is_none(),
                "the returned repository uses its physical HEAD"
            );
            assert_eq!(created.head_id()?, commit_id, "HEAD agrees with the checked-out tree");
            assert_eq!(
                created.head_name()?,
                Some(topic.clone()),
                "HEAD stays on the default branch"
            );
            assert_eq!(
                gix_testtools::git(created.workdir().expect("new checkout"), "status --porcelain")?,
                "",
                "the new worktree has no staged or unstaged changes"
            );
            assert_eq!(
                source.find_reference(topic.as_ref())?.id(),
                namespaced_commit_id,
                "the source repository retains its namespace"
            );

            for name in ["main", "topic", "only-namespaced"] {
                gix_testtools::git(
                    &work_dir,
                    &format!("--namespace={namespace} worktree add rejected {name}"),
                )
                .expect_err("Git rejects occupied branches and branches that only exist in a namespace");
                let err = source
                    .add_worktree(
                        work_dir.join("rejected"),
                        gix::worktree::add::Head::Attached(branch(name)),
                        gix::progress::Discard,
                        &AtomicBool::default(),
                    )
                    .expect_err("validation uses the default namespace like Git");
                assert!(
                    match name {
                        "only-namespaced" => matches!(err, gix::worktree::add::Error::FindBranch(_)),
                        _ => matches!(err, gix::worktree::add::Error::CheckedOut { .. }),
                    },
                    "{name}: the default namespace determines whether a branch is missing or occupied: {err:?}"
                );
                assert!(
                    !work_dir.join("rejected").exists(),
                    "validation leaves the destination absent"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn separate_git_dir_preserves_main_worktree_and_its_occupied_branch() -> crate::Result {
        let fixture = gix_testtools::tempfile::TempDir::new()?;
        gix_testtools::git(
            fixture.path(),
            "init --initial-branch=main --separate-git-dir separate.git main",
        )?;
        let work_dir = gix_path::realpath(fixture.path().join("main"))?;
        gix_testtools::git(&work_dir, "commit --allow-empty -m initial")?;
        let mut repo = gix::open_opts(&work_dir, crate::restricted())?;
        let commit_id = repo.head_id()?.detach();
        let name = branch("main");
        let destination = fixture.path().join("rejected");

        for command in ["worktree add ../rejected main", "branch -D main"] {
            gix_testtools::git(&work_dir, command)
                .expect_err("Git protects the main worktree's branch with a separate Git directory");
        }
        let err = repo
            .add_worktree(
                &destination,
                gix::worktree::add::Head::Attached(name.clone()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("the main worktree's branch cannot be checked out twice");
        let expected_dirs = vec![work_dir.clone()];
        assert!(
            matches!(&err, gix::worktree::add::Error::CheckedOut { name: actual, worktree_dirs }
                if actual == &name && worktree_dirs == &expected_dirs),
            "the occupied branch reports its known main worktree: {err:?}"
        );
        let err = repo
            .delete_local_branches([name.clone()])
            .expect_err("the main worktree's checked-out branch cannot be deleted");
        assert!(
            matches!(&err, gix::repository::branch::delete::Error::CheckedOut { name: actual, worktree_dirs }
                if actual == &name && worktree_dirs == &expected_dirs),
            "branch deletion protects the same main worktree: {err:?}"
        );
        assert_eq!(repo.head_id()?, commit_id, "the checked-out branch is unchanged");
        assert!(!destination.exists(), "rejection leaves the destination absent");

        let repositories = repo.worktrees_including_main()?.collect::<Result<Vec<_>, _>>()?;
        assert_eq!(repositories.len(), 1, "only the main repository is registered");
        assert_eq!(
            repositories[0].workdir(),
            Some(work_dir.as_path()),
            "enumeration preserves the worktree known from its .git file"
        );
        assert_eq!(
            repo.main_repo()?.workdir(),
            Some(work_dir.as_path()),
            "requesting the already-open main repository preserves its worktree"
        );
        Ok(())
    }

    #[test]
    fn adds_a_worktree_from_a_bare_parent() -> crate::Result {
        let Some(fixture) = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
            "make_worktree_repo.sh",
            ["bare"],
            gix_testtools::Creation::CopyFromReadOnly,
            |version| version >= (2, 31, 0),
        )?
        else {
            return Ok(());
        };
        let mut repo = gix::open_opts(fixture.path().join("repo.git"), crate::restricted())?;
        assert!(repo.is_bare(), "the parent repository has no main worktree");
        let mut config = repo.config_file_mut(repo.common_dir().join("config"))?;
        config.set_raw_value("extensions.worktreeConfig", "true")?;
        // With worktree config enabled, the bare setting belongs only to the main repository's config.worktree.
        config.raw_values_mut("core.bare")?.delete_all();
        config.commit()?;
        let mut config = repo.config_file_mut(repo.git_dir().join("config.worktree"))?;
        config.set_raw_value("core.bare", "true")?;
        config.set_raw_value("core.autocrlf", "true")?;
        config.commit()?;
        repo.reload()?;
        gix_testtools::git(repo.git_dir(), "worktree add --detach ../git-added-from-bare HEAD")?;
        let destination = fixture.path().join("added-from-bare");
        let main = branch("main");

        let (worktree, _) = repo.add_worktree(
            &destination,
            gix::worktree::add::Head::Attached(main.clone()),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;

        assert_eq!(worktree.head_name()?, Some(main));
        assert_eq!(worktree.workdir(), Some(gix_path::realpath(&destination)?.as_path()));
        let copied = gix_config::File::from_path_no_includes(
            worktree.git_dir().join("config.worktree"),
            gix_config::Source::Worktree,
        )?;
        assert_eq!(copied.boolean("core.bare")?, None, "a true bare setting is excluded");
        let git_contents = std::fs::read(fixture.path().join("git-added-from-bare/a"))?;
        assert_eq!(
            git_contents, b"hello\r\n",
            "Git inherits the bare source's line-ending settings"
        );
        assert_eq!(
            std::fs::read(destination.join("a"))?,
            git_contents,
            "checkout from a bare parent applies the same inherited settings as Git"
        );
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
            .add_worktree(
                &destination,
                gix::worktree::add::Head::Attached(main.clone()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("the main branch is already checked out");
        assert!(
            matches!(err, gix::worktree::add::Error::CheckedOut { name, .. } if name == main),
            "the checked-out branch is identified"
        );
        assert!(!destination.exists(), "validation happens before creating files");

        let interrupted = AtomicBool::new(true);
        let err = repo
            .add_worktree(
                &destination,
                gix::worktree::add::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &interrupted,
            )
            .expect_err("an already-interrupted operation does no work");
        assert!(matches!(err, gix::worktree::add::Error::Interrupted));
        assert!(!destination.exists(), "interruption leaves no destination behind");

        std::fs::create_dir(&destination)?;
        std::fs::write(destination.join("keep"), b"user data")?;
        let err = repo
            .add_worktree(
                &destination,
                gix::worktree::add::Head::Detached(repo.head_id()?.detach()),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("a non-empty destination is rejected");
        assert!(matches!(err, gix::worktree::add::Error::Prepare(_)));
        assert_eq!(
            std::fs::read(destination.join("keep"))?,
            b"user data",
            "a failed addition preserves existing destination contents"
        );
        Ok(())
    }

    #[test]
    fn registered_destinations_are_rejected_even_when_missing_or_empty() -> crate::Result {
        let (repo, _fixture) = crate::basic_rw_repo()?;
        let destinations = gix_testtools::tempfile::TempDir::new()?;
        let destination = destinations.path().join("registered");
        let commit_id = repo.head_id()?.detach();
        repo.add_worktree(
            &destination,
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;

        std::fs::remove_dir_all(&destination)?;
        for exists in [false, true] {
            if exists {
                std::fs::create_dir(&destination)?;
            }
            let err = repo
                .add_worktree(
                    &destination,
                    gix::worktree::add::Head::Detached(commit_id),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("registered destinations cannot be reused");
            assert!(
                matches!(err, gix::worktree::add::Error::DestinationRegistered { destination: actual } if actual == destination),
                "the registered destination is identified"
            );
        }

        let case_variant = destination.with_file_name("REGISTERED");
        if case_variant.exists() {
            let err = repo
                .add_worktree(
                    &case_variant,
                    gix::worktree::add::Head::Detached(commit_id),
                    gix::progress::Discard,
                    &AtomicBool::default(),
                )
                .expect_err("filesystem-equivalent casing cannot bypass registration");
            assert!(
                matches!(err, gix::worktree::add::Error::DestinationRegistered { destination } if destination == case_variant),
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
        repo.add_worktree(
            &destination,
            gix::worktree::add::Head::Detached(commit_id),
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        std::fs::remove_dir_all(&destination)?;

        let alias = linked_parent.join("registered");
        let err = repo
            .add_worktree(
                &alias,
                gix::worktree::add::Head::Detached(commit_id),
                gix::progress::Discard,
                &AtomicBool::default(),
            )
            .expect_err("registered destinations are compared by their real paths");
        assert!(
            matches!(err, gix::worktree::add::Error::DestinationRegistered { destination } if destination == alias),
            "the alias supplied by the caller is identified"
        );
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

    let linked_repo = actual
        .first()
        .expect("the fixture has linked worktrees")
        .clone()
        .into_repo()
        .expect("the first linked checkout exists");
    for repo in [&main_repo, &linked_repo] {
        let repositories = repo
            .worktrees_including_main()
            .expect("the fixture's worktrees can be listed")
            .collect::<Result<Vec<_>, _>>()
            .expect("repositories open even for removed checkouts");
        let (main, linked) = repositories.split_first().expect("the main repository is included");
        assert_eq!(main, &main_repo, "the main repository comes first, even when bare");
        assert_eq!(
            linked.iter().map(gix::Repository::git_dir).collect::<Vec<_>>(),
            actual.iter().map(gix::worktree::Proxy::git_dir).collect::<Vec<_>>(),
            "each linked worktree is listed once in private Git directory order, from either caller"
        );
    }

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
        assert_eq!(
            actual.is_prunable(),
            expected.prunable.is_some(),
            "prunability matches `git worktree list --porcelain`"
        );
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
        assert_eq!(
            proxy_by_id.git_dir(),
            actual.git_dir(),
            "The git directories are the same"
        );
        assert_eq!(
            gix_path::realpath(proxy_by_id.git_dir()).ok(),
            gix_path::realpath(actual.git_dir()).ok(),
            "the git directories are effectively the same"
        );
    }
}
