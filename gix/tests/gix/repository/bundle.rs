use std::{collections::BTreeSet, io::Write as _, path::Path, process::Stdio, sync::atomic::AtomicBool};

use gix::{ObjectId, bstr::ByteSlice, refs::FullName};
use gix_testtools::TestResult;

fn refs(bytes: &[u8]) -> TestResult<Vec<(FullName, ObjectId)>> {
    bytes
        .lines()
        .filter(|line| line.first().is_some_and(u8::is_ascii_hexdigit))
        .map(|line| {
            let (id, name) = line.split_once_str(b" ").expect("reference has a name");
            Ok((name.as_bstr().try_into()?, ObjectId::from_hex(id)?))
        })
        .collect()
}

fn ids(bytes: &[u8]) -> TestResult<BTreeSet<ObjectId>> {
    Ok(bytes.lines().map(ObjectId::from_hex).collect::<Result<_, _>>()?)
}

fn prerequisites(header: &[u8]) -> TestResult<BTreeSet<ObjectId>> {
    header
        .lines()
        .filter_map(|line| line.strip_prefix(b"-"))
        .map(|line| {
            Ok(ObjectId::from_hex(
                line.split_once_str(b" ").expect("prerequisite has a comment").0,
            )?)
        })
        .collect()
}

fn git(path: &Path, args: &[&str], input: Option<&[u8]>) -> TestResult<Vec<u8>> {
    let mut child = gix_testtools::git_command(path)
        .args(args)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(input) = input {
        child.stdin.take().expect("stdin is piped").write_all(input)?;
    }
    let output = child.wait_with_output()?;
    assert!(output.status.success(), "git {args:?}: {}", output.stderr.as_bstr());
    Ok(output.stdout)
}

fn verify_bundle(
    source: &gix::Repository,
    bytes: &[u8],
    expected_refs: &[(FullName, ObjectId)],
    expected_prerequisites: &BTreeSet<ObjectId>,
    expected_objects: &BTreeSet<ObjectId>,
) -> TestResult {
    let header_length = bytes.find(b"\n\n").expect("bundle has a terminated header") + 2;
    let header = &bytes[..header_length];
    assert_eq!(
        refs(header)?,
        expected_refs,
        "only selected, non-excluded names are advertised"
    );
    assert_eq!(
        prerequisites(header)?,
        *expected_prerequisites,
        "prerequisites are the actual boundary commits"
    );
    assert!(
        header.starts_with(if source.object_hash() == gix::hash::Kind::Sha1 {
            b"# v2 git bundle\n".as_slice()
        } else {
            b"# v3 git bundle\n@object-format=sha256\n".as_slice()
        }),
        "the bundle version declares the repository's object format"
    );
    let destination = gix_testtools::tempfile::tempdir()?;
    let path = destination.path();
    git(
        path,
        &["init", "--bare", "--object-format", &source.object_hash().to_string()],
        None,
    )?;
    // Indexing the payload separately proves its exact object set without adding
    // objects from the prerequisites to the result.
    git(path, &["index-pack", "--stdin"], Some(&bytes[header_length..]))?;
    assert_eq!(
        ids(&git(
            path,
            &["cat-file", "--batch-all-objects", "--batch-check=%(objectname)"],
            None
        )?)?,
        *expected_objects,
        "the pack contains exactly the needed objects"
    );
    if !expected_prerequisites.is_empty() {
        let mut input = Vec::new();
        for commit_id in expected_prerequisites {
            writeln!(input, "{commit_id}")?;
        }
        let seed = git(
            source.workdir().expect("fixture has a worktree"),
            &["pack-objects", "--stdout", "--revs"],
            Some(&input),
        )?;
        git(path, &["index-pack", "--stdin"], Some(&seed))?;
    }
    let bundle_path = path.join("test.bundle");
    std::fs::write(&bundle_path, bytes)?;
    git(path, &["bundle", "verify", "test.bundle"], None)?;
    assert_eq!(
        refs(&git(path, &["bundle", "unbundle", "test.bundle"], None)?)?,
        expected_refs
    );
    let mut args = vec!["fsck".into(), "--full".into(), "--strict".into(), "--no-reflogs".into()];
    args.extend(expected_refs.iter().map(|(_, id)| id.to_string()));
    git(path, &args.iter().map(String::as_str).collect::<Vec<_>>(), None)?;
    Ok(())
}

