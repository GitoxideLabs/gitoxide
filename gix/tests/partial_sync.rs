#![cfg(all(feature = "blocking-network-client", feature = "revision", feature = "pack-generate"))]

use gix::{
    bstr::BString,
    push,
    remote::{Direction, fetch},
};
use gix_testtools::TestResult;
use std::sync::atomic::AtomicBool;

fn source() -> TestResult<std::path::PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("make_partial_sync_repo.sh")?
        .canonicalize()?
        .join("source"))
}

fn fetch_remote<'repo>(repo: &'repo gix::Repository, source: &std::path::Path) -> gix::Result<gix::Remote<'repo>> {
    let mut remote = repo
        .remote_at(gix::path::into_bstr(source)?)?
        .with_fetch_tags(fetch::Tags::None);
    remote.replace_refspecs(["+refs/heads/main:refs/remotes/origin/main"], Direction::Fetch)?;
    Ok(remote)
}

#[test]
fn blobless_protocol_fetch_and_hydration() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let source = source()?;
    for protocol in ["1", "2"] {
        let dir = gix_testtools::tempfile::tempdir()?;
        gix::init_bare(dir.path())?;
        let repo = gix::open_opts(
            dir.path(),
            gix::open::Options::isolated().config_overrides([format!("protocol.version={protocol}")]),
        )?;
        let mut remote = fetch_remote(&repo, &source)?;
        let outcome = remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_filter(Some(fetch::Filter::BlobNone))
            .receive(gix::progress::Discard, &AtomicBool::default())?;
        assert!(
            matches!(outcome.status, fetch::Status::Change { .. }),
            "filtered fetch transfers a pack"
        );
        let tip = repo
            .find_reference("refs/remotes/origin/main")?
            .into_fully_peeled_id()?
            .object()?
            .peel_to_commit()?;
        let blob_id = tip.tree()?.find_entry("value").expect("fixture value").object_id();
        assert!(!repo.has_object(blob_id), "tip blob is omitted by protocol {protocol}");
        remote.replace_refspecs([BString::from(blob_id.to_string())], Direction::Fetch)?;
        remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_filter(Some(fetch::Filter::BlobNone))
            .receive(gix::progress::Discard, &AtomicBool::default())?;
        assert_eq!(
            repo.find_blob(blob_id)?.data,
            b"tip value\n",
            "explicit blob hydration works in protocol {protocol}"
        );
    }
    Ok(())
}

#[test]
fn partial_clone_push_does_not_require_advertised_historical_blobs() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let source = source()?;
    for wire in [false, true] {
        let dir = gix_testtools::tempfile::tempdir()?;
        let receiver_path = dir.path().join("receiver");
        let status = gix_testtools::git_command(dir.path())
            .args(["clone", "--bare"])
            .arg(&source)
            .arg(&receiver_path)
            .status()?;
        assert!(status.success(), "create writable receiver");
        let local_path = dir.path().join("client");
        gix::init_bare(&local_path)?;
        let repo = gix::open_opts(&local_path, gix::open::Options::isolated())?;
        let remote = fetch_remote(&repo, &receiver_path)?;
        remote.fetch(
            fetch::Options {
                filter: Some(fetch::Filter::BlobNone),
                ..Default::default()
            },
            gix::progress::Discard,
            &AtomicBool::default(),
        )?;
        let parent = repo
            .find_reference("refs/remotes/origin/main")?
            .into_fully_peeled_id()?
            .object()?
            .peel_to_commit()?;
        let tree = parent.tree()?;
        let old_blob = repo
            .find_commit(parent.parent_ids().next().expect("parent"))?
            .tree()?
            .find_entry("value")
            .expect("value")
            .object_id();
        assert!(!repo.has_object(old_blob), "historical value remains promised");
        let signature = gix::actor::Signature {
            name: "Author".into(),
            email: "author@example.com".into(),
            time: gix::date::Time::new(946684900, 0),
        };
        let commit = gix::objs::Commit {
            tree: tree.id,
            parents: vec![parent.id].into(),
            author: signature.clone(),
            committer: signature,
            message: "new metadata commit".into(),
            encoding: None,
            extra_headers: Default::default(),
        };
        let commit_id = repo.write_object(&commit)?.detach();
        let oracle_path = dir.path().join("oracle");
        let status = gix_testtools::git_command(dir.path())
            .args(["clone", "--bare"])
            .arg(&source)
            .arg(&oracle_path)
            .status()?;
        assert!(status.success(), "create the Git oracle receiver");
        let status = gix_testtools::git_command(&local_path)
            .arg("push")
            .arg(&oracle_path)
            .arg(format!("{commit_id}:refs/heads/main"))
            .env("GIT_NO_LAZY_FETCH", "1")
            .status()?;
        assert!(
            status.success(),
            "Git sends only new objects without hydrating old values"
        );
        let update = push::Update {
            destination: "refs/heads/main".try_into()?,
            new_id: Some(commit_id),
            lease: push::Lease::Any,
            force: false,
        };
        let outcome = if wire {
            remote.connect(Direction::Push)?.push(
                [update],
                Default::default(),
                gix::progress::Discard,
                |_, _| Ok(()),
                &AtomicBool::default(),
            )?
        } else {
            remote.push(
                [update],
                Default::default(),
                gix::progress::Discard,
                |_, _| Ok(()),
                &AtomicBool::default(),
            )?
        };
        assert!(
            matches!(outcome.updates[0].status, push::Status::Updated { .. }),
            "partial-clone publication works through wire={wire}"
        );
        let receiver = gix::open_opts(&receiver_path, gix::open::Options::isolated())?;
        assert_eq!(receiver.head_id()?, commit_id, "receiver advances");
        assert!(
            !repo.has_object(old_blob),
            "publication did not hydrate historical data"
        );
        let invalid_tree = gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: "missing".into(),
                oid: gix::ObjectId::from_hex(b"ffffffffffffffffffffffffffffffffffffffff")?,
            }],
        };
        let mut invalid_commit = commit.clone();
        invalid_commit.tree = repo.write_object(&invalid_tree)?.detach();
        invalid_commit.parents = vec![commit_id].into();
        let invalid_id = repo.write_object(&invalid_commit)?.detach();
        let update = push::Update {
            destination: "refs/heads/main".try_into()?,
            new_id: Some(invalid_id),
            lease: push::Lease::Any,
            force: false,
        };
        let failed = if wire {
            remote.connect(Direction::Push)?.push(
                [update],
                Default::default(),
                gix::progress::Discard,
                |_, _| Ok(()),
                &AtomicBool::default(),
            )
        } else {
            remote.push(
                [update],
                Default::default(),
                gix::progress::Discard,
                |_, _| Ok(()),
                &AtomicBool::default(),
            )
        };
        assert!(
            failed.is_err(),
            "a missing newly referenced object still fails, wire={wire}"
        );
        assert_eq!(
            receiver.head_id()?,
            commit_id,
            "a failed transfer leaves the receiver unchanged"
        );
    }
    Ok(())
}
