use std::{ffi::OsStr, io, path::Path, str::FromStr, time::Instant};

use anyhow::anyhow;
use gix::{
    Count, NestedProgress, Progress, hash, hash::ObjectId, interrupt, objs::bstr::ByteVec, odb::pack,
    parallel::InOrderIter, prelude::Finalize, progress, traverse,
};

use crate::OutputFormat;

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

#[derive(Default, Eq, PartialEq, Debug, Clone)]
pub enum ObjectExpansion {
    #[default]
    None,
    TreeTraversal,
    TreeDiff,
}

impl ObjectExpansion {
    pub fn variants() -> &'static [&'static str] {
        &["none", "tree-traversal", "tree-diff"]
    }
}

impl FromStr for ObjectExpansion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use ObjectExpansion::*;
        let slc = s.to_ascii_lowercase();
        Ok(match slc.as_str() {
            "none" => None,
            "tree-traversal" => TreeTraversal,
            "tree-diff" => TreeDiff,
            _ => return Err("invalid value".into()),
        })
    }
}

impl From<ObjectExpansion> for pack::data::output::count::objects::ObjectExpansion {
    fn from(v: ObjectExpansion) -> Self {
        use pack::data::output::count::objects::ObjectExpansion::*;
        match v {
            ObjectExpansion::None => AsIs,
            ObjectExpansion::TreeTraversal => TreeContents,
            ObjectExpansion::TreeDiff => TreeAdditionsComparedToAncestor,
        }
    }
}

/// A general purpose context for many operations provided here
pub struct Context<W> {
    /// The way input objects should be handled
    pub expansion: ObjectExpansion,
    /// If `Some(threads)`, use this amount of `threads` to accelerate the counting phase at the cost of losing
    /// determinism as the order of objects during expansion changes with multiple threads unless no expansion is performed.
    /// In the latter case, this flag has no effect.
    /// If `None`, counting will only use one thread and thus yield the same sequence of objects in any case.
    pub nondeterministic_thread_count: Option<usize>,
    /// If true, delta objects may refer to their base as reference, allowing it not to be included in the created back.
    /// Otherwise these have to be recompressed in order to make the pack self-contained.
    pub thin: bool,
    /// If set, don't use more than this amount of threads.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
    pub thread_limit: Option<usize>,
    /// If set, statistics about the operation will be written to the output stream.
    pub statistics: Option<OutputFormat>,
    /// The size of the cache storing fully decoded delta objects. This can greatly speed up pack decoding by reducing the length of delta
    /// chains. Note that caches also incur a cost and poorly used caches may reduce overall performance.
    /// This is a total, shared among all threads if `thread_limit` permits.
    ///
    /// If 0, the cache is disabled entirely.
    pub pack_cache_size_in_bytes: usize,
    /// The size of the cache to store full objects by their ID, bypassing any lookup in the object database.
    /// Note that caches also incur a cost and poorly used caches may reduce overall performance.
    ///
    /// This is a total, shared among all threads if `thread_limit` permits.
    /// Only used when known to be effective, namely when `expansion == ObjectExpansion::TreeDiff`.
    pub object_cache_size_in_bytes: usize,
    /// The output stream for use of additional information
    pub out: W,
}

