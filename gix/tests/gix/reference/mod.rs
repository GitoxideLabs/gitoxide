use gix::remote::Direction;

mod log {

    #[test]
    fn message() {
        assert_eq!(
            gix::reference::log::message("commit", "the subject\n\nthe body".into(), 0),
            "commit (initial): the subject"
        );
        assert_eq!(
            gix::reference::log::message("other", "the subject".into(), 1),
            "other: the subject"
        );

        assert_eq!(
            gix::reference::log::message("rebase", "the subject".into(), 2),
            "rebase (merge): the subject"
        );
    }
}

#[test]
fn remote_name() -> crate::Result {
    let repo = crate::named_subrepo_opts(
        "make_remote_config_repos.sh",
        "multiple-remotes",
        gix::open::Options::isolated(),
    )?;
    for (ref_name, expected_remote) in [
        ("main", "origin"),
        ("other-main", "other"),
        ("refs/remotes/origin/main", "origin"),
        ("refs/remotes/other/main", "other"),
        ("with/two/slashes/main", "with/two/slashes"),
        ("with/two/main", "with/two"),
    ] {
        let r = repo.find_reference(ref_name)?;
        assert_eq!(
            r.remote_name(Direction::Fetch).map(|name| name.as_bstr().to_owned()),
            Some(expected_remote.into())
        );
    }
    Ok(())
}

mod find {
    use gix_ref::{FullName, FullNameRef, Target, TargetRef};

    use crate::util::hex_to_id;

    fn repo() -> crate::Result<gix::Repository> {
        crate::repo("make_references_repo.sh").map(Into::into)
    }

