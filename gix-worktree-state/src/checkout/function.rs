use gix_error::Result;
use std::sync::atomic::AtomicBool;

use gix_error::{ResultExt, message};
use gix_parallel::in_parallel_with_finalize;
use gix_utils::interrupt;
use gix_worktree::{Stack, stack};

use crate::checkout::{chunk, entry};

/// Checkout the entire `index` into `dir`, and resolve objects found in index entries with `objects` to write their content to their
/// respective path in `dir`.
/// Use `files` to count each fully checked out file, and count the amount written `bytes`. If `should_interrupt` is `true`, the
/// operation will abort.
/// `options` provide a lot of context on how to perform the operation.
///
/// Symlinks are created after regular files. On Windows, unresolved targets are retried while
/// other links are created, allowing directory-link chains to resolve regardless of index order.
/// Once a pass makes no progress, remaining targets become file links.
///
/// ### Handling the return value
///
/// Note that interruption still produce an `Ok(…)` value, so the caller should look at `should_interrupt` to communicate the outcome.
///
pub fn checkout<Find>(
    index: &mut gix_index::State,
    dir: impl Into<std::path::PathBuf>,
    objects: Find,
    files: &dyn gix_utils::progress::Count,
    bytes: &dyn gix_utils::progress::Count,
    should_interrupt: &AtomicBool,
    options: crate::checkout::Options,
) -> Result<crate::checkout::Outcome>
where
    Find: gix_object::Find + Send + Clone,
{
    let paths = index.take_path_backing();
    let res = checkout_inner(
        index,
        &paths,
        None,
        dir,
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
pub(super) fn checkout_inner<Find>(
    index: &mut gix_index::State,
    paths: &gix_index::PathStorage,
    selected_entries: Option<&[usize]>,
    dir: impl Into<std::path::PathBuf>,
    objects: Find,
    files: &dyn gix_utils::progress::Count,
    bytes: &dyn gix_utils::progress::Count,
    should_interrupt: &AtomicBool,
    mut options: crate::checkout::Options,
) -> Result<crate::checkout::Outcome>
where
    Find: gix_object::Find + Send + Clone,
{
    let num_files = files.counter();
    let num_bytes = bytes.counter();
    let dir = dir.into();
    let (chunk_size, thread_limit, num_threads) = gix_parallel::optimize_chunk_size_and_thread_limit(
        100,
        selected_entries.map_or(index.entries().len(), <[usize]>::len).into(),
        options.thread_limit(),
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

    let entries_with_paths = interrupt::Iter::new(
        index
            .entries_mut_with_paths_in(paths)
            .enumerate()
            .filter_map(|(position, entry)| {
                selected_entries
                    .is_none_or(|selected| selected.binary_search(&position).is_ok())
                    .then_some(entry)
            }),
        should_interrupt,
    );
    let chunk::Outcome {
        mut collisions,
        mut errors,
        mut bytes_written,
        files: files_updated,
        mut delayed_symlinks,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    } = if num_threads == 1 {
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
        in_parallel_with_finalize(
            gix_utils::iter::Chunks {
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
                ctx.filters
                    .driver_state_mut()
                    .shutdown(gix_filter::driver::shutdown::Mode::WaitForProcesses)
                    .or_raise(|| message("Could not shut down filter processes"))?;
                Ok(out)
            },
            chunk::Reduce {
                aggregate: Default::default(),
            },
        )?
    };

    while !delayed_symlinks.is_empty() {
        let previous_len = delayed_symlinks.len();
        let mut retry = Vec::new();
        for (entry, entry_path) in interrupt::Iter::new(delayed_symlinks.into_iter(), should_interrupt) {
            match chunk::checkout_entry_handle_result(
                entry,
                entry_path,
                &mut errors,
                &mut collisions,
                &num_files,
                &num_bytes,
                &mut ctx,
            )? {
                entry::Outcome::Written { bytes } => bytes_written += bytes as u64,
                entry::Outcome::DelayedSymlink { entry, entry_path } => retry.push((entry, entry_path)),
                entry::Outcome::Delayed(_) => unreachable!("symlinks are never filtered"),
            }
        }
        if retry.len() == previous_len {
            // Missing targets and cycles cannot make progress. Create the remaining links
            // using the file fallback, without replacing any links we already created.
            ctx.options.delay_symlinks = false;
        }
        delayed_symlinks = retry;
    }

    ctx.filters
        .driver_state_mut()
        .shutdown(gix_filter::driver::shutdown::Mode::WaitForProcesses)
        .or_raise(|| message("Could not shut down filter processes"))?;

    Ok(crate::checkout::Outcome {
        files_updated,
        collisions,
        errors,
        bytes_written,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    })
}
