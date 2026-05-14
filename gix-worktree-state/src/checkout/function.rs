use std::sync::atomic::AtomicBool;

use gix_features::{interrupt, parallel::in_parallel_with_finalize};
use gix_worktree::{Stack, stack};

use crate::checkout::chunk;

/// Checkout the entire `index` into `dir`, and resolve objects found in index entries with `objects` to write their content to their
/// respective path in `dir`.
/// Use `files` to count each fully checked out file, and count the amount written `bytes`. If `should_interrupt` is `true`, the
/// operation will abort.
/// `options` provide a lot of context on how to perform the operation.
///
/// ### Handling the return value
///
/// Note that interruption still produce an `Ok(…)` value, so the caller should look at `should_interrupt` to communicate the outcome.
///
#[allow(clippy::too_many_arguments)]
pub fn checkout<Find>(
    index: &mut gix_index::State,
    dir: impl Into<std::path::PathBuf>,
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
    let res = checkout_inner(index, &paths, dir, objects, files, bytes, should_interrupt, options);
    index.return_path_backing(paths);
    res
}

#[allow(clippy::too_many_arguments)]
fn checkout_inner<Find>(
    index: &mut gix_index::State,
    paths: &gix_index::PathStorage,
    dir: impl Into<std::path::PathBuf>,
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
    let (chunk_size, thread_limit, num_threads) = gix_features::parallel::optimize_chunk_size_and_thread_limit(
        100,
        index.entries().len().into(),
        options.thread_limit,
        None,
    );

    let mut ctx = chunk::Context {
        buf: Vec::new(),
        options: (&options).into(),
        path_cache: Stack::from_state_and_ignore_case(
            dir,
            options.fs.ignore_case,
            stack::State::for_checkout(
                options.overwrite_existing,
                options.validate,
                std::mem::take(&mut options.attributes),
            ),
            index,
            paths,
        ),
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

    Ok(crate::checkout::Outcome {
        files_updated,
        collisions,
        errors,
        bytes_written,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    })
}