    #[test]
    fn and_peel() -> crate::Result {
        let repo = repo()?;
        let mut packed_tag_ref = repo.try_find_reference("dt1")?.expect("tag to exist");
        let expected: &FullNameRef = "refs/tags/dt1".try_into()?;
        assert_eq!(packed_tag_ref.name(), expected);

        assert_eq!(
            packed_tag_ref.inner.target,
            Target::Object(hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3")),
            "it points to a tag object"
        );

        let object = packed_tag_ref.peel_to_id()?;
        let the_commit = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
        assert_eq!(object, the_commit, "it is assumed to be fully peeled");
        assert_eq!(
            object,
            packed_tag_ref.peel_to_id()?,
            "peeling again yields the same object"
        );

        let mut symbolic_ref = repo.find_reference("multi-link-target1")?;

        let expected: &FullNameRef = "refs/heads/multi-link-target1".try_into()?;
        assert_eq!(symbolic_ref.name(), expected);
        assert_eq!(symbolic_ref.peel_to_id()?, the_commit);

        let expected: &FullNameRef = "refs/remotes/origin/multi-link-target3".try_into()?;
        assert_eq!(symbolic_ref.name(), expected, "it follows symbolic refs, too");
        assert_eq!(symbolic_ref.into_fully_peeled_id()?, the_commit, "idempotency");

        let mut tag_ref = repo.find_reference("dt3")?;
        assert_eq!(
            tag_ref.target(),
            TargetRef::Symbolic("refs/tags/dt2".try_into()?),
            "the ref points at another tag"
        );
        assert_eq!(tag_ref.inner.peeled, None, "it wasn't peeled yet, nothing is stored");
        let obj = tag_ref.peel_to_kind(gix::object::Kind::Tag)?;
        assert_eq!(tag_ref.peel_to_tag()?.id, obj.id);
        assert_eq!(obj.kind, gix::object::Kind::Tag);
        assert_eq!(
            obj.into_tag().decode()?.name,
            "dt2",
            "it stop at the first direct target"
        );

        let first_tag_id = hex_to_id("0f35190769db39bc70f60b6fbec9156370ce2f83");
        assert_eq!(
            tag_ref.target().id(),
            first_tag_id,
            "it's now followed to the first target"
        );
        let target_commit_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
        assert_eq!(
            tag_ref.inner.peeled,
            Some(target_commit_id),
            "It only counts as peeled as this ref is packed, and peeling in place is a way to 'make it the target' officially."
        );

        let err = tag_ref.peel_to_kind(gix::object::Kind::Blob).unwrap_err();
        let empty_tree_id = hex_to_id("4b825dc642cb6eb9a060e54bf8d69288fbee4904");
        let expected_err = format!(
            "Last encountered object {} was tree while trying to peel to blob",
            &empty_tree_id.to_string()[..7]
        );
        assert_eq!(
            err.to_string(),
            expected_err,
            "it's an error if the desired type isn't actually present"
        );
        match tag_ref.peel_to_blob() {
            Ok(_) => {
                unreachable!("target is a commit")
            }
            Err(err) => {
                assert_eq!(err.to_string(), expected_err);
            }
        }

        let obj = tag_ref.peel_to_kind(gix::object::Kind::Tree)?;
        assert!(obj.kind.is_tree());
        assert_eq!(obj.id, empty_tree_id);
        assert_eq!(tag_ref.peel_to_tree()?.id, obj.id);

        assert_eq!(
            tag_ref.target().id(),
            first_tag_id,
            "nothing changed - it still points to the target"
        );
        assert_eq!(
            tag_ref.inner.peeled,
            Some(target_commit_id),
            "the peeling cache wasn't changed"
        );

        let obj = tag_ref.peel_to_kind(gix::object::Kind::Commit)?;
        assert!(obj.kind.is_commit());
        assert_eq!(
            obj.id, target_commit_id,
            "the standard-peel peels to right after all tags"
        );
        assert_eq!(tag_ref.peel_to_commit()?.id, obj.id);

        let mut tag_ref = repo.find_reference("dt3")?;
        assert_eq!(
            tag_ref.follow_to_object()?,
            first_tag_id,
            "it's similar to peel_to_kind(), but provides the id instead"
        );
        assert_eq!(tag_ref.follow_to_object()?, first_tag_id, "it's idempotent");
        assert_eq!(
            tag_ref.target().id(),
            first_tag_id,
            "it now points to the first tag as well"
        );
        assert_eq!(
            tag_ref.inner.peeled,
            Some(target_commit_id),
            "as it was read from a packed-ref, it contains peeling information nonetheless"
        );

        Ok(())
    }

    #[test]
    fn and_follow() -> crate::Result {
        let repo = repo()?;
        let mut symbolic_ref = repo.find_reference("multi-link-target1")?;
        let first_hop = Target::Symbolic(FullName::try_from("refs/tags/multi-link-target2").expect("valid"));
        assert_eq!(symbolic_ref.target(), first_hop.to_ref());

        let second_hop = Target::Symbolic(FullName::try_from("refs/remotes/origin/multi-link-target3").expect("valid"));
        symbolic_ref = symbolic_ref.follow().expect("another hop")?;
        assert_eq!(symbolic_ref.target(), second_hop.to_ref());

        let last_hop = Target::Object(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03"));
        symbolic_ref = symbolic_ref.follow().expect("another hop")?;
        assert_eq!(symbolic_ref.target(), last_hop.to_ref());

        assert!(symbolic_ref.follow().is_none(), "direct references can't be followed");
        Ok(())
    }
}

#[test]
#[cfg(feature = "revision")]
fn set_target_id() {
    use crate::repo_rw;
    let (repo, _tmp) = repo_rw("make_basic_repo.sh").unwrap();
    let mut head_ref = repo.head_ref().unwrap().expect("present");
    let target_id = repo.rev_parse_single(":/c1").unwrap();
    let prev_id = head_ref.id();
    assert_ne!(prev_id, target_id, "we don't point to the target id yet");
    head_ref.set_target_id(target_id, "reflog message").unwrap();
    assert_eq!(head_ref.id(), target_id, "the id was set and is observable right away");

    head_ref.delete().unwrap();
    assert!(
        head_ref
            .set_target_id(prev_id, "fails")
            .unwrap_err()
            .to_string()
            .starts_with("Reference \"refs/heads/main\" was supposed to exist")
    );
}

mod remote;
