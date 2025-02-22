use std::{ffi::OsStr, io, path::Path, str::FromStr, sync::atomic::AtomicBool};

use anyhow::{anyhow, Context as AnyhowContext, Result};
use bytesize::ByteSize;
use gix::{
    object, odb,
    odb::{pack, pack::index},
    NestedProgress,
};
pub use index::verify::Mode;
pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=2;

use crate::OutputFormat;

#[derive(Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum Algorithm {
    LessTime,
    LessMemory,
}

impl Algorithm {
    pub fn variants() -> &'static [&'static str] {
        &["less-time", "less-memory"]
    }
}

impl FromStr for Algorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s_lc = s.to_ascii_lowercase();
        Ok(match s_lc.as_str() {
            "less-memory" => Algorithm::LessMemory,
            "less-time" => Algorithm::LessTime,
            _ => return Err(format!("Invalid verification algorithm: '{s}'")),
        })
    }
}

impl From<Algorithm> for index::traverse::Algorithm {
    fn from(v: Algorithm) -> Self {
        match v {
            Algorithm::LessMemory => index::traverse::Algorithm::Lookup,
            Algorithm::LessTime => index::traverse::Algorithm::DeltaTreeLookup,
        }
    }
}

/// A general purpose context for many operations provided here
pub struct Context<'a, W1: io::Write, W2: io::Write> {
    /// If set, provide statistics to `out` in the given format
    pub output_statistics: Option<OutputFormat>,
    /// A stream to which to output operation results
    pub out: W1,
    /// A stream to which to errors
    pub err: W2,
    /// If set, don't use more than this amount of threads.
    /// Otherwise, usually use as many threads as there are logical cores.
    /// A value of 0 is interpreted as no-limit
    pub thread_limit: Option<usize>,
    pub mode: index::verify::Mode,
    pub algorithm: Algorithm,
    pub should_interrupt: &'a AtomicBool,
    pub object_hash: gix::hash::Kind,
}

enum EitherCache<const SIZE: usize> {
    Left(pack::cache::Never),
    Right(pack::cache::lru::StaticLinkedList<SIZE>),
}

impl<const SIZE: usize> pack::cache::DecodeEntry for EitherCache<SIZE> {
    fn put(&mut self, pack_id: u32, offset: u64, data: &[u8], kind: object::Kind, compressed_size: usize) {
        match self {
            EitherCache::Left(v) => v.put(pack_id, offset, data, kind, compressed_size),
            EitherCache::Right(v) => v.put(pack_id, offset, data, kind, compressed_size),
        }
    }

    fn get(&mut self, pack_id: u32, offset: u64, out: &mut Vec<u8>) -> Option<(object::Kind, usize)> {
        match self {
            EitherCache::Left(v) => v.get(pack_id, offset, out),
            EitherCache::Right(v) => v.get(pack_id, offset, out),
        }
    }
}

pub fn pack_or_pack_index<W1, W2>(
    path: impl AsRef<Path>,
    mut progress: impl NestedProgress + 'static,
    Context {
        mut out,
        mut err,
        mode,
        output_statistics,
        thread_limit,
        algorithm,
        should_interrupt,
        object_hash,
    }: Context<'_, W1, W2>,
) -> Result<()>
where
    W1: io::Write,
    W2: io::Write,
{
    let path = path.as_ref();
    let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
    const CACHE_SIZE: usize = 64;
    let cache = || -> EitherCache<CACHE_SIZE> {
        if matches!(algorithm, Algorithm::LessMemory) {
            if output_statistics.is_some() {
                // turn off acceleration as we need to see entire chains all the time
                EitherCache::Left(pack::cache::Never)
            } else {
                EitherCache::Right(pack::cache::lru::StaticLinkedList::<CACHE_SIZE>::default())
            }
        } else {
            EitherCache::Left(pack::cache::Never)
        }
    };
    let res = match ext {
        "pack" => {
            let pack = odb::pack::data::File::at(path, object_hash).with_context(|| "Could not open pack file")?;
            pack.verify_checksum(&mut progress.add_child("Sha1 of pack"), should_interrupt)
                .map(|id| (id, None))?
        }
        "idx" => {
            let idx =
                odb::pack::index::File::at(path, object_hash).with_context(|| "Could not open pack index file")?;
            let packfile_path = path.with_extension("pack");
            let pack = odb::pack::data::File::at(&packfile_path, object_hash)
                .map_err(|e| {
                    writeln!(
                        err,
                        "Could not find matching pack file at '{}' - only index file will be verified, error was: {}",
                        packfile_path.display(),
                        e
                    )
                    .ok();
                    e
                })
                .ok();

            idx.verify_integrity(
                pack.as_ref().map(|p| gix::odb::pack::index::verify::PackContext {
                    data: p,
                    options: gix::odb::pack::index::verify::integrity::Options {
                        verify_mode: mode,
                        traversal: algorithm.into(),
                        make_pack_lookup_cache: cache,
                        thread_limit
                    }
                }),
                &mut progress,
                should_interrupt,
            )
            .map(|o| (o.actual_index_checksum, o.pack_traverse_statistics))
            .with_context(|| "Verification failure")?
        }
        "" => {
            match path.file_name() {
                Some(file_name) if file_name == "multi-pack-index" => {
                    let multi_index = gix::odb::pack::multi_index::File::at(path)?;
                    let res = multi_index.verify_integrity(&mut progress, should_interrupt, gix::odb::pack::index::verify::integrity::Options{
                        verify_mode: mode,
                        traversal: algorithm.into(),
                        thread_limit,
                        make_pack_lookup_cache: cache
                    })?;
                    match output_statistics {
                        Some(OutputFormat::Human) => {
                            for (index_name, stats) in multi_index.index_names().iter().zip(res.pack_traverse_statistics) {
                                writeln!(out, "{}", index_name.display()).ok();
                                drop(print_statistics(&mut out, &stats));
                            }
                        },
                        #[cfg(feature = "serde")]
                        Some(OutputFormat::Json) => serde_json::to_writer_pretty(out, &multi_index.index_names().iter().zip(res.pack_traverse_statistics).collect::<Vec<_>>())?,
                        _ => {}
                    }
                    return Ok(())
                },
                _ => return Err(anyhow!(
                        "Cannot determine data type on path without extension '{}', expecting default extensions 'idx' and 'pack'",
                        path.display()
                    ))
            }
        }
        ext => return Err(anyhow!("Unknown extension {:?}, expecting 'idx' or 'pack'", ext)),
    };
    if let Some(stats) = res.1.as_ref() {
        #[cfg_attr(not(feature = "serde"), allow(clippy::single_match))]
        match output_statistics {
            Some(OutputFormat::Human) => drop(print_statistics(&mut out, stats)),
            #[cfg(feature = "serde")]
            Some(OutputFormat::Json) => serde_json::to_writer_pretty(out, stats)?,
            _ => {}
        }
    }
    Ok(())
}

