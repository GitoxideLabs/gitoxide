use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_features::{
    parallel,
    progress::Progress,
    threading::{Mutable, OwnShared, lock},
};

use crate::{data, index::traverse};

fn add_decode_result(lhs: &mut data::decode::entry::Outcome, rhs: data::decode::entry::Outcome) {
    lhs.num_deltas += rhs.num_deltas;
    lhs.decompressed_size += rhs.decompressed_size;
    lhs.compressed_size += rhs.compressed_size;
    lhs.object_size += rhs.object_size;
}

fn div_decode_result(lhs: &mut data::decode::entry::Outcome, div: usize) {
    if div != 0 {
        lhs.num_deltas = (lhs.num_deltas as f32 / div as f32) as u32;
        lhs.decompressed_size /= div as u64;
        lhs.compressed_size /= div;
        lhs.object_size /= div as u64;
    }
}

pub struct Reducer<'a, P, E> {
    progress: OwnShared<Mutable<P>>,
    check: traverse::SafetyCheck,
    then: Instant,
    entries_seen: usize,
    stats: traverse::Statistics,
    should_interrupt: &'a AtomicBool,
    _error: std::marker::PhantomData<E>,
}

impl<'a, P, E> Reducer<'a, P, E>
where
    P: Progress,
{
    pub fn from_progress(
        progress: OwnShared<Mutable<P>>,
        pack_data_len_in_bytes: usize,
        check: traverse::SafetyCheck,
        should_interrupt: &'a AtomicBool,
    ) -> Self {
        let stats = traverse::Statistics {
            pack_size: pack_data_len_in_bytes as u64,
            ..Default::default()
        };
        Reducer {
            progress,
            check,
            then: Instant::now(),
            entries_seen: 0,
            should_interrupt,
            stats,
            _error: Default::default(),
        }
    }
}

impl<P, E> parallel::Reduce for Reducer<'_, P, E>
where
    P: Progress,
    E: std::error::Error + Send + Sync + 'static,
{
    type Input = Result<Vec<data::decode::entry::Outcome>, traverse::Error<E>>;
    type FeedProduce = ();
    type Output = traverse::Statistics;
    type Error = traverse::Error<E>;

    fn feed(&mut self, input: Self::Input) -> Result<(), Self::Error> {
        let chunk_stats: Vec<_> = match input {
            Err(err @ traverse::Error::PackDecode { .. }) if !self.check.fatal_decode_error() => {
                lock(&self.progress).info(format!("Ignoring decode error: {err}"));
                return Ok(());
            }
            res => res,
        }?;
        self.entries_seen += chunk_stats.len();

        let chunk_total = chunk_stats.into_iter().fold(
            data::decode::entry::Outcome::default_from_kind(gix_object::Kind::Tree),
            |mut total, stats| {
                *self.stats.objects_per_chain_length.entry(stats.num_deltas).or_insert(0) += 1;
                self.stats.total_decompressed_entries_size += stats.decompressed_size;
                self.stats.total_compressed_entries_size += stats.compressed_size as u64;
                self.stats.total_object_size += stats.object_size;
                use gix_object::Kind::*;
                match stats.kind {
                    Commit => self.stats.num_commits += 1,
                    Tree => self.stats.num_trees += 1,
                    Blob => self.stats.num_blobs += 1,
                    Tag => self.stats.num_tags += 1,
                }
                add_decode_result(&mut total, stats);
                total
            },
        );

        add_decode_result(&mut self.stats.average, chunk_total);
        lock(&self.progress).set(self.entries_seen);

        if self.should_interrupt.load(Ordering::SeqCst) {
            return Err(Self::Error::Interrupted);
        }
        Ok(())
    }

    fn finalize(mut self) -> Result<Self::Output, Self::Error> {
        div_decode_result(&mut self.stats.average, self.entries_seen);

        let elapsed_s = self.then.elapsed().as_secs_f32();
        let objects_per_second = (self.entries_seen as f32 / elapsed_s) as u32;

        lock(&self.progress).info(format!(
            "of {} objects done in {:.2}s ({} objects/s, ~{}/s)",
            self.entries_seen,
            elapsed_s,
            objects_per_second,
            gix_features::progress::bytesize::ByteSize(self.stats.average.object_size * u64::from(objects_per_second))
        ));
        Ok(self.stats)
    }
}
