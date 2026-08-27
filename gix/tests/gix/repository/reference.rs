mod set_namespace {
    use gix::refs::transaction::PreviousValue;
    use gix_testtools::tempfile;

    fn easy_repo_rw() -> crate::Result<(gix::Repository, tempfile::TempDir)> {
        crate::repo_rw("make_references_repo.sh")
    }

    #[test]
    fn affects_edits_and_iteration() -> crate::Result {
        let (mut repo, _keep) = easy_repo_rw()?;
        assert_eq!(
            repo.references()?.all()?.count(),
            17,
            "there are plenty of references in the default namespace"
        );
        assert!(repo.namespace().is_none(), "no namespace is set initially");
        assert!(repo.set_namespace("foo")?.is_none(), "there is no previous namespace");

        assert_eq!(
            repo.references()?.all()?.filter_map(Result::ok).count(),
            0,
            "no references are in the namespace yet"
        );

        repo.tag_reference("new-tag", repo.object_hash().empty_tree(), PreviousValue::MustNotExist)?;

        repo.reference(
            "refs/heads/new-branch",
            repo.object_hash().empty_tree(),
            PreviousValue::MustNotExist,
            "message",
        )?;

        assert_eq!(
            repo.references()?.all()?.filter_map(Result::ok).collect::<Vec<_>>(),
            vec!["refs/heads/new-branch", "refs/tags/new-tag"],
            "namespaced references appear like normal ones"
        );

        assert_eq!(
            repo.references()?
                .prefixed("refs/tags/")?
                .filter_map(Result::ok)
                .collect::<Vec<_>>(),
            vec!["refs/tags/new-tag"],
            "namespaced references appear like normal ones"
        );
        let fully_qualified_tag_name = "refs/tags/new-tag";
        assert_eq!(
            repo.find_reference(fully_qualified_tag_name)?,
            fully_qualified_tag_name,
            "fully qualified (yet namespaced) names work"
        );
        assert_eq!(
            repo.find_reference("new-tag")?,
            fully_qualified_tag_name,
            "namespaces are transparent"
        );

        let previous_ns = repo.clear_namespace().expect("namespace set");
        assert_eq!(previous_ns, "refs/namespaces/foo/");
        assert!(repo.clear_namespace().is_none(), "it doesn't invent namespaces");

        assert_eq!(
            repo.references()?.all()?.count(),
            19,
            "it lists all references, also the ones in namespaces"
        );
        Ok(())
    }
}

#[test]
fn try_find_reference_with_existing_ref_as_path_prefix_returns_none() -> crate::Result {
    let (repo, _tmp) = crate::repo_rw("make_references_repo.sh")?;
    std::fs::create_dir_all(repo.git_dir().join("refs/heads"))?;
    std::fs::write(
        repo.git_dir().join("refs/heads/A"),
        repo.head_id()?.to_hex().to_string(),
    )?;

    assert!(
        repo.try_find_reference("refs/heads/A/new")?.is_none(),
        "a ref whose path prefix is an existing ref does not exist"
    );
    Ok(())
}

mod maintenance {
    #[test]
    fn files_storage_can_be_verified_and_default_optimization_is_nondestructive() -> crate::Result {
        let (repo, _keep) = crate::basic_rw_repo()?;
        let before = repo
            .references()?
            .all()?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(gix::Reference::detach)
            .collect::<Vec<_>>();
        repo.verify_references()?;
        repo.optimize_references(Default::default())?;
        let after = repo
            .references()?
            .all()?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(gix::Reference::detach)
            .collect::<Vec<_>>();
        assert_eq!(after, before, "files-backed behavior remains unchanged");

        let err = repo
            .optimize_references(gix::reference::maintenance::Options {
                expire_reflogs_before: Some(u64::MAX),
                ..Default::default()
            })
            .expect_err("files-backed reflog expiry must not be silently ignored");
        assert_eq!(
            err.to_string(),
            "Could not expire reference logs",
            "the top-level error identifies the unsupported operation"
        );
        assert!(
            std::iter::successors(std::error::Error::source(&err), |source| source.source())
                .any(|source| source.to_string().contains("not supported by the files backend")),
            "the concrete files-backend limitation remains available through the error source chain"
        );
        Ok(())
    }

