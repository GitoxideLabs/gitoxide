use std::{
    collections::BTreeSet,
    sync::atomic::{AtomicUsize, Ordering},
};

use bstr::{BStr, BString};
use gix_worktree::Stack;

use crate::{checkout, checkout::entry};

mod reduce {
    use crate::checkout;

    pub struct Reduce<'entry> {
        pub aggregate: super::Outcome<'entry>,
    }

    impl<'entry> gix_features::parallel::Reduce for Reduce<'entry> {
        type Input = Result<super::Outcome<'entry>, checkout::Error>;
        type FeedProduce = ();
        type Output = super::Outcome<'entry>;
        type Error = checkout::Error;

        fn feed(&mut self, item: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
            let item = item?;
            let super::Outcome {
                bytes_written,
                files,
                delayed_symlinks,
                errors,
                collisions,
                delayed_paths_unknown,
                delayed_paths_unprocessed,
            } = item;
            self.aggregate.bytes_written += bytes_written;
            self.aggregate.files += files;
            self.aggregate.delayed_symlinks.extend(delayed_symlinks);
            self.aggregate.errors.extend(errors);
            self.aggregate.collisions.extend(collisions);
            self.aggregate.delayed_paths_unknown.extend(delayed_paths_unknown);
            self.aggregate
                .delayed_paths_unprocessed
                .extend(delayed_paths_unprocessed);

            Ok(())
        }

        fn finalize(self) -> Result<Self::Output, Self::Error> {
            Ok(self.aggregate)
        }
    }
}
pub use reduce::Reduce;

use crate::checkout::entry::DelayedFilteredStream;

#[derive(Default)]
pub struct Outcome<'a> {
    pub collisions: Vec<checkout::Collision>,
    pub errors: Vec<checkout::ErrorRecord>,
    pub delayed_symlinks: Vec<(&'a mut gix_index::Entry, &'a BStr)>,
    // all (immediately) written bytes
    pub bytes_written: u64,
    // the amount of files we processed
    pub files: usize,
    /// Relative paths that the process listed as 'delayed' even though we never passed them.
    pub delayed_paths_unknown: Vec<BString>,
    /// All paths that were left unprocessed, because they were never listed by the process even though we passed them.
    pub delayed_paths_unprocessed: Vec<BString>,
}

#[derive(Clone)]
pub struct Context<Find: Clone> {
    pub objects: Find,
    pub path_cache: Stack,
    pub filters: gix_filter::Pipeline,
    pub buf: Vec<u8>,
    pub options: Options,
}

#[derive(Clone, Copy)]
pub struct Options {
    pub fs: gix_fs::Capabilities,
    pub destination_is_initially_empty: bool,
    pub overwrite_existing: bool,
    pub keep_going: bool,
    pub filter_process_delay: gix_filter::driver::apply::Delay,
}

impl From<&checkout::Options> for Options {
    fn from(opts: &checkout::Options) -> Self {
        Options {
            fs: opts.fs,
            destination_is_initially_empty: opts.destination_is_initially_empty,
            overwrite_existing: opts.overwrite_existing,
            keep_going: opts.keep_going,
            filter_process_delay: opts.filter_process_delay,
        }
    }
}

