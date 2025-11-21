use std::{io, sync::atomic::Ordering};

use anyhow::bail;

use crate::OutputFormat;

#[cfg_attr(not(feature = "serde"), allow(unused_variables))]
pub fn info(
    repo: gix::Repository,
    format: OutputFormat,
    out: impl io::Write,
    mut err: impl io::Write,
) -> anyhow::Result<()> {
    if format == OutputFormat::Human {
        writeln!(err, "Only JSON is implemented - using that instead")?;
    }

    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    pub struct Statistics {
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub path: std::path::PathBuf,
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub object_hash: String,
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub use_multi_pack_index: bool,
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub structure: Vec<gix::odb::store::structure::Record>,
        #[cfg_attr(not(feature = "serde"), allow(dead_code))]
        pub metrics: gix::odb::store::Metrics,
    }

    let store = repo.objects.store_ref();
    let stats = Statistics {
        path: store.path().into(),
        object_hash: store.object_hash().to_string(),
        use_multi_pack_index: store.use_multi_pack_index(),
        structure: store.structure()?,
        metrics: store.metrics(),
    };

    #[cfg(feature = "serde")]
    {
        serde_json::to_writer_pretty(out, &stats)?;
    }

    Ok(())
}

pub mod statistics {
    use crate::OutputFormat;

    pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 0..=3;

    #[derive(Debug, Copy, Clone)]
    pub struct Options {
        pub format: OutputFormat,
        pub thread_limit: Option<usize>,
        /// A debug-flag that triggers looking up the headers of all objects again, but without indices preloaded
        pub extra_header_lookup: bool,
    }
}

