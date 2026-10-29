use std::{io::Write as _, process::Stdio, sync::atomic::AtomicBool};

use gix::{ObjectId, bstr::ByteSlice};
use gix_testtools::TestResult;

fn object_ids(repo: &gix::Repository) -> TestResult<Vec<ObjectId>> {
    Ok(
        std::fs::read(repo.workdir().expect("fixture has a worktree").join("objects"))?
            .lines()
            .map(|line| ObjectId::from_hex(line.split(|byte| *byte == b' ').next().expect("line has an id")))
            .collect::<Result<_, _>>()?,
    )
}

fn verify_pack(repo: &gix::Repository, bytes: &[u8], expected: &[ObjectId]) -> TestResult {
    let tmp = gix_testtools::tempfile::tempdir()?;
    let hash = repo.object_hash().to_string();
    let output = gix_testtools::git_command(tmp.path())
        .args(["init", "--bare", "--object-format", &hash])
        .output()?;
    assert!(
        output.status.success(),
        "isolated destination initializes: {}",
        output.stderr.as_bstr()
    );
    let mut child = gix_testtools::git_command(tmp.path())
        .args(["index-pack", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child.stdin.take().expect("stdin is piped").write_all(bytes)?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "Git accepts a self-contained pack: {}",
        output.stderr.as_bstr()
    );
    let output = gix_testtools::git_command(tmp.path())
        .args(["cat-file", "--batch-all-objects", "--batch-check=%(objectname)"])
        .output()?;
    assert!(output.status.success(), "Git enumerates the imported objects");
    let mut actual = output
        .stdout
        .lines()
        .map(ObjectId::from_hex)
        .collect::<Result<Vec<_>, _>>()?;
    actual.sort();
    let mut expected = expected.to_vec();
    expected.sort();
    expected.dedup();
    assert_eq!(
        actual, expected,
        "packing includes exactly the requested objects without duplicates or replacements"
    );
    Ok(())
}

#[test]
fn packed_loose_and_duplicate_objects_roundtrip_through_git() -> TestResult {
    let repo = crate::named_repo("make_pack_creation.sh")?;
    let root = repo.workdir().expect("fixture has a worktree");
    let mut ids = object_ids(&repo)?;
    ids.push(ObjectId::from_hex(std::fs::read(root.join("loose-id"))?.trim())?);
    ids.extend(ids.clone());
    let mut bytes = Vec::new();
    let checksum = repo.write_pack(
        ids.iter().copied(),
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::default(),
    )?;
    assert!(
        bytes.ends_with(checksum.pack_hash.as_slice()),
        "the returned hash is the pack trailer"
    );
    verify_pack(&repo, &bytes, &ids)?;
    let entries = gix_pack::data::input::BytesToEntriesIter::new_from_header(
        bytes.as_slice(),
        gix_pack::data::input::Mode::Verify,
        gix_pack::data::input::EntryDataMode::Ignore,
        repo.object_hash(),
    )?
    .collect::<gix::Result<Vec<_>>>()?;
    assert!(
        entries
            .iter()
            .any(|entry| matches!(entry.header, gix_pack::data::entry::Header::OfsDelta { .. })),
        "selected delta bases allow existing deltas to be reused"
    );
    // Each packed object can be exported without bringing along its delta base.
    for id in object_ids(&repo)? {
        bytes.clear();
        repo.write_pack([id], &mut bytes, gix::progress::Discard, &AtomicBool::default())?;
        verify_pack(&repo, &bytes, &[id])?;
    }
    Ok(())
}

#[test]
fn empty_and_in_memory_packs_roundtrip_through_git() -> TestResult {
    let mut repo = crate::named_repo("make_pack_creation.sh")?;
    let mut bytes = Vec::new();
    repo.write_pack([], &mut bytes, gix::progress::Discard, &AtomicBool::default())?;
    verify_pack(&repo, &bytes, &[])?;
    repo.objects.enable_object_memory();
    let blob_id = repo.write_blob(b"memory only")?.detach();
    bytes.clear();
    repo.write_pack([blob_id], &mut bytes, gix::progress::Discard, &AtomicBool::default())?;
    verify_pack(&repo, &bytes, &[blob_id])?;
    assert_eq!(
        repo.objects.num_objects_in_memory(),
        1,
        "export leaves the source object memory intact"
    );
    Ok(())
}

#[test]
fn missing_objects_and_cancellation_do_not_write_a_header() -> TestResult {
    let repo = crate::named_repo("make_pack_creation.sh")?;
    let missing_id = ObjectId::from_hex("1".repeat(repo.object_hash().len_in_hex()).as_bytes())?;
    for (ids, cancelled) in [
        (vec![repo.object_hash().null()], false),
        (vec![repo.head_id()?.detach(), missing_id], false),
        (object_ids(&repo)?, true),
    ] {
        let mut bytes = Vec::new();
        let error = repo
            .write_pack(ids, &mut bytes, gix::progress::Discard, &AtomicBool::new(cancelled))
            .expect_err("invalid or cancelled packs must fail");
        assert!(bytes.is_empty(), "preflight failures do not write a pack header");
        if cancelled {
            assert!(error.is_cancelled(), "cancellation is classified");
        }
    }
    Ok(())
}

#[test]
fn output_failure_is_reported_and_can_leave_a_partial_pack() -> TestResult {
    let repo = crate::named_repo("make_pack_creation.sh")?;
    let mut prefix = [0; 8];
    repo.write_pack(
        [repo.head_id()?.detach()],
        prefix.as_mut_slice(),
        gix::progress::Discard,
        &AtomicBool::default(),
    )
    .expect_err("a writer unable to accept the full pack must fail");
    assert_eq!(&prefix[..4], b"PACK", "write errors can follow a partial header");
    Ok(())
}

#[cfg(all(feature = "sha1", feature = "sha256"))]
#[test]
fn object_ids_must_match_the_repository_hash() -> TestResult {
    let repo = crate::named_repo("make_pack_creation.sh")?;
    let wrong_hash = if repo.object_hash() == gix::hash::Kind::Sha1 {
        gix::hash::Kind::Sha256
    } else {
        gix::hash::Kind::Sha1
    };
    let mut bytes = Vec::new();
    let error = repo
        .write_pack(
            [wrong_hash.null()],
            &mut bytes,
            gix::progress::Discard,
            &AtomicBool::default(),
        )
        .expect_err("mixed object hashes cannot form a pack");
    assert!(error.is_validation(), "the hash mismatch is invalid input");
    assert!(bytes.is_empty(), "invalid input is rejected before output");
    Ok(())
}

#[test]
fn generated_pack_is_accepted_by_git_and_contains_each_object_once() -> TestResult {
    let repo = crate::remote::repo("base");
    let tip_id = repo.head_id()?.detach();
    let object_ids: Vec<_> = gix::traverse::object::BreadthFirst::new([tip_id], &repo.objects)
        .map(|entry| entry.map(|entry| entry.id))
        .collect::<gix::Result<_>>()?;
    let mut bytes = Vec::new();
    let outcome = repo.write_pack(
        object_ids.iter().copied().chain([tip_id]),
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::default(),
    )?;
    assert_eq!(
        outcome.num_objects as usize,
        object_ids.len(),
        "repeated IDs produce one entry"
    );
    assert_eq!(
        outcome.bytes_written as usize,
        bytes.len(),
        "the count includes the pack trailer"
    );
    assert_eq!(
        bytes.get(bytes.len() - repo.object_hash().len_in_bytes()..),
        Some(outcome.pack_hash.as_slice())
    );

    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    let pack_path = destination.path().join("input.pack");
    std::fs::write(&pack_path, bytes)?;
    let output = gix_testtools::git_command(destination.path())
        .args(["index-pack", "--strict", "--stdin"])
        .stdin(std::fs::File::open(pack_path)?)
        .output()?;
    assert!(
        output.status.success(),
        "Git accepts the pack: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let listed = gix_testtools::git(
        destination.path(),
        "cat-file --batch-all-objects --batch-check='%(objectname)'",
    )?;
    let actual: std::collections::BTreeSet<_> = listed.lines().map(str::to_owned).collect();
    let expected: std::collections::BTreeSet<_> = object_ids.iter().map(ToString::to_string).collect();
    assert_eq!(
        actual, expected,
        "Git reads every reachable object from the generated pack"
    );
    Ok(())
}

#[test]
fn empty_pack_and_failures() -> TestResult {
    let repo = crate::remote::repo("base");
    let mut bytes = Vec::new();
    let outcome = repo.write_pack([], &mut bytes, gix::progress::Discard, &AtomicBool::default())?;
    assert_eq!(outcome.num_objects, 0);
    assert_eq!(
        bytes.len(),
        12 + repo.object_hash().len_in_bytes(),
        "empty packs still carry header and checksum"
    );

    bytes.clear();
    let err = repo
        .write_pack(
            [repo.head_id()?.detach()],
            &mut bytes,
            gix::progress::Discard,
            &AtomicBool::new(true),
        )
        .expect_err("cancellation stops before writing");
    assert!(err.is_cancelled());
    assert!(bytes.is_empty());
    let err = repo
        .write_pack(
            [repo.object_hash().null()],
            &mut bytes,
            gix::progress::Discard,
            &AtomicBool::default(),
        )
        .expect_err("null IDs do not represent objects");
    assert!(err.is_validation());
    assert!(bytes.is_empty());
    let missing_id = gix::ObjectId::from_hex("1".repeat(repo.object_hash().len_in_hex()).as_bytes())?;
    let err = repo
        .write_pack([missing_id], &mut bytes, gix::progress::Discard, &AtomicBool::default())
        .expect_err("missing objects are not silently skipped");
    assert!(
        err.is_not_found(),
        "the error identifies a missing object, not an invalid ID"
    );
    Ok(())
}

#[test]
fn replacement_refs_do_not_change_packed_objects_and_corruption_is_rejected() -> TestResult {
    let repo = crate::named_subrepo_opts("make_replacement_refs.sh", "repo", gix::open::Options::isolated())?;
    let replaced_id = repo.find_reference("refs/heads/squash")?.id().detach();
    let mut bytes = Vec::new();
    repo.write_pack(
        [replaced_id],
        &mut bytes,
        gix::progress::Discard,
        &AtomicBool::default(),
    )?;
    let mut original = repo.clone();
    original.objects.ignore_replacements = true;
    let mut expected = Vec::new();
    original.write_pack(
        [replaced_id],
        &mut expected,
        gix::progress::Discard,
        &AtomicBool::default(),
    )?;
    assert_eq!(
        bytes, expected,
        "replacement refs affect history views, not packed object identities"
    );

    let mut memory = gix::odb::memory::Storage::default();
    memory.insert(replaced_id, (gix::object::Kind::Blob, b"wrong content".to_vec()));
    original.objects.set_object_memory(memory);
    bytes.clear();
    let err = original
        .write_pack(
            [replaced_id],
            &mut bytes,
            gix::progress::Discard,
            &AtomicBool::default(),
        )
        .expect_err("a corrupt object must never be sent under an unrelated ID");
    assert!(err.is_corrupted(), "checksum failures retain corruption classification");
    assert!(
        bytes.is_empty(),
        "corruption is rejected before writing the pack header"
    );
    Ok(())
}

#[test]
fn the_implicit_empty_tree_is_materialized_in_a_pack() -> TestResult {
    use gix_object::Exists;

    let repo = crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?;
    let tree_id = gix::ObjectId::empty_tree(repo.object_hash());
    assert!(!repo.objects.exists(&tree_id), "the source has no physical empty tree");
    let mut bytes = Vec::new();
    let outcome = repo.write_pack([tree_id], &mut bytes, gix::progress::Discard, &AtomicBool::default())?;
    assert_eq!(outcome.num_objects, 1);
    assert!(
        !repo.objects.exists(&tree_id),
        "packing does not persist the implicit tree"
    );
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    let pack_path = destination.path().join("empty-tree.pack");
    std::fs::write(&pack_path, bytes)?;
    let output = gix_testtools::git_command(destination.path())
        .args(["index-pack", "--strict", "--stdin"])
        .stdin(std::fs::File::open(pack_path)?)
        .output()?;
    assert!(output.status.success(), "Git accepts the materialized tree");
    assert_eq!(
        gix_testtools::git(destination.path(), &format!("cat-file -t {tree_id}"))?.trim(),
        "tree"
    );
    Ok(())
}