    #[test]
    fn repository_maintenance_compacts_reftable_storage() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let repo = gix::ThreadSafeRepository::init(
            tmp.path(),
            gix::create::Kind::WithWorktree,
            gix::create::Options {
                reference_storage: gix::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .to_thread_local();
        let empty_tree_id = repo.object_hash().empty_tree();
        repo.reference(
            "refs/heads/topic",
            empty_tree_id,
            gix::refs::transaction::PreviousValue::MustNotExist,
            "create",
        )?;
        repo.reference(
            "refs/heads/topic",
            empty_tree_id,
            gix::refs::transaction::PreviousValue::Any,
            "same value",
        )?;
        let list_path = repo.git_dir().join("reftable/tables.list");
        assert!(
            std::fs::read_to_string(&list_path)?.lines().count() > 1,
            "two reference updates produce multiple immutable stack members"
        );

        repo.verify_references()?;
        repo.optimize_references(gix::reference::maintenance::Options {
            expire_reflogs_before: Some(u64::MAX),
            ..Default::default()
        })?;
        assert_eq!(
            std::fs::read_to_string(&list_path)?.lines().count(),
            1,
            "repository maintenance compacts the stack to one member"
        );
        repo.verify_references()?;
        Ok(())
    }

    #[test]
    fn reftable_maintenance_uses_the_aggregate_lock_timeout() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut repo = gix::ThreadSafeRepository::init(
            tmp.path(),
            gix::create::Kind::Bare,
            gix::create::Options {
                reference_storage: gix::create::ReferenceStorage::Reftable,
                ..Default::default()
            },
        )?
        .to_thread_local();
        let mut config = repo.config_snapshot_mut();
        config.append_config(
            ["core.filesRefLockTimeout=0", "core.packedRefsTimeout=1000"],
            gix_config::Source::Api,
        )?;
        config.commit()?;

        let list_path = repo.git_dir().join("reftable/tables.list");
        let lock = gix_lock::File::acquire_to_update_resource(&list_path, gix_lock::acquire::Fail::Immediately, None)?;
        let release_lock = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            drop(lock);
        });

        repo.optimize_references(Default::default())?;
        release_lock.join().expect("the publication-lock holder exits normally");
        Ok(())
    }
}

mod reftable_bare_interop {
    fn git_ok(cwd: &std::path::Path, args: &[&str]) -> crate::Result<std::process::Output> {
        gix_testtools::isolated_git_output_checked(Some(cwd), args)
    }

    fn enabled_hashes() -> Vec<gix::hash::Kind> {
        #[cfg(feature = "sha256")]
        {
            vec![gix::hash::Kind::Sha1, gix::hash::Kind::Sha256]
        }
        #[cfg(not(feature = "sha256"))]
        {
            vec![gix::hash::Kind::Sha1]
        }
    }

    #[test]
    #[serial_test::serial]
    fn git_and_gix_created_bare_repositories_are_bidirectionally_writable() -> crate::Result {
        if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 45, 0) {
            return Ok(());
        }

        let temp = gix_testtools::tempfile::TempDir::new()?;
        for object_hash in enabled_hashes() {
            let git_created_path = temp.path().join(format!("git-{object_hash}.git"));
            let init = gix_testtools::isolated_git_command(None)
                .current_dir(temp.path())
                .args(["init", "--quiet", "--bare", "--ref-format=reftable"])
                .arg(format!("--object-format={object_hash}"))
                .arg(&git_created_path)
                .output()?;
            assert!(
                init.status.success(),
                "Git initializes a bare {object_hash} reftable repository: {}",
                String::from_utf8_lossy(&init.stderr)
            );
            let git_created = gix::open_opts(&git_created_path, gix::open::Options::isolated())?;
            let git_created_tree_id = git_created.write_object(gix::objs::Tree::empty())?;
            git_created.reference(
                "refs/tags/from-gix",
                git_created_tree_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "write through gix",
            )?;
            let resolved = git_ok(&git_created_path, &["rev-parse", "refs/tags/from-gix"])?;
            assert_eq!(
                String::from_utf8(resolved.stdout)?.trim(),
                git_created_tree_id.to_string(),
                "Git reads a reference written by gix in a Git-created bare {object_hash} repository"
            );

            let gix_created_path = temp.path().join(format!("gix-{object_hash}.git"));
            let gix_created = gix::ThreadSafeRepository::init(
                &gix_created_path,
                gix::create::Kind::Bare,
                gix::create::Options {
                    object_hash: Some(object_hash),
                    reference_storage: gix::create::ReferenceStorage::Reftable,
                    ..Default::default()
                },
            )?
            .to_thread_local();
            let gix_created_tree_id = gix_created.write_object(gix::objs::Tree::empty())?;
            gix_created.reference(
                "refs/tags/from-gix",
                gix_created_tree_id,
                gix::refs::transaction::PreviousValue::MustNotExist,
                "write through gix",
            )?;
            let object_id = gix_created_tree_id.to_string();
            git_ok(&gix_created_path, &["update-ref", "refs/tags/from-git", &object_id])?;
            assert_eq!(
                gix_created.find_reference("refs/tags/from-git")?.target().try_id(),
                Some(gix_created_tree_id.as_ref()),
                "gix reads a reference written by Git in a gix-created bare {object_hash} repository"
            );
            let resolved = git_ok(&gix_created_path, &["rev-parse", "refs/tags/from-gix"])?;
            assert_eq!(
                String::from_utf8(resolved.stdout)?.trim(),
                object_id,
                "Git can continue reading gix-written refs after updating a gix-created bare {object_hash} repository"
            );
        }
        Ok(())
    }
}