pub fn process<'entry, Find>(
    entries_with_paths: impl Iterator<Item = (&'entry mut gix_index::Entry, &'entry BStr)>,
    files: &AtomicUsize,
    bytes: &AtomicUsize,
    delayed_filter_results: &mut Vec<DelayedFilteredStream<'entry>>,
    ctx: &mut Context<Find>,
) -> Result<Outcome<'entry>, checkout::Error>
where
    Find: gix_object::Find + Clone,
{
    let mut delayed_symlinks = Vec::new();
    let mut collisions = Vec::new();
    let mut errors = Vec::new();
    let mut bytes_written = 0;
    let mut files_in_chunk = 0;

    for (entry, entry_path) in entries_with_paths {
        // TODO: write test for that
        if entry.flags.contains(gix_index::entry::Flags::SKIP_WORKTREE) {
            files.fetch_add(1, Ordering::Relaxed);
            files_in_chunk += 1;
            continue;
        }

        // Symlinks always have to be delayed on windows as they have to point to something that exists on creation.
        // And even if not, there is a distinction between file and directory symlinks, hence we have to check what the target is
        // before creating it.
        // And to keep things sane, we just do the same on non-windows as well which is similar to what git does and adds some safety
        // around writing through symlinks (even though we handle this).
        // This also means that we prefer content in files over symlinks in case of collisions, which probably is for the better, too.
        if entry.mode == gix_index::entry::Mode::SYMLINK {
            delayed_symlinks.push((entry, entry_path));
            continue;
        }

        match checkout_entry_handle_result(entry, entry_path, &mut errors, &mut collisions, files, bytes, ctx)? {
            entry::Outcome::Written { bytes } => {
                bytes_written += bytes as u64;
                files_in_chunk += 1;
            }
            entry::Outcome::Delayed(delayed) => delayed_filter_results.push(delayed),
        }
    }

    Ok(Outcome {
        bytes_written,
        files: files_in_chunk,
        errors,
        collisions,
        delayed_symlinks,
        delayed_paths_unknown: Vec::new(),
        delayed_paths_unprocessed: Vec::new(),
    })
}

