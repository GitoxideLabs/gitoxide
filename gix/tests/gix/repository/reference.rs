use gix_testtools::TestResult;

mod set_namespace {
    use gix::refs::transaction::PreviousValue;
    use gix_testtools::tempfile;
    use gix_testtools::{Result, TestResult};

    fn easy_repo_rw() -> Result<(gix::Repository, tempfile::TempDir)> {
        crate::repo_rw("make_references_repo.sh")
    }

    #[test]
    fn affects_edits_and_iteration() -> TestResult {
        let (mut repo, _keep) = easy_repo_rw()?;
        assert_eq!(
            repo.references()?.all()?.count(),
            17,
            "there are plenty of references in the default namespace"
        );
        assert!(repo.namespace().is_none(), "no namespace is set initially");
        assert!(repo.set_namespace("foo")?.is_none(), "there is no previous namespace");

        assert_eq!(
            repo.references()?.all()?.filter_map(std::result::Result::ok).count(),
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
            repo.references()?
                .all()?
                .filter_map(std::result::Result::ok)
                .collect::<Vec<_>>(),
            vec!["refs/heads/new-branch", "refs/tags/new-tag"],
            "namespaced references appear like normal ones"
        );

        assert_eq!(
            repo.references()?
                .prefixed("refs/tags/")?
                .filter_map(std::result::Result::ok)
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
fn reference_snapshot_comparison_matches_plumbing() -> gix_error::TestResult {
    use std::hash::{BuildHasher, RandomState};

    let repo = crate::basic_repo()?;
    let other_repo = crate::basic_repo()?;
    let head = repo.find_reference("HEAD")?;
    let other_head = other_repo.find_reference("HEAD")?;
    assert_eq!(head, other_head, "the repository is not part of snapshot equality");
    let raw = other_head.detach();
    assert_eq!(head, raw, "an attached snapshot equals its plumbing equivalent");
    assert_eq!(raw, head, "mixed equality is symmetric");

    let commit_id = repo.head_id()?.detach();
    let mut with_other_target = head.clone();
    with_other_target.inner_mut().target = gix::refs::Target::Object(commit_id);
    assert_ne!(
        head, with_other_target,
        "same-name snapshots can have different targets"
    );
    assert_eq!(with_other_target, "HEAD", "comparing to a name ignores the target");
    assert_ne!(with_other_target, raw, "mixed equality includes the target");

    let mut with_peeled = head.clone();
    with_peeled.inner_mut().peeled = Some(commit_id);
    assert_ne!(head, with_peeled, "snapshot equality includes the cached peeled ID");
    assert_eq!(
        head.cmp(&with_peeled),
        head.inner().cmp(with_peeled.inner()),
        "snapshot ordering matches the plumbing reference"
    );
    assert_ne!(raw, with_peeled, "mixed equality includes the cached peeled ID");
    assert_eq!(
        with_peeled.partial_cmp(&raw),
        Some(with_peeled.inner().cmp(&raw)),
        "mixed ordering matches plumbing ordering"
    );
    assert_eq!(
        raw.partial_cmp(&with_peeled),
        with_peeled.partial_cmp(&raw).map(std::cmp::Ordering::reverse),
        "mixed ordering reverses when the operands are swapped"
    );
    let hasher = RandomState::new();
    assert_eq!(
        hasher.hash_one(&head),
        hasher.hash_one(&raw),
        "snapshot hashing matches the plumbing reference"
    );
    Ok(())
}

#[test]
fn try_find_reference_with_existing_ref_as_path_prefix_returns_none() -> TestResult {
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

mod iter_references {
    use crate::util::hex_to_id;
    use gix_testtools::{Result, TestResult};

    fn repo() -> Result<gix::Repository> {
        crate::repo("make_references_repo.sh").map(|r| r.to_thread_local())
    }

    #[test]
    fn all() -> TestResult {
        let mut repo = repo()?;
        assert_eq!(
            repo.references()?
                .all()?
                .filter_map(std::result::Result::ok)
                .collect::<Vec<_>>(),
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
        let cached_buffers = repo.set_freelist(None).expect("the repository has a default freelist");
        assert_eq!(
            cached_buffers.len(),
            2,
            "dropping a collection bounds name buffer retention"
        );
        Ok(())
    }

    #[test]
    fn prefixed() -> TestResult {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed("refs/heads/")?
                .filter_map(std::result::Result::ok)
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
    fn prefixed_and_peeled() -> TestResult {
        let repo = repo()?;
        assert_eq!(
            repo.references()?
                .prefixed(b"refs/heads/")?
                .peeled()?
                .filter_map(std::result::Result::ok)
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
    fn tags() -> TestResult {
        let repo = repo()?;
        let actual = repo
            .references()?
            .tags()?
            .filter_map(std::result::Result::ok)
            .max_by_key(|tag| tag.name().shorten().to_owned())
            .ok_or(std::io::Error::other("latest tag not found"))?;
        assert_eq!(actual, "refs/tags/t1");
        Ok(())
    }
}

mod head {

    use gix_error::TestResult;

    use crate::util::hex_to_id;

    #[test]
    fn symbolic() -> TestResult {
        let repo = crate::basic_repo()?;
        let mut head = repo.head()?;
        let commit_id = hex_to_id("3189cd3cb0af8586c39a838aa3e54fd72a872a41");
        assert!(std::ptr::eq(head.repo(), &repo), "HEAD retains its repository");
        assert_eq!(head.id().expect("born"), commit_id);
        assert_eq!(head.referent_name().expect("born"), "refs/heads/main");
        assert!(!head.is_detached());
        assert_eq!(head.try_peel_to_id()?.expect("born"), commit_id);
        let referent = head.try_into_referent().expect("symbolic head");
        assert_eq!(referent, "refs/heads/main");
        assert!(std::ptr::eq(referent.repo(), &repo));

        let snapshot: gix::HeadOwned = crate::basic_repo()?.head()?.detach_repo();
        assert_eq!(snapshot.referent_name().expect("born"), "refs/heads/main");
        assert_eq!(snapshot.id(), Some(commit_id), "the symbolic target ID is retained");
        assert!(!snapshot.is_detached() && !snapshot.is_unborn());
        let referent = snapshot.attach(&repo).try_into_referent().expect("symbolic head");
        assert_eq!(referent, "refs/heads/main");
        assert!(std::ptr::eq(referent.repo(), &repo));
        Ok(())
    }

    #[test]
    fn symbolic_snapshot_prefers_cached_peeled_id() -> TestResult {
        let repo = crate::named_repo("make_head_with_packed_tag.sh")?;
        let snapshot = repo.head()?.detach_repo();
        let id = snapshot.id().expect("symbolic HEAD has a target");
        assert_eq!(
            repo.find_commit(id)?.id,
            repo.head_commit()?.id,
            "HEAD names an annotated tag, so the ref stores the tag object's ID. The snapshot has no repository \
             to peel the tag itself, so it must use the peeled commit ID cached in packed-refs."
        );
        Ok(())
    }

    #[test]
    fn detached() -> TestResult {
        let repo = crate::named_subrepo_opts("make_head_repos.sh", "tag-detached", gix::open::Options::isolated())?;
        let mut head = repo.head()?;
        assert!(head.is_detached(), "HEAD points to the tag object directly");
        assert!(head.referent_name().is_none(), "a direct HEAD has no referent name");

        let tag_id = head.id().expect("a direct HEAD has a target").detach();
        let commit_id = head.try_peel_to_id()?.expect("the tag peels to a commit").detach();
        assert_ne!(tag_id, commit_id, "peeling follows the tag to a different object");
        let snapshot = head.detach_repo();
        assert!(
            snapshot.is_detached() && !snapshot.is_unborn(),
            "the Git state is unchanged"
        );
        assert!(snapshot.referent_name().is_none(), "the snapshot has no referent name");
        assert_eq!(snapshot.id(), Some(commit_id), "the cached peeled ID is retained");
        assert!(
            matches!(snapshot, gix::HeadOwned::Detached { target, .. } if target == tag_id),
            "the original tag ID is retained too"
        );
        let head = snapshot.attach(&repo);
        assert!(
            head.is_detached(),
            "reattaching the Rust value does not change the Git state"
        );
        assert_eq!(
            head.id().map(gix::Id::detach),
            Some(commit_id),
            "reattaching retains the peeled ID"
        );
        Ok(())
    }

    #[test]
    fn unborn() -> TestResult {
        let repo = crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?;
        let head = repo.head()?;
        let name = head.referent_name().expect("an unborn HEAD names a branch").to_owned();
        let snapshot = head.detach_repo();
        assert!(snapshot.is_unborn() && !snapshot.is_detached());
        assert!(snapshot.id().is_none(), "an unborn HEAD has no object ID");
        assert_eq!(snapshot.referent_name(), Some(name.as_ref()));
        let head = snapshot.attach(&repo);
        assert!(head.is_unborn());
        assert_eq!(head.referent_name(), Some(name.as_ref()));
        Ok(())
    }
}
