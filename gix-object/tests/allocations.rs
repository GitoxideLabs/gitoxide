use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use gix_object::CommitRefIter;

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

#[test]
fn message_lookup_skips_single_line_and_multiline_headers_without_allocating() -> gix_error::TestResult {
    for &hash_kind in gix_hash::Kind::all() {
        let tree_id = hash_kind.empty_tree();
        let parent_id = hash_kind.null();
        let input = format!(
            "tree {tree_id}
parent {parent_id}
author Author <a@example.com> 1 +0000
committer Committer <b@example.com> 2 +0000
encoding UTF-8
x-extra single-line value
gpgsig first line
 second line
mergetag another header
 second line again

  Workspace commit
"
        );
        let from_start = CommitRefIter::from_bytes(input.as_bytes(), hash_kind);
        let mut after_committer = from_start;
        for _ in 0..4 {
            after_committer.next().expect("the fixture has four leading tokens")?;
        }

        for iter in [from_start, after_committer] {
            let before = ALLOCATIONS.get();
            let message = std::hint::black_box(iter).message()?;
            let allocations = ALLOCATIONS.get().wrapping_sub(before);
            assert_eq!(message, "  Workspace commit\n", "the message retains its whitespace");
            assert_eq!(
                allocations, 0,
                "reading the message only needs to validate extra headers"
            );
        }
    }
    Ok(())
}