pub fn process_delayed_filter_results<Find>(
    mut delayed_filter_results: Vec<DelayedFilteredStream<'_>>,
    files: &AtomicUsize,
    bytes: &AtomicUsize,
    out: &mut Outcome<'_>,
    ctx: &mut Context<Find>,
) -> Result<(), checkout::Error>
where
    Find: gix_object::Find + Clone,
{
    let Options {
        destination_is_initially_empty,
        overwrite_existing,
        keep_going,
        ..
    } = ctx.options;
    let mut bytes_written = 0;
    let mut delayed_files = 0;
    // Sort by path for fast lookups
    delayed_filter_results.sort_by(|a, b| a.entry_path.cmp(b.entry_path));
    // We process each key and do as the filter process tells us, while collecting data about the overall progress.
    let keys: BTreeSet<_> = delayed_filter_results.iter().map(|d| d.key.clone()).collect();
    let mut unknown_paths = Vec::new();
    for key in keys {
        loop {
            let rela_paths = ctx.filters.driver_state_mut().list_delayed_paths(&key)?;
            if rela_paths.is_empty() {
                break;
            }

            for rela_path in rela_paths {
                let delayed = match delayed_filter_results.binary_search_by(|d| d.entry_path.cmp(rela_path.as_ref())) {
                    Ok(idx) => &mut delayed_filter_results[idx],
                    Err(_) => {
                        if keep_going {
                            unknown_paths.push(rela_path);
                            continue;
                        } else {
                            return Err(checkout::Error::FilterPathUnknown { rela_path });
                        }
                    }
                };
                let mut read = std::io::BufReader::with_capacity(
                    512 * 1024,
                    ctx.filters.driver_state_mut().fetch_delayed(
                        &key,
                        rela_path.as_ref(),
                        gix_filter::driver::Operation::Smudge,
                    )?,
                );
                let (file, set_executable_after_creation) = match entry::open_file(
                    &std::mem::take(&mut delayed.validated_file_path), // mark it as seen, relevant for `unprocessed_paths`
                    destination_is_initially_empty,
                    overwrite_existing,
                    delayed.needs_executable_bit,
                    delayed.entry.mode,
                ) {
                    Ok(res) => res,
                    Err(err) => {
                        if !is_collision(&err, delayed.entry_path, &mut out.collisions, files) {
                            handle_error(err, delayed.entry_path, files, &mut out.errors, ctx.options.keep_going)?;
                        }
                        std::io::copy(&mut read, &mut std::io::sink())?;
                        continue;
                    }
                };
                let mut write = WriteWithProgress {
                    inner: std::io::BufWriter::with_capacity(512 * 1024, file),
                    progress: bytes,
                };
                let actual_bytes = std::io::copy(&mut read, &mut write)?;
                bytes_written += actual_bytes;
                entry::finalize_entry(
                    delayed.entry,
                    write.inner.into_inner().map_err(std::io::IntoInnerError::into_error)?,
                    actual_bytes,
                    set_executable_after_creation,
                )?;
                delayed_files += 1;
                files.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    let unprocessed_paths = delayed_filter_results
        .into_iter()
        .filter_map(|d| (!d.validated_file_path.as_os_str().is_empty()).then(|| d.entry_path.to_owned()))
        .collect();

    if !keep_going && !unknown_paths.is_empty() {
        return Err(checkout::Error::FilterPathsUnprocessed {
            rela_paths: unprocessed_paths,
        });
    }

    out.delayed_paths_unknown = unknown_paths;
    out.delayed_paths_unprocessed = unprocessed_paths;
    out.bytes_written += bytes_written;
    out.files += delayed_files;
    Ok(())
}

pub struct WriteWithProgress<'a, T> {
    pub inner: T,
    pub progress: &'a AtomicUsize,
}

impl<T> std::io::Write for WriteWithProgress<'_, T>
where
    T: std::io::Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.progress
            .fetch_add(written as gix_features::progress::Step, Ordering::SeqCst);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

pub fn checkout_entry_handle_result<'entry, Find>(
    entry: &'entry mut gix_index::Entry,
    entry_path: &'entry BStr,
    errors: &mut Vec<checkout::ErrorRecord>,
    collisions: &mut Vec<checkout::Collision>,
    files: &AtomicUsize,
    bytes: &AtomicUsize,
    Context {
        objects,
        path_cache,
        filters,
        buf,
        options,
    }: &mut Context<Find>,
) -> Result<entry::Outcome<'entry>, checkout::Error>
where
    Find: gix_object::Find + Clone,
{
    let res = entry::checkout(
        entry,
        entry_path,
        entry::Context {
            objects,
            path_cache,
            filters,
            buf,
        },
        *options,
    );
    match res {
        Ok(out) => {
            if let Some(num) = out.as_bytes() {
                bytes.fetch_add(num, Ordering::Relaxed);
                files.fetch_add(1, Ordering::Relaxed);
            }
            Ok(out)
        }
        Err(checkout::Error::Io(err)) if is_collision(&err, entry_path, collisions, files) => {
            Ok(entry::Outcome::Written { bytes: 0 })
        }
        Err(err) => handle_error(err, entry_path, files, errors, options.keep_going)
            .map(|()| entry::Outcome::Written { bytes: 0 }),
    }
}

fn handle_error<E>(
    err: E,
    entry_path: &BStr,
    files: &AtomicUsize,
    errors: &mut Vec<checkout::ErrorRecord>,
    keep_going: bool,
) -> Result<(), E>
where
    E: std::error::Error + Send + Sync + 'static,
{
    if keep_going {
        errors.push(checkout::ErrorRecord {
            path: entry_path.into(),
            error: Box::new(err),
        });
        files.fetch_add(1, Ordering::Relaxed);
        Ok(())
    } else {
        Err(err)
    }
}

fn is_collision(
    err: &std::io::Error,
    entry_path: &BStr,
    collisions: &mut Vec<checkout::Collision>,
    files: &AtomicUsize,
) -> bool {
    if !gix_fs::symlink::is_collision_error(err) {
        return false;
    }
    // We are here because a file existed or was blocked by a directory which shouldn't be possible unless
    // we are on a file insensitive file system.
    gix_features::trace::error!("{entry_path}: collided ({:?})", err.kind());
    collisions.push(checkout::Collision {
        path: entry_path.into(),
        error_kind: err.kind(),
    });
    files.fetch_add(1, Ordering::Relaxed);
    true
}