fn print_statistics(out: &mut impl io::Write, stats: &index::traverse::Statistics) -> io::Result<()> {
    writeln!(out, "objects per delta chain length")?;
    let mut chain_length_to_object: Vec<_> = stats.objects_per_chain_length.iter().map(|(a, b)| (*a, *b)).collect();
    chain_length_to_object.sort_by_key(|e| e.0);
    let mut total_object_count = 0;
    for (chain_length, object_count) in chain_length_to_object.into_iter() {
        total_object_count += object_count;
        writeln!(out, "\t{chain_length:>2}: {object_count}")?;
    }
    writeln!(out, "\t->: {total_object_count}")?;

    let pack::data::decode::entry::Outcome {
        kind: _,
        num_deltas,
        decompressed_size,
        compressed_size,
        object_size,
    } = stats.average;

    let width = 30;
    writeln!(out, "\naverages")?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\t{:<width$} {};\n\t{:<width$} {};\n\t{:<width$} {};\n\t{:<width$} {};",
        "delta chain length:", num_deltas,
        "decompressed entry [B]:", decompressed_size,
        "compressed entry [B]:", compressed_size,
        "decompressed object size [B]:", object_size,
        width = width
    )?;

    writeln!(out, "\ncompression")?;
    #[rustfmt::skip]
    writeln!(
        out, "\t{:<width$}: {}\n\t{:<width$}: {}\n\t{:<width$}: {}\n\t{:<width$}: {}",
        "compressed entries size", ByteSize(stats.total_compressed_entries_size),
        "decompressed entries size", ByteSize(stats.total_decompressed_entries_size),
        "total object size", ByteSize(stats.total_object_size),
        "pack size", ByteSize(stats.pack_size),
        width = width
    )?;
    #[rustfmt::skip]
    writeln!(
        out,
        "\n\t{:<width$}: {}\n\t{:<width$}: {}\n\t{:<width$}: {}\n\t{:<width$}: {}",
        "num trees", stats.num_trees,
        "num blobs", stats.num_blobs,
        "num commits", stats.num_commits,
        "num tags", stats.num_tags,
        width = width
    )?;
    let compression_ratio = stats.total_decompressed_entries_size as f64 / stats.total_compressed_entries_size as f64;
    let delta_compression_ratio = stats.total_object_size as f64 / stats.total_compressed_entries_size as f64;
    #[rustfmt::skip]
    writeln!(
        out,
        "\n\t{:<width$}: {:.2}\n\t{:<width$}: {:.2}\n\t{:<width$}: {:.2}\n\t{:<width$}: {:.3}%",
        "compression ratio", compression_ratio,
        "delta compression ratio", delta_compression_ratio,
        "delta gain", delta_compression_ratio / compression_ratio,
        "pack overhead", (1.0 - (stats.total_compressed_entries_size as f64 / stats.pack_size as f64)) * 100.0,
        width = width
    )?;
    Ok(())
}
