use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    io::Cursor,
};

use gix::{Repository, Result, objs::Write};
use gix_testtools::TestResult;

/// Counts allocations and reallocations on the calling thread.
///
/// The allocator is global, but each thread has its own counter. Tests take the difference
/// between two reads on the same thread, so allocations from other tests running in parallel
/// don't affect the result. This also means allocations on worker threads spawned by the code
/// under test aren't included in the measurement.
struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn count_allocation() {
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get().wrapping_add(1)));
}

// SAFETY: Allocation requests are forwarded unchanged, and counting neither allocates nor unwinds.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count_allocation();
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

const ANNOTATED_TAG: &str = "refs/tags/point-at-commit";

#[test]
fn packed_reference_iteration_reuses_name_storage() -> TestResult {
    check_reference_iteration(
        &open_fixture("make_many_packed_refs.sh", ".")?,
        "refs/heads/packed/",
        "refs/heads/packed/00000",
        100_000,
        0,
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn loose_reference_iteration_reuses_directory_storage() -> TestResult {
    let repo = open_fixture("make_many_loose_refs.sh", ".")?;
    let prefix = "refs/heads/loose/";
    // The first ref warms directory 000. Measure what std needs to list the remaining siblings.
    let mut directory_allocations = 0;
    for group in 1..10 {
        let path = repo.git_dir().join(format!("{prefix}{group:03}"));
        let before = ALLOCATIONS.get();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            std::hint::black_box((entry.file_type()?, entry.file_name()));
        }
        directory_allocations += ALLOCATIONS.get().wrapping_sub(before);
    }
    check_reference_iteration(&repo, prefix, "refs/heads/loose/000/000", 1_000, directory_allocations)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
#[test]
fn loose_symbolic_reference_iteration_reuses_target_storage() -> TestResult {
    check_reference_iteration(
        &open_fixture("make_many_loose_refs.sh", ".")?,
        "refs/heads/symbolic/",
        "refs/heads/symbolic/0000",
        100,
        0,
    )
}

fn check_reference_iteration(
    repo: &Repository,
    prefix: &str,
    first_name: &str,
    expected_count: usize,
    directory_allocations: usize,
) -> TestResult {
    let platform = repo.references()?;
    let mut refs = platform.prefixed(prefix)?;
    let first = refs.next().expect("the prefix contains references")?;
    assert_eq!(first.name().as_bstr(), first_name, "the smallest matching ref is first");
    let mut previous = first.name().as_bstr().to_vec();
    let target = first.target().into_owned();
    drop(first);

    let before = ALLOCATIONS.get();
    let mut count = 1;
    for reference in refs {
        let reference = reference?;
        assert_eq!(target, reference.target(), "each reference retains its target");
        let name: &[u8] = reference.name().as_bstr().as_ref();
        assert!(previous.as_slice() < name, "references are returned in name order");
        previous.clear();
        previous.extend_from_slice(name);
        count += 1;
    }
    let allocations = ALLOCATIONS.get().wrapping_sub(before);
    assert_eq!(count, expected_count, "the prefix matches every generated reference");
    assert!(
        allocations <= directory_allocations,
        "the scan made {allocations} allocations, exceeding the {directory_allocations} used to list the remaining directories"
    );
    Ok(())
}

#[test]
fn reusable_buffer_is_cleared_and_returned_on_drop() -> TestResult {
    let mut repo = open_fixture("make_worktree_repo.sh", "repo")?;
    let mut buf = repo.empty_reusable_buffer();
    buf.extend_from_slice(b"previous contents");
    let storage = buf.as_ptr();
    drop(buf);

    let buf = repo.empty_reusable_buffer();
    assert!(buf.is_empty(), "a checked-out buffer has no leftover data");
    assert_eq!(buf.as_ptr(), storage, "checking out a buffer preserves its allocation");
    drop(buf);

    repo.set_freelist(None);
    repo.empty_reusable_buffer().extend_from_slice(b"uncached contents");
    assert_eq!(
        repo.empty_reusable_buffer().capacity(),
        0,
        "disabled freelists retain no buffers"
    );
    Ok(())
}

#[test]
fn object_reads_reuse_storage_across_object_kinds() -> TestResult {
    let mut repo = open_fixture("make_worktree_repo.sh", "repo")?;
    let commit_id = repo.head_id()?.detach();
    let tree_id = repo.find_commit(commit_id)?.tree_id()?.detach();
    let blob_id = repo
        .find_tree(tree_id)?
        .find_entry("a")
        .expect("the fixture has a non-empty blob at a")
        .object_id();
    let storage = seed_buffer(&mut repo);

    let object = repo.find_object(blob_id)?;
    assert_eq!(object.data.as_ptr(), storage, "find_object checks out the buffer");
    drop(object);

    let object = repo.try_find_object(blob_id)?.expect("the blob exists");
    assert_eq!(
        object.data.as_ptr(),
        storage,
        "try_find_object reuses the dropped object's buffer"
    );
    drop(object);

    let blob = repo.find_blob(blob_id)?;
    assert_eq!(blob.data.as_ptr(), storage, "Blob uses the shared buffer");
    assert_eq!(blob.data, b"hello\n", "the reused buffer contains the blob payload");
    drop(blob);

    let tree = repo.find_tree(tree_id)?;
    assert_eq!(tree.data.as_ptr(), storage, "Tree reuses the dropped Blob's buffer");
    drop(tree);

    let commit = repo.find_commit(commit_id)?;
    assert_eq!(commit.data.as_ptr(), storage, "Commit reuses the dropped Tree's buffer");
    drop(commit);

    assert_eq!(
        repo.empty_reusable_buffer().as_ptr(),
        storage,
        "Commit also returns its buffer"
    );
    Ok(())
}

#[test]
fn buffered_writes_do_not_allocate_for_duplicate_objects() -> TestResult {
    let mut repo = open_fixture("make_worktree_repo.sh", "repo")?.with_object_memory();
    let data = &b"object to serialize or stream"[..];
    let blob = gix::objs::BlobRef { data };
    let kind = gix::objs::Kind::Blob;
    // Pre-store the object so the measurement covers buffering, not object insertion.
    let blob_id = repo.write_blob(data)?.detach();
    let storage = seed_buffer(&mut repo);

    assert_no_allocations("write_object", || repo.write_object(blob).map(gix::Id::detach), blob_id)?;
    assert_no_allocations(
        "write_blob_stream",
        || repo.write_blob_stream(data).map(gix::Id::detach),
        blob_id,
    )?;
    assert_no_allocations("Write::write", || repo.write(&blob), blob_id)?;
    assert_no_allocations(
        "Write::write_stream",
        || repo.write_stream(kind, data.len() as u64, &mut Cursor::new(data)),
        blob_id,
    )?;
    assert_no_allocations(
        "Write::write_stream_with_known_id",
        || repo.write_stream_with_known_id(kind, data.len() as u64, &mut Cursor::new(data), blob_id),
        blob_id,
    )?;
    assert_buffer_returned(&mut repo, storage, data);
    Ok(())
}

#[test]
fn nested_tree_lookup_reuses_scratch_storage() -> TestResult {
    let mut repo = open_fixture("make_worktree_repo.sh", "repo")?;
    let tree = repo.head_tree()?;
    let subtree = tree
        .find_entry("dir")
        .expect("the root contains dir")
        .object()?
        .into_tree();
    let blob_id = subtree.find_entry("c").expect("dir contains c").object_id();
    let data = subtree.data.clone();
    drop(subtree);
    let mut scratch = repo.empty_reusable_buffer();
    scratch.reserve(tree.data.len().max(data.len()));
    let storage = scratch.as_ptr();
    drop(scratch);

    let entry = tree.lookup_entry(["dir", "c"])?.expect("lookup descends into dir");
    assert_eq!(entry.object_id(), blob_id, "lookup returns the nested entry");
    drop(tree);
    assert_buffer_returned(&mut repo, storage, &data);
    Ok(())
}

#[test]
fn reading_and_peeling_an_annotated_tag_reuses_object_storage() -> TestResult {
    let mut repo = open_fixture("make_head_repos.sh", "tag-symbolic")?;
    let commit = repo.head_commit()?;
    let commit_id = commit.id;
    let commit_data = commit.data.clone();
    drop(commit);
    let storage = seed_buffer(&mut repo);

    let mut tag = repo.find_reference(ANNOTATED_TAG)?;
    let tag_object = repo.find_tag(tag.id())?;
    assert_eq!(tag_object.data.as_ptr(), storage, "Tag checks out the shared buffer");
    drop(tag_object);
    assert_eq!(
        tag.peel_to_id()?,
        commit_id,
        "peeling resolves the annotated tag to its commit"
    );
    drop(tag);
    assert_buffer_returned(&mut repo, storage, &commit_data);
    Ok(())
}

#[test]
fn peeled_reference_iteration_returns_its_object_buffer() -> TestResult {
    let mut repo = open_fixture("make_head_repos.sh", "tag-symbolic")?;
    let commit = repo.head_commit()?;
    let commit_id = commit.id;
    let commit_data = commit.data.clone();
    drop(commit);
    let storage = seed_buffer(&mut repo);

    let platform = repo.references()?;
    let mut tags = platform.prefixed(ANNOTATED_TAG)?.peeled()?;
    let tag = tags.next().expect("the fixture has an annotated tag")?;
    assert_eq!(tag.id(), commit_id, "iteration peels the tag before yielding it");
    drop(tag);
    assert!(tags.next().is_none(), "only the annotated tag matches the prefix");
    drop(tags);
    drop(platform);
    assert_buffer_returned(&mut repo, storage, &commit_data);
    Ok(())
}

fn open_fixture(script: &str, subdir: &str) -> TestResult<Repository> {
    let path = gix_testtools::scripted_fixture_read_only(script)?;
    Ok(gix::open_opts(path.join(subdir), gix::open::Options::isolated())?)
}

fn seed_buffer(repo: &mut Repository) -> *const u8 {
    let buf = Vec::with_capacity(4096);
    let storage = buf.as_ptr();
    repo.set_freelist(Some(vec![buf]));
    storage
}

fn assert_buffer_returned(repo: &mut Repository, storage: *const u8, data: &[u8]) {
    let pool = repo.set_freelist(None).expect("the freelist is enabled");
    let buf = pool
        .iter()
        .find(|buf| buf.as_ptr() == storage)
        .expect("the original buffer was returned");
    assert_eq!(buf, data, "the operation wrote its result into the reused buffer");
}

fn assert_no_allocations(
    operation: &str,
    write: impl FnOnce() -> Result<gix::ObjectId>,
    expected: gix::ObjectId,
) -> TestResult {
    let before = ALLOCATIONS.get();
    let actual = write()?;
    let allocations = ALLOCATIONS.get().wrapping_sub(before);
    assert_eq!(actual, expected, "{operation} writes the expected object");
    assert_eq!(allocations, 0, "{operation} reuses an existing buffer");
    Ok(())
}
