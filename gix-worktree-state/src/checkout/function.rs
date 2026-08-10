use std::sync::atomic::AtomicBool;

use gix_features::{interrupt, parallel::in_parallel_with_finalize};
use gix_worktree::{Stack, stack};

use crate::checkout::chunk;

/// Checkout the entire `index` into `dir`, and resolve objects found in index entries with `objects` to write their content to their
/// respective path in `dir`.
/// If `previous_index` is `Some`, remove files that are tracked in it but absent from `index` before writing anything,
/// along with directories that become empty that way, similar to how `git checkout` removes files when switching revisions.
/// Untracked files are never removed. Files that appear to have changed on disk since `previous_index` was written
/// are not removed either, but instead cause a [`RemovalPrevented`](crate::checkout::Error::RemovalPrevented) error,
/// unless [`overwrite_existing`](crate::checkout::Options::overwrite_existing) is set.
/// Pass `None` to leave everything but the checked out files alone, which is all that's needed when
/// checking out into an empty directory.
/// Use `files` to count each fully checked out file, and count the amount written `bytes`. If `should_interrupt` is `true`, the
/// operation will abort.
/// `options` provide a lot of context on how to perform the operation.
///
/// ### Handling the return value
///
/// Note that interruption still produce an `Ok(…)` value, so the caller should look at `should_interrupt` to communicate the outcome.
///
#[expect(clippy::too_many_arguments)]
pub fn checkout<Find>(
    index: &mut gix_index::State,
    dir: impl Into<std::path::PathBuf>,
    previous_index: Option<&gix_index::State>,
    objects: Find,
    files: &dyn gix_features::progress::Count,
    bytes: &dyn gix_features::progress::Count,
    should_interrupt: &AtomicBool,
    options: crate::checkout::Options,
) -> Result<crate::checkout::Outcome, crate::checkout::Error>
where
    Find: gix_object::Find + Send + Clone,
{
    let paths = index.take_path_backing();
    let res = checkout_inner(
        index,
        &paths,
        dir,
        previous_index,
        objects,
        files,
        bytes,
        should_interrupt,
        options,
    );
    index.return_path_backing(paths);
    res
}

#[expect(clippy::too_many_arguments)]
fn checkout_inner<Find>(
    index: &mut gix_index::State,
    paths: &gix_index::PathStorage,
    dir: impl Into<std::path::PathBuf>,
    previous_index: Option<&gix_index::State>,
    objects: Find,
    files: &dyn gix_features::progress::Count,
    bytes: &dyn gix_features::progress::Count,
    should_interrupt: &AtomicBool,
    mut options: crate::checkout::Options,
) -> Result<crate::checkout::Outcome, crate::checkout::Error>
where
    Find: gix_object::Find + Send + Clone,
{
    let num_files = files.counter();
    let num_bytes = bytes.counter();
    let dir = dir.into();

    let mut removal_errors = Vec::new();
    let files_removed = match previous_index {
        Some(previous_index) => super::removal::remove_stale_entries(
            &dir,
            previous_index,
            index,
            paths,
            &mut removal_errors,
            should_interrupt,
            &options,
        )?,
        None => 0,
    };
    let (chunk_size, thread_limit, num_threads) = gix_features::parallel::optimize_chunk_size_and_thread_limit(
        100,
        index.entries().len().into(),
        options.thread_limit,
        None,
    );

    let mut path_cache = Stack::from_state_and_ignore_case(
        dir,
        options.fs.ignore_case,
        stack::State::for_checkout(
            options.overwrite_existing,
            options.validate,
            std::mem::take(&mut options.attributes),
        ),
        index,
        paths,
    );
    if !options.destination_is_initially_empty {
        path_cache.enable_terminal_symlink_check();
    }
    let mut ctx = chunk::Context {
        buf: Vec::new(),
        options: (&options).into(),
        path_cache,
        filters: options.filters,
        objects,
    };

    let chunk::Outcome {
        mut collisions,
        mut errors,
        mut bytes_written,
        files: files_updated,
        delayed_symlinks,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    } = if num_threads == 1 {
        let entries_with_paths = interrupt::Iter::new(index.entries_mut_with_paths_in(paths), should_interrupt);
        let mut delayed_filter_results = Vec::new();
        let mut out = chunk::process(
            entries_with_paths,
            &num_files,
            &num_bytes,
            &mut delayed_filter_results,
            &mut ctx,
        )?;
        chunk::process_delayed_filter_results(delayed_filter_results, &num_files, &num_bytes, &mut out, &mut ctx)?;
        out
    } else {
        let entries_with_paths = interrupt::Iter::new(index.entries_mut_with_paths_in(paths), should_interrupt);
        in_parallel_with_finalize(
            gix_features::iter::Chunks {
                inner: entries_with_paths,
                size: chunk_size,
            },
            thread_limit,
            {
                let ctx = ctx.clone();
                move |_| (Vec::new(), ctx)
            },
            |chunk, (delayed_filter_results, ctx)| {
                chunk::process(chunk.into_iter(), &num_files, &num_bytes, delayed_filter_results, ctx)
            },
            |(delayed_filter_results, mut ctx)| {
                let mut out = chunk::Outcome::default();
                chunk::process_delayed_filter_results(
                    delayed_filter_results,
                    &num_files,
                    &num_bytes,
                    &mut out,
                    &mut ctx,
                )?;
                Ok(out)
            },
            chunk::Reduce {
                aggregate: Default::default(),
            },
        )?
    };

    for (entry, entry_path) in delayed_symlinks {
        bytes_written += chunk::checkout_entry_handle_result(
            entry,
            entry_path,
            &mut errors,
            &mut collisions,
            &num_files,
            &num_bytes,
            &mut ctx,
        )?
        .as_bytes()
        .expect("only symlinks are delayed here, they are never filtered (or delayed again)")
            as u64;
    }

    removal_errors.extend(errors);
    Ok(crate::checkout::Outcome {
        files_updated,
        files_removed,
        collisions,
        errors: removal_errors,
        bytes_written,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    })
}