#[test]
fn bundle_selections_match_git_and_roundtrip() -> TestResult {
    for object_hash in [
        #[cfg(feature = "sha1")]
        "sha1",
        #[cfg(feature = "sha256")]
        "sha256",
    ] {
        let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_bundle_creation.sh", [object_hash])?;
        let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
        let all_refs = refs(&std::fs::read(fixture.join("full.refs"))?)?;
        let remote_id = repo.find_reference("refs/remotes/origin/main")?.id().detach();
        for case in [
            "full",
            "incremental",
            "tree-excluded",
            "blob-excluded",
            "disjoint-shared",
        ] {
            let expected_refs = refs(&std::fs::read(fixture.join(format!("{case}.refs")))?)?;
            let selected = if matches!(case, "full" | "incremental") {
                all_refs.clone()
            } else {
                expected_refs.clone()
            };
            let excluded = (case != "full").then_some(remote_id);
            let mut bytes = Vec::new();
            let checksum = repo.write_bundle(
                selected,
                excluded,
                &mut bytes,
                gix::progress::Discard,
                &AtomicBool::new(false),
            )?;
            assert!(bytes.ends_with(checksum.as_slice()), "the result is the pack checksum");
            verify_bundle(
                &repo,
                &bytes,
                &expected_refs,
                &prerequisites(&std::fs::read(fixture.join(format!("{case}.header")))?)?,
                &ids(&std::fs::read(fixture.join(format!("{case}.objects")))?)?,
            )?;
        }
    }
    Ok(())
}

#[test]
fn exclusions_cannot_make_advertised_tags_or_blobs_incomplete() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_bundle_creation.sh", ["sha1"])?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    for (case, excluded, prerequisite, objects) in [
        (
            "tag-excluded",
            "refs/remotes/origin/main",
            Some("base-id"),
            "tag-excluded",
        ),
        ("main-only", "refs/data/blob", None, "main-only"),
    ] {
        let selected = refs(&std::fs::read(fixture.join(format!("{case}.refs")))?)?;
        let expected_prerequisites = match prerequisite {
            Some(name) => ids(&std::fs::read(fixture.join(name))?)?,
            None => BTreeSet::new(),
        };
        let mut bytes = Vec::new();
        repo.write_bundle(
            selected.clone(),
            [repo.find_reference(excluded)?.id().detach()],
            &mut bytes,
            gix::progress::Discard,
            &AtomicBool::new(false),
        )?;
        verify_bundle(
            &repo,
            &bytes,
            &selected,
            &expected_prerequisites,
            &ids(&std::fs::read(fixture.join(format!("{objects}.objects")))?)?,
        )?;
    }
    Ok(())
}

#[test]
fn merge_boundaries_and_exclusions_ignore_commit_dates() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_bundle_creation.sh", ["sha1"])?;
    let repo = gix::open_opts(fixture.join("repo"), crate::restricted())?;
    let selected = refs(&std::fs::read(fixture.join("merge.refs"))?)?;
    let mut bytes = Vec::new();
    repo.write_bundle(
        selected.clone(),
        ["refs/remotes/origin/main", "refs/remotes/origin/side"]
            .map(|name| repo.find_reference(name).map(|reference| reference.id().detach()))
            .into_iter()
            .collect::<gix::Result<Vec<_>>>()?,
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::new(false),
    )?;
    verify_bundle(
        &repo,
        &bytes,
        &selected,
        &prerequisites(&std::fs::read(fixture.join("merge.header"))?)?,
        &ids(&std::fs::read(fixture.join("merge.objects"))?)?,
    )?;

    let expected = refs(&std::fs::read(fixture.join("disjoint-shared.refs"))?)?;
    let mut selected = expected.clone();
    selected.push((
        "refs/heads/skew-old".try_into()?,
        repo.find_reference("refs/heads/skew-old")?.id().detach(),
    ));
    bytes.clear();
    repo.write_bundle(
        selected,
        [repo.find_reference("refs/remotes/origin/skew")?.id().detach()],
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::new(false),
    )?;
    verify_bundle(
        &repo,
        &bytes,
        &expected,
        &BTreeSet::new(),
        &ids(&std::fs::read(fixture.join("disjoint-shared.objects"))?)?,
    )?;
    Ok(())
}

#[test]
fn duplicate_names_and_memory_objects_preserve_reference_bytes() -> TestResult {
    let mut repo = crate::named_repo("make_empty_repo.sh")?;
    repo.objects.enable_object_memory();
    let blob_id = repo.write_blob(b"in-memory bundle data")?.detach();
    let raw_name: FullName = b"refs/data/non-utf8-\xff".as_bstr().try_into()?;
    let selected = vec![(raw_name, blob_id), ("refs/data/other".try_into()?, blob_id)];
    let mut bytes = Vec::new();
    repo.write_bundle(
        selected.iter().cloned().chain(selected.iter().cloned()),
        [],
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::new(false),
    )?;
    verify_bundle(&repo, &bytes, &selected, &BTreeSet::new(), &BTreeSet::from([blob_id]))?;
    assert_eq!(
        repo.objects.num_objects_in_memory(),
        1,
        "bundle creation does not persist or consume memory objects"
    );
    Ok(())
}

