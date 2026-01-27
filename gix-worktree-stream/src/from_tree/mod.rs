use gix_error::{message, ResultExt};
use gix_object::{bstr::BStr, FindExt};
use std::io::Write;

use crate::{entry, entry::Error, protocol, AdditionalEntry, SharedErrorSlot, Stream};

/// Use `objects` to traverse `tree` and fetch the contained blobs to return as [`Stream`], which makes them queryable
/// on demand with support for streaming each entry.
///
/// `pipeline` is used to convert blobs to their worktree representation, and `attributes` is used to read
/// the `export-ignore` attribute. If set on a directory or blob, it won't be added to the archive.
///
/// ### Types of entries in stream
///
/// We only return blobs (with or without executable), which may be symlinks in which case their content will
/// be target of the symlink.
/// Directories are never returned, but maybe added by the caller via [Stream::add_entry()].
///
/// ### Progress and interruptions
///
/// For per-file progress, integrate progress handling into the calls of [`Stream::next_entry()`] as that
/// correlates blobs.
/// Additional interrupt handling can be wrapped around the `Read` implementation of each [`Entry`][crate::Entry].
/// For progress on bytes-written, integrate progress reporting when consuming the stream.
/// Further it's possible to drop the returned [`Stream`] to halt all operation.
///
/// ### Threaded Operation
///
/// This function spawns a thread that will access the tree data in the background, synchronized through
/// `Stream` so that it will not be faster than the consumer, with at most one file in flight at any time.
///
/// ### Limitations
///
/// * `export-subst` is not support, as it requires the entire formatting engine of `git log`.
pub fn from_tree<Find, E>(
    tree: gix_hash::ObjectId,
    objects: Find,
    pipeline: gix_filter::Pipeline,
    attributes: impl FnMut(&BStr, gix_object::tree::EntryMode, &mut gix_attributes::search::Outcome) -> Result<(), E>
        + Send
        + 'static,
) -> Stream
where
    Find: gix_object::Find + Clone + Send + 'static,
    E: std::error::Error + Send + Sync + 'static,
{
    let (stream, mut write, additional_entries) = Stream::new();
    std::thread::spawn({
        let slot = stream.err.clone();
        move || {
            if let Err(err) = run(
                tree,
                objects,
                pipeline,
                attributes,
                &mut write,
                slot.clone(),
                additional_entries,
            ) {
                {
                    let mut slot = slot.lock();
                    if slot.is_none() {
                        *slot = Some(err);
                    } else {
                        drop(slot);
                        write.channel.send(Err(std::io::Error::other(err))).ok();
                    }
                }
            }
        }
    });
    stream
}

fn run<Find, E>(
    tree: gix_hash::ObjectId,
    objects: Find,
    mut pipeline: gix_filter::Pipeline,
    mut attributes: impl FnMut(&BStr, gix_object::tree::EntryMode, &mut gix_attributes::search::Outcome) -> Result<(), E>
        + Send
        + 'static,
    out: &mut gix_features::io::pipe::Writer,
    err: SharedErrorSlot,
    additional_entries: std::sync::mpsc::Receiver<AdditionalEntry>,
) -> Result<(), Error>
where
    Find: gix_object::Find + Clone,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut buf = Vec::new();
    let tree_iter = objects.find_tree_iter(tree.as_ref(), &mut buf)?;
    if pipeline.driver_context_mut().treeish.is_none() {
        pipeline.driver_context_mut().treeish = Some(tree);
    }

    let mut attrs = gix_attributes::search::Outcome::default();
    attrs.initialize_with_selection(&Default::default(), Some("export-ignore"));
    let mut dlg = traverse::Delegate {
        out,
        err,
        pipeline,
        attrs,
        objects: objects.clone(),
        fetch_attributes: move |a: &BStr, b: gix_object::tree::EntryMode, c: &mut gix_attributes::search::Outcome| {
            attributes(a, b, c).or_raise(|| message!("Could not query attributes for path \"{a}\""))
        },
        path_deque: Default::default(),
        path: Default::default(),
        buf: Vec::with_capacity(1024),
    };
    gix_traverse::tree::breadthfirst(
        tree_iter,
        gix_traverse::tree::breadthfirst::State::default(),
        &objects,
        &mut dlg,
    )?;

    for entry in additional_entries {
        protocol::write_entry_header_and_path(
            entry.relative_path.as_ref(),
            &entry.id,
            entry.mode,
            entry.source.len(),
            out,
        )?;
        // pipe writer always writes all in one go.
        #[allow(clippy::unused_io_amount)]
        match entry.source {
            entry::Source::Memory(buf) => out.write(&buf).map(|_| ()),
            entry::Source::Null => out.write(&[]).map(|_| ()),
            entry::Source::Path(path) => {
                let file = std::fs::File::open(path)?;
                protocol::write_stream(&mut buf, file, out)
            }
        }?;
    }
    Ok(())
}

mod traverse;
