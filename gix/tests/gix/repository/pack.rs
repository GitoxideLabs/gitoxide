use std::sync::atomic::AtomicBool;

use gix_testtools::TestResult;

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
    let outcome = repo.write_pack([], &mut bytes, &AtomicBool::default())?;
    assert_eq!(outcome.num_objects, 0);
    assert_eq!(
        bytes.len(),
        12 + repo.object_hash().len_in_bytes(),
        "empty packs still carry header and checksum"
    );

    bytes.clear();
    let err = repo
        .write_pack([repo.head_id()?.detach()], &mut bytes, &AtomicBool::new(true))
        .expect_err("cancellation stops before writing");
    assert!(err.is_cancelled());
    assert!(bytes.is_empty());
    let err = repo
        .write_pack([repo.object_hash().null()], &mut bytes, &AtomicBool::default())
        .expect_err("null IDs do not represent objects");
    assert!(err.is_validation());
    assert!(bytes.is_empty());
    let missing_id = gix::ObjectId::from_hex("1".repeat(repo.object_hash().len_in_hex()).as_bytes())?;
    let err = repo
        .write_pack([missing_id], &mut bytes, &AtomicBool::default())
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
    repo.write_pack([replaced_id], &mut bytes, &AtomicBool::default())?;
    let mut original = repo.clone();
    original.objects.ignore_replacements = true;
    let mut expected = Vec::new();
    original.write_pack([replaced_id], &mut expected, &AtomicBool::default())?;
    assert_eq!(
        bytes, expected,
        "replacement refs affect history views, not packed object identities"
    );

    let mut memory = gix::odb::memory::Storage::default();
    memory.insert(replaced_id, (gix::object::Kind::Blob, b"wrong content".to_vec()));
    original.objects.set_object_memory(memory);
    let err = original
        .write_pack([replaced_id], Vec::new(), &AtomicBool::default())
        .expect_err("a corrupt object must never be sent under an unrelated ID");
    assert!(err.is_corrupted(), "checksum failures retain corruption classification");
    Ok(())
}

#[test]
fn the_implicit_empty_tree_is_materialized_in_a_pack() -> TestResult {
    use gix_object::Exists;

    let repo = crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?;
    let tree_id = gix::ObjectId::empty_tree(repo.object_hash());
    assert!(!repo.objects.exists(&tree_id), "the source has no physical empty tree");
    let mut bytes = Vec::new();
    let outcome = repo.write_pack([tree_id], &mut bytes, &AtomicBool::default())?;
    assert_eq!(outcome.num_objects, 1);
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