#[test]
fn invalid_selection_missing_objects_and_cancellation_produce_no_output() -> TestResult {
    use gix::objs::Write;
    let mut repo = crate::named_repo("make_empty_repo.sh")?;
    repo.objects.enable_object_memory();
    let blob_id = repo.write_blob(b"valid")?.detach();
    let missing_id = repo.object_hash().null();
    let tree_id = repo
        .write_object(&gix::objs::Tree {
            entries: vec![gix::objs::tree::Entry {
                mode: gix::objs::tree::EntryKind::Blob.into(),
                filename: "missing".into(),
                oid: missing_id,
            }],
        })?
        .detach();
    let commit_id = repo
        .write_object(&gix::objs::Commit {
            tree: repo.object_hash().empty_tree(),
            parents: [missing_id].into(),
            author: Default::default(),
            committer: Default::default(),
            encoding: None,
            message: "missing parent".into(),
            extra_headers: Vec::new(),
        })?
        .detach();
    let malformed_tag_id = repo.objects.write_buf(gix::objs::Kind::Tag, b"invalid tag")?;
    let malformed_commit_id = repo.objects.write_buf(gix::objs::Kind::Commit, b"invalid commit")?;
    for (selected, excluded, interrupt) in [
        (vec![], vec![], false),
        (vec![("refs/data/a", blob_id)], vec![blob_id], false),
        (vec![("refs/data/a", blob_id), ("refs/data/a", tree_id)], vec![], false),
        (vec![("refs/data/a", blob_id)], vec![], true),
        (vec![("refs/data/a", missing_id)], vec![], false),
        (vec![("refs/data/a", tree_id)], vec![], false),
        (vec![("refs/data/a", commit_id)], vec![], false),
        (vec![("refs/data/a", malformed_tag_id)], vec![], false),
        (vec![("refs/data/a", malformed_commit_id)], vec![], false),
        (
            vec![(
                "refs/data/a",
                ObjectId::from_hex(b"0000000000000000000000000000000000000000000000000000000000000000")?,
            )],
            vec![],
            false,
        ),
    ] {
        let selected = selected
            .into_iter()
            .map(|(name, id)| Ok((name.try_into()?, id)))
            .collect::<TestResult<Vec<(FullName, ObjectId)>>>()?;
        let mut bytes = Vec::new();
        let err = repo
            .write_bundle(
                selected,
                excluded,
                &mut bytes,
                gix::progress::Discard,
                &AtomicBool::new(interrupt),
            )
            .expect_err("invalid input cannot form a complete bundle");
        assert!(
            bytes.is_empty(),
            "selection and pack preflight failures leave the output untouched: {err:?}"
        );
        if interrupt {
            assert!(err.is_cancelled(), "cancellation retains its error classification");
        }
    }
    Ok(())
}

#[test]
fn output_failures_preserve_the_written_prefix_and_error() -> TestResult {
    struct FailAfter {
        bytes: Vec<u8>,
        remaining: usize,
    }
    impl std::io::Write for FailAfter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.remaining == 0 {
                return Err(std::io::Error::other("injected bundle output failure"));
            }
            let len = self.remaining.min(buf.len());
            self.bytes.extend_from_slice(&buf[..len]);
            self.remaining -= len;
            Ok(len)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut repo = crate::named_repo("make_empty_repo.sh")?;
    repo.objects.enable_object_memory();
    let blob_id = repo.write_blob(b"bundle output")?.detach();
    let selected = [("refs/data/blob".try_into()?, blob_id)];
    let mut expected = Vec::new();
    repo.write_bundle(
        selected.clone(),
        [],
        &mut expected,
        gix::progress::Discard,
        &AtomicBool::new(false),
    )?;
    let header_length = expected.find(b"\n\n").expect("header ends before pack") + 2;
    for remaining in [0, 1, header_length - 1, header_length + 8] {
        let mut out = FailAfter {
            bytes: Vec::new(),
            remaining,
        };
        let err = repo
            .write_bundle(
                selected.clone(),
                [],
                &mut out,
                gix::progress::Discard,
                &AtomicBool::new(false),
            )
            .expect_err("output errors are not swallowed");
        assert!(
            format!("{err:?}").contains("injected bundle output failure"),
            "the original output error is retained"
        );
        assert_eq!(
            out.bytes,
            expected[..remaining],
            "partially written headers or packs are not replayed"
        );
    }
    Ok(())
}