pub fn create<W, P>(
    repository_path: impl AsRef<Path>,
    tips: impl IntoIterator<Item = impl AsRef<OsStr>>,
    input: Option<impl io::BufRead + Send + 'static>,
    output_directory: Option<impl AsRef<Path>>,
    mut progress: P,
    Context {
        expansion,
        nondeterministic_thread_count,
        thin,
        thread_limit,
        statistics,
        pack_cache_size_in_bytes,
        object_cache_size_in_bytes,
        mut out,
    }: Context<W>,
) -> anyhow::Result<()>
where
    W: std::io::Write,
    P: NestedProgress,
    P::SubProgress: 'static,
{
    type ObjectIdIter = dyn Iterator<Item = Result<ObjectId, Box<dyn std::error::Error + Send + Sync>>> + Send;

    let repo = gix::discover(repository_path)?.into_sync();
    progress.init(Some(2), progress::steps());
    let tips = tips.into_iter();
    let make_cancellation_err = || anyhow!("Cancelled by user");
    let (mut handle, mut input): (_, Box<ObjectIdIter>) = match input {
        None => {
            let mut progress = progress.add_child("traversing");
            progress.init(None, progress::count("commits"));
            let tips = tips
                .map({
                    let easy = repo.to_thread_local();
                    move |tip| {
                        ObjectId::from_hex(&Vec::from_os_str_lossy(tip.as_ref())).or_else(|_| {
                            easy.find_reference(tip.as_ref())
                                .map_err(anyhow::Error::from)
                                .and_then(|r| r.into_fully_peeled_id().map(gix::Id::detach).map_err(Into::into))
                        })
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            let handle = repo.objects.into_shared_arc().to_cache_arc();
            let iter = Box::new(
                traverse::commit::Simple::new(tips, handle.clone())
                    .map(|res| res.map_err(|err| Box::new(err) as Box<_>).map(|c| c.id))
                    .inspect(move |_| progress.inc()),
            );
            (handle, iter)
        }
        Some(input) => {
            let mut progress = progress.add_child("iterating");
            progress.init(None, progress::count("objects"));
            let handle = repo.objects.into_shared_arc().to_cache_arc();
            (
                handle,
                Box::new(
                    input
                        .lines()
                        .map(|hex_id| {
                            hex_id
                                .map_err(|err| Box::new(err) as Box<_>)
                                .and_then(|hex_id| ObjectId::from_hex(hex_id.as_bytes()).map_err(Into::into))
                        })
                        .inspect(move |_| progress.inc()),
                ),
            )
        }
    };

    let mut stats = Statistics::default();
    let chunk_size = 1000; // What's a good value for this?
    let counts = {
        let mut progress = progress.add_child("counting");
        progress.init(None, progress::count("objects"));
        let may_use_multiple_threads =
            nondeterministic_thread_count.is_some() || matches!(expansion, ObjectExpansion::None);
        let thread_limit = if may_use_multiple_threads {
            nondeterministic_thread_count.or(thread_limit)
        } else {
            Some(1)
        };
        if nondeterministic_thread_count.is_some() && !may_use_multiple_threads {
            progress.fail("Cannot use multi-threaded counting in tree-diff object expansion mode as it may yield way too many objects.".into());
        }
        let (_, _, thread_count) = gix::parallel::optimize_chunk_size_and_thread_limit(50, None, thread_limit, None);
        let progress = progress::ThroughputOnDrop::new(progress);

        {
            let per_thread_object_pack_size = pack_cache_size_in_bytes / thread_count;
            if per_thread_object_pack_size >= 10_000 {
                handle.set_pack_cache(move || {
                    Box::new(pack::cache::lru::MemoryCappedHashmap::new(per_thread_object_pack_size))
                });
            }
            if matches!(expansion, ObjectExpansion::TreeDiff) {
                handle.set_object_cache(move || {
                    let per_thread_object_cache_size = object_cache_size_in_bytes / thread_count;
                    Box::new(pack::cache::object::MemoryCappedHashmap::new(
                        per_thread_object_cache_size,
                    ))
                });
            }
        }
        let input_object_expansion = expansion.into();
        handle.prevent_pack_unload();
        handle.ignore_replacements = true;
        let (mut counts, count_stats) = if may_use_multiple_threads {
            pack::data::output::count::objects(
                handle.clone(),
                input,
                &progress,
                &interrupt::IS_INTERRUPTED,
                pack::data::output::count::objects::Options {
                    thread_limit,
                    chunk_size,
                    input_object_expansion,
                },
            )?
        } else {
            pack::data::output::count::objects_unthreaded(
                &handle,
                &mut input,
                &progress,
                &interrupt::IS_INTERRUPTED,
                input_object_expansion,
            )?
        };
        stats.counts = count_stats;
        counts.shrink_to_fit();
        counts
    };

    progress.inc();
    let num_objects = counts.len();
    let mut in_order_entries = {
        let progress = progress.add_child("creating entries");
        InOrderIter::from(pack::data::output::entry::iter_from_counts(
            counts,
            handle,
            Box::new(progress),
            pack::data::output::entry::iter_from_counts::Options {
                thread_limit,
                mode: pack::data::output::entry::iter_from_counts::Mode::PackCopyAndBaseObjects,
                allow_thin_pack: thin,
                chunk_size,
                version: Default::default(),
            },
        ))
    };

    let mut entries_progress = progress.add_child("consuming");
    entries_progress.init(Some(num_objects), progress::count("entries"));
    let mut write_progress = progress.add_child("writing");
    write_progress.init(None, progress::bytes());
    let start = Instant::now();

    let mut named_tempfile_store: Option<tempfile::NamedTempFile> = None;
    let mut sink_store: std::io::Sink;
    let (mut pack_file, output_directory): (&mut dyn std::io::Write, Option<_>) = match output_directory {
        Some(dir) => {
            named_tempfile_store = Some(tempfile::NamedTempFile::new_in(dir.as_ref())?);
            (named_tempfile_store.as_mut().expect("packfile just set"), Some(dir))
        }
        None => {
            sink_store = std::io::sink();
            (&mut sink_store, None)
        }
    };
    let mut interruptible_output_iter = interrupt::Iter::new(
        pack::data::output::bytes::FromEntriesIter::new(
            in_order_entries.by_ref().inspect(|e| {
                if let Ok(entries) = e {
                    entries_progress.inc_by(entries.len());
                }
            }),
            &mut pack_file,
            num_objects as u32,
            pack::data::Version::default(),
            hash::Kind::default(),
        ),
        make_cancellation_err,
    );
    for io_res in interruptible_output_iter.by_ref() {
        let written = io_res??;
        write_progress.inc_by(written as usize);
    }

    let hash = interruptible_output_iter
        .into_inner()
        .digest()
        .expect("iteration is done");
    let pack_name = format!("{hash}.pack");
    if let (Some(pack_file), Some(dir)) = (named_tempfile_store.take(), output_directory) {
        pack_file.persist(dir.as_ref().join(pack_name))?;
    } else {
        writeln!(out, "{pack_name}")?;
    }
    stats.entries = in_order_entries.inner.finalize()?;

    write_progress.show_throughput(start);
    entries_progress.show_throughput(start);

    if let Some(format) = statistics {
        print(stats, format, out)?;
    }
    progress.inc();
    Ok(())
}

fn print(stats: Statistics, format: OutputFormat, out: impl std::io::Write) -> anyhow::Result<()> {
    match format {
        OutputFormat::Human => human_output(stats, out).map_err(Into::into),
        #[cfg(feature = "serde")]
        OutputFormat::Json => serde_json::to_writer_pretty(out, &stats).map_err(Into::into),
    }
}

fn human_output(
    Statistics {
        counts:
            pack::data::output::count::objects::Outcome {
                input_objects,
                expanded_objects,
                decoded_objects,
                total_objects,
            },
        entries:
            pack::data::output::entry::iter_from_counts::Outcome {
                decoded_and_recompressed_objects,
                missing_objects,
                objects_copied_from_pack,
                ref_delta_objects,
            },
    }: Statistics,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    let width = 30;
    writeln!(out, "counting phase")?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}",
        "input objects", input_objects,
        "expanded objects", expanded_objects,
        "decoded objects", decoded_objects,
        "total objects", total_objects,
        width = width
    )?;
    writeln!(out, "generation phase")?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}\n\t{:<width$} {}",
        "decoded and recompressed", decoded_and_recompressed_objects,
        "pack-to-pack copies", objects_copied_from_pack,
        "ref-delta-objects", ref_delta_objects,
        "missing objects", missing_objects,
        width = width
    )?;
    Ok(())
}

#[derive(Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Statistics {
    counts: pack::data::output::count::objects::Outcome,
    entries: pack::data::output::entry::iter_from_counts::Outcome,
}

pub mod input_iteration {
    use gix::{hash, traverse};
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("input objects couldn't be iterated completely")]
        Iteration(#[from] traverse::commit::simple::Error),
        #[error("An error occurred while reading hashes from standard input")]
        InputLinesIo(#[from] std::io::Error),
        #[error("Could not decode hex hash provided on standard input")]
        HashDecode(#[from] hash::decode::Error),
    }
}