#[cfg_attr(not(feature = "serde"), allow(unused_variables))]
pub fn statistics(
    repo: gix::Repository,
    mut progress: impl gix::Progress,
    out: impl io::Write,
    mut err: impl io::Write,
    statistics::Options {
        format,
        thread_limit,
        extra_header_lookup,
    }: statistics::Options,
) -> anyhow::Result<()> {
    use bytesize::ByteSize;
    use gix::odb::{find, HeaderExt};

    if format == OutputFormat::Human {
        writeln!(err, "Only JSON is implemented - using that instead")?;
    }

    progress.init(None, gix::progress::count("objects"));
    progress.set_name("counting".into());
    let counter = progress.counter();
    let start = std::time::Instant::now();

    #[cfg_attr(feature = "serde", derive(serde::Serialize))]
    #[derive(Default)]
    struct Statistics {
        /// All objects that were used to produce these statistics.
        /// Only `Some` if we are doing an extra round of header queries on a repository without loaded indices.
        #[cfg_attr(feature = "serde", serde(skip_serializing))]
        ids: Option<Vec<gix::ObjectId>>,
        total_objects: usize,
        loose_objects: usize,
        packed_objects: usize,
        packed_delta_objects: usize,
        total_delta_chain_length: u64,
        trees: usize,
        trees_size: ByteSize,
        tags: usize,
        tags_size: ByteSize,
        commits: usize,
        commits_size: ByteSize,
        blobs: usize,
        blobs_size: ByteSize,
    }

    impl Statistics {
        fn count(&mut self, kind: gix::object::Kind, size: u64) {
            use gix::object::Kind::*;
            match kind {
                Commit => {
                    self.commits += 1;
                    self.commits_size += size;
                }
                Tree => {
                    self.trees += 1;
                    self.trees_size += size;
                }
                Tag => {
                    self.tags += 1;
                    self.tags_size += size;
                }
                Blob => {
                    self.blobs += 1;
                    self.blobs_size += size;
                }
            }
        }
        fn consume(&mut self, item: gix::odb::find::Header) {
            match item {
                find::Header::Loose { size, kind } => {
                    self.loose_objects += 1;
                    self.count(kind, size);
                }
                find::Header::Packed(packed) => {
                    self.packed_objects += 1;
                    self.packed_delta_objects += usize::from(packed.num_deltas > 0);
                    self.total_delta_chain_length += u64::from(packed.num_deltas);
                    self.count(packed.kind, packed.object_size);
                }
            }
        }
    }

    #[derive(Default)]
    struct Reduce {
        stats: Statistics,
    }

    impl gix::parallel::Reduce for Reduce {
        type Input = Result<Vec<(gix::ObjectId, gix::odb::find::Header)>, anyhow::Error>;
        type FeedProduce = ();
        type Output = Statistics;
        type Error = anyhow::Error;

        fn feed(&mut self, items: Self::Input) -> Result<Self::FeedProduce, Self::Error> {
            for (id, item) in items? {
                self.stats.consume(item);
                if let Some(ids) = self.stats.ids.as_mut() {
                    ids.push(id);
                }
            }
            Ok(())
        }

        fn finalize(mut self) -> Result<Self::Output, Self::Error> {
            self.stats.total_objects = self.stats.loose_objects + self.stats.packed_objects;
            Ok(self.stats)
        }
    }

    let cancelled = || anyhow::anyhow!("Cancelled by user");
    let object_ids = repo.objects.iter()?.filter_map(Result::ok);
    let chunk_size = 1_000;
    let mut stats = if gix::parallel::num_threads(thread_limit) > 1 {
        gix::parallel::in_parallel(
            gix::interrupt::Iter::new(
                gix::features::iter::Chunks {
                    inner: object_ids,
                    size: chunk_size,
                },
                cancelled,
            ),
            thread_limit,
            {
                let objects = repo.objects.clone();
                move |_| (objects.clone().into_inner(), counter)
            },
            |ids, (handle, counter)| {
                let ids = ids?;
                counter.fetch_add(ids.len(), Ordering::Relaxed);
                let out = ids
                    .into_iter()
                    .map(|id| handle.header(id).map(|hdr| (id, hdr)))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(out)
            },
            Reduce {
                stats: Statistics {
                    ids: extra_header_lookup.then(Vec::new),
                    ..Default::default()
                },
            },
        )?
    } else {
        if extra_header_lookup {
            bail!("extra-header-lookup is only meaningful in threaded mode");
        }
        let mut stats = Statistics::default();

        for (count, id) in object_ids.enumerate() {
            if count % chunk_size == 0 && gix::interrupt::is_triggered() {
                return Err(cancelled());
            }
            stats.consume(repo.objects.header(id)?);
            progress.inc();
        }
        stats
    };

    progress.show_throughput(start);

    if let Some(mut ids) = stats.ids.take() {
        // Critical to re-open the repo to assure we don't have any ODB state and start fresh.
        let start = std::time::Instant::now();
        let repo = gix::open_opts(repo.git_dir(), repo.open_options().to_owned())?;
        progress.set_name("re-counting".into());
        progress.init(Some(ids.len()), gix::progress::count("objects"));
        let counter = progress.counter();
        counter.store(0, Ordering::Relaxed);
        let errors = gix::parallel::in_parallel_with_slice(
            &mut ids,
            thread_limit,
            {
                let objects = repo.objects.clone();
                move |_| (objects.clone().into_inner(), counter, false)
            },
            |id, (odb, counter, has_error), _threads_left, _stop_everything| -> anyhow::Result<()> {
                counter.fetch_add(1, Ordering::Relaxed);
                if let Err(_err) = odb.header(id) {
                    *has_error = true;
                    gix::trace::error!(err = ?_err, "Object that is known to be present wasn't found");
                }
                Ok(())
            },
            || Some(std::time::Duration::from_millis(100)),
            |(_, _, has_error)| has_error,
        )?;

        progress.show_throughput(start);
        if errors.contains(&true) {
            bail!("At least one object couldn't be looked up even though it must exist");
        }
    }

    #[cfg(feature = "serde")]
    {
        serde_json::to_writer_pretty(out, &stats)?;
    }

    Ok(())
}

pub fn entries(repo: gix::Repository, format: OutputFormat, mut out: impl io::Write) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("Only human output format is supported at the moment");
    }

    for object in repo.objects.iter()? {
        let object = object?;
        writeln!(out, "{object}")?;
    }

    Ok(())
}
