use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
use gix_error::{Error, OptionExt};
use gix_error::{ErrorExt, ResultExt, message};

struct CountingAllocator;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

fn record_allocation() {
    let _ = ALLOCATIONS.try_with(|count| count.set(count.get().wrapping_add(1)));
}

// SAFETY: System receives every allocation unchanged. The thread-local counter neither allocates nor unwinds.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        record_allocation();
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn allocations<T>(f: impl FnOnce() -> T) -> (T, usize) {
    let before = ALLOCATIONS.get();
    let value = std::hint::black_box(f());
    (value, ALLOCATIONS.get().wrapping_sub(before))
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn public_error_context_costs_no_more_than_typed_exception_context() {
    for depth in [1, 16, 64] {
        let original = message("leaf").raise_typed();
        let (_, typed) = allocations(|| (0..depth).fold(original, |error, _| error.raise(message("context"))));
        for add_context in [
            |error: Error| error.and_raise(message("context")),
            |error: Error| {
                Err::<(), _>(error)
                    .or_raise(|| message("context"))
                    .expect_err("the failed result gains context")
            },
        ] {
            let original = message("leaf").raise();
            let (_, public) = allocations(|| (0..depth).fold(original, |error, _| add_context(error)));
            assert!(
                public <= typed,
                "{depth} contexts: public errors used {public} allocations, typed exceptions used {typed}"
            );
        }
    }
}

#[cfg(any(feature = "tree-error", not(feature = "auto-chain-error")))]
#[test]
fn an_already_erased_public_error_needs_no_new_allocations() {
    for erase in [
        |error: Error| error.raise_erased(),
        |error: Error| {
            Err::<(), _>(error)
                .or_erased()
                .expect_err("the failed result is erased")
        },
        |error: Error| {
            None::<()>
                .ok_or_raise_erased(|| error)
                .expect_err("the missing value raises the existing error")
        },
        Error::into_exn,
    ] {
        let original = message("leaf").raise_erased().into_error();
        let (_, count) = allocations(|| erase(original));
        assert_eq!(count, 0, "erasure should reuse an already erased frame");
    }
}

#[cfg(all(feature = "auto-chain-error", not(feature = "tree-error")))]
#[test]
fn adding_context_does_not_reconstruct_an_existing_chain() {
    let mut previous = None;
    for depth in [1, 16, 64] {
        let original = (0..depth)
            .fold(message("leaf").raise_typed(), |error, _| {
                error.raise(message("context"))
            })
            .into_error();
        let (_, count) = allocations(|| {
            Err::<(), _>(original)
                .or_raise(|| message("outer context"))
                .expect_err("the failed result gains context")
        });
        if let Some(previous) = previous {
            assert_eq!(
                count, previous,
                "adding context must not allocate for each existing chain node"
            );
        }
        previous = Some(count);
    }
}

#[test]
fn converting_a_public_error_is_an_allocation_free_identity() {
    use gix_error::{Error, OptionExt};

    for convert in [
        |error: Error| error.raise(),
        |error: Error| Err::<(), _>(error).or_error().expect_err("the failure is retained"),
        |error: Error| {
            None::<()>
                .ok_or_raise(|| error)
                .expect_err("the error supplies the missing value")
        },
    ] {
        for original in [message("raised").raise(), Error::from_error(message("native"))] {
            let pointer = std::ptr::from_ref(original.error());
            let diagnostic = format!("{original:?}");
            let (converted, count) = allocations(|| convert(original));
            assert_eq!(count, 0, "conversion reuses either error representation");
            assert!(
                std::ptr::eq(converted.error(), pointer),
                "conversion retains the original allocation"
            );
            assert_eq!(
                format!("{converted:?}"),
                diagnostic,
                "conversion preserves formatting and caller locations"
            );
        }
    }
}
