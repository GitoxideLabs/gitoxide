use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::bail;
use gix::{NestedProgress, Progress, objs::find::Error, worktree::state::checkout};

use crate::{
    index,
    index::{Options, parse_file},
};

pub fn checkout_exclusive(
    index_path: impl AsRef<Path>,
    dest_directory: impl AsRef<Path>,
    repo: Option<PathBuf>,
    mut err: impl std::io::Write,
    mut progress: impl NestedProgress,
    should_interrupt: &AtomicBool,
    index::checkout_exclusive::Options {
        index: Options { object_hash, .. },
        empty_files,
        keep_going,
        thread_limit,
    }: index::checkout_exclusive::Options,
) -> anyhow::Result<()> {
    let repo = repo.map(gix::discover).transpose()?;

    let dest_directory = dest_directory.as_ref();
    if dest_directory.exists() {
        bail!(
            "Refusing to checkout index into existing directory '{}' - remove it and try again",
            dest_directory.display()
        )
    }
    std::fs::create_dir_all(dest_directory)?;

    let mut index = parse_file(index_path, object_hash)?;

    let mut num_skipped = 0;
    let maybe_symlink_mode = if !empty_files && repo.is_some() {
        gix::index::entry::Mode::DIR
    } else {
        gix::index::entry::Mode::SYMLINK
    };
    for entry in index.entries_mut().iter_mut().filter(|e| {
        e.mode
            .contains(maybe_symlink_mode | gix::index::entry::Mode::DIR | gix::index::entry::Mode::COMMIT)
    }) {
        entry.flags.insert(gix::index::entry::Flags::SKIP_WORKTREE);
        num_skipped += 1;
    }
    if num_skipped > 0 {
        progress.info(format!("Skipping {num_skipped} DIR/SYMLINK/COMMIT entries"));
    }

    let opts = gix::worktree::state::checkout::Options {
        fs: gix::fs::Capabilities::probe(dest_directory),

        destination_is_initially_empty: true,
        overwrite_existing: false,
        keep_going,
        thread_limit,
        filters: repo
            .as_ref()
            .and_then(|repo| repo.filter_pipeline(None).ok().map(|t| t.0.into_parts().0))
            .unwrap_or_default(),
        ..Default::default()
    };

    let mut files = progress.add_child("checkout");
    let mut bytes = progress.add_child("writing");

    let entries_for_checkout = index.entries().len() - num_skipped;
    files.init(Some(entries_for_checkout), gix::progress::count("files"));
    bytes.init(None, gix::progress::bytes());

    let start = std::time::Instant::now();
    let no_repo = repo.is_none();
    let checkout::Outcome {
        errors,
        collisions,
        files_updated,
        bytes_written,
        delayed_paths_unknown,
        delayed_paths_unprocessed,
    } = match repo {
        Some(repo) => gix::worktree::state::checkout(
            &mut index,
            dest_directory,
            EmptyOrDb {
                empty_files,
                db: repo.objects.into_arc()?,
            },
            &files,
            &bytes,
            should_interrupt,
            opts,
        ),
        None => gix::worktree::state::checkout(
            &mut index,
            dest_directory,
            Empty,
            &files,
            &bytes,
            should_interrupt,
            opts,
        ),
    }?;

    files.show_throughput(start);
    bytes.show_throughput(start);

    progress.done(format!(
        "Created {} {} files{} ({})",
        files_updated,
        if no_repo { "empty" } else { Default::default() },
        if should_interrupt.load(Ordering::Relaxed) {
            {
                format!(
                    " of {}",
                    entries_for_checkout
                        .saturating_sub(errors.len() + collisions.len() + delayed_paths_unprocessed.len())
                )
            }
        } else {
            Default::default()
        },
        gix::progress::bytes()
            .unwrap()
            .display(bytes_written as usize, None, None)
    ));

    let mut messages = Vec::new();
    if !errors.is_empty() {
        messages.push(format!("kept going through {} errors(s)", errors.len()));
        for record in errors {
            writeln!(err, "{}: {}", record.path, record.error).ok();
        }
    }
    if !collisions.is_empty() {
        messages.push(format!("encountered {} collision(s)", collisions.len()));
        for col in collisions {
            writeln!(err, "{}: collision ({:?})", col.path, col.error_kind).ok();
        }
    }
    if !delayed_paths_unknown.is_empty() {
        messages.push(format!(
            "A delayed process provided us with {} paths we never sent to it",
            delayed_paths_unknown.len()
        ));
        for unknown in delayed_paths_unknown {
            writeln!(err, "{unknown}: unknown").ok();
        }
    }
    if !delayed_paths_unprocessed.is_empty() {
        messages.push(format!(
            "A delayed process forgot to process {} paths",
            delayed_paths_unprocessed.len()
        ));
        for unprocessed in delayed_paths_unprocessed {
            writeln!(err, "{unprocessed}: unprocessed and forgotten").ok();
        }
    }
    if !messages.is_empty() {
        bail!(
            "One or more errors occurred - checkout is incomplete: {}",
            messages.join(", ")
        );
    }
    Ok(())
}

#[derive(Clone)]
struct EmptyOrDb<Find> {
    empty_files: bool,
    db: Find,
}

impl<Find> gix::objs::Find for EmptyOrDb<Find>
where
    Find: gix::objs::Find,
{
    fn try_find<'a>(&self, id: &gix::oid, buf: &'a mut Vec<u8>) -> Result<Option<gix::objs::Data<'a>>, Error> {
        if self.empty_files {
            // We always want to query the ODB here…
            let Some(kind) = self.db.try_find(id, buf)?.map(|d| d.kind) else {
                return Ok(None);
            };
            buf.clear();
            // …but write nothing
            Ok(Some(gix::objs::Data {
                kind,
                object_hash: id.kind(),
                data: buf,
            }))
        } else {
            self.db.try_find(id, buf)
        }
    }
}

#[derive(Clone)]
struct Empty;

impl gix::objs::Find for Empty {
    fn try_find<'a>(&self, id: &gix::oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix::objs::Data<'a>>, Error> {
        buffer.clear();
        Ok(Some(gix::objs::Data {
            kind: gix::object::Kind::Blob,
            object_hash: id.kind(),
            data: buffer,
        }))
    }
}