mod iter_references {
    use crate::util::hex_to_id;

    fn repo() -> crate::Result<gix::Repository> {
        crate::repo("make_references_repo.sh").map(|r| r.to_thread_local())
    }

    #[test]
    fn all() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?.all()?.filter_map(Result::ok).collect::<Vec<_>>(),
            vec![
                "refs/d1",
                "refs/heads/d1",
                "refs/heads/dt1",
                "refs/heads/main",
                "refs/heads/multi-link-target1",
                "refs/loop-a",
                "refs/loop-b",
                "refs/multi-link",
                "refs/remotes/origin/HEAD",
                "refs/remotes/origin/main",
                "refs/remotes/origin/multi-link-target3",
                "refs/tags/dt1",
                "refs/tags/dt2",
                "refs/tags/dt3",
                "refs/tags/multi-link-target2",
                "refs/tags/t1"
            ]
        );
        Ok(())
    }

    #[test]
    fn prefixed() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed("refs/heads/")?
                .filter_map(Result::ok)
                .map(|r| (
                    r.name().as_bstr().to_string(),
                    r.target().try_id().map(ToOwned::to_owned)
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "refs/heads/d1".to_string(),
                    Some(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"))
                ),
                (
                    "refs/heads/dt1".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/heads/main".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                ("refs/heads/multi-link-target1".into(), None),
            ]
        );
        Ok(())
    }

    #[test]
    fn prefixed_and_peeled() -> crate::Result {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed(b"refs/heads/")?
                .peeled()?
                .filter_map(Result::ok)
                .map(|r| (
                    r.name().as_bstr().to_string(),
                    r.target().try_id().map(ToOwned::to_owned)
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "refs/heads/d1".to_string(),
                    Some(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"))
                ),
                (
                    "refs/heads/dt1".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/heads/main".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
                (
                    "refs/remotes/origin/multi-link-target3".into(),
                    hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03").into()
                ),
            ]
        );
        Ok(())
    }

    /// Regression test for https://github.com/GitoxideLabs/gitoxide/issues/2103
    /// This only ensures we can return a reference, not that the code below is correct
    #[test]
    fn tags() -> crate::Result {
        let repo = repo()?;
        let actual = repo
            .references()?
            .tags()?
            .filter_map(Result::ok)
            .max_by_key(|tag| tag.name().shorten().to_owned())
            .ok_or(std::io::Error::other("latest tag not found"))?;
        assert_eq!(actual, "refs/tags/t1");
        Ok(())
    }
}

mod head {

    use gix_ref::transaction::PreviousValue;

    use crate::util::hex_to_id;

    #[test]
    fn symbolic() -> crate::Result {
        let repo = crate::basic_repo()?;
        let head = repo.head()?;
        match &head.kind {
            gix::head::Kind::Symbolic(r) => {
                assert_eq!(
                    r.target.try_id().map(ToOwned::to_owned),
                    Some(hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41"))
                );
            }
            _ => panic!("unexpected head kind"),
        }
        assert_eq!(head.referent_name().expect("born"), "refs/heads/main");
        assert!(!head.is_detached());
        Ok(())
    }

    #[test]
    fn detached() -> crate::Result {
        let (repo, _keep) = crate::basic_rw_repo()?;
        repo.reference(
            "HEAD",
            hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41"),
            PreviousValue::Any,
            "",
        )?;

        let head = repo.head()?;
        assert!(head.is_detached(), "head is detached");
        assert!(head.referent_name().is_none());
        Ok(())
    }
}
