use std::{io::Write, sync::atomic::AtomicBool};

use gix_features::{parallel::InOrderIter, progress::Discard};
use gix_hash::ObjectId;
use gix_pack::{
    data::{
        output::{
            bytes::FromEntriesIter,
            count::{objects::ObjectExpansion, objects_unthreaded},
            entry::iter_from_counts,
        },
        Version,
    },
    Find,
};

/// Generate a packfile containing objects reachable from `wants` but not from `haves`.
pub fn generate_pack<F: Find + Send + Clone + 'static>(
    db: F,
    wants: &[ObjectId],
    _haves: &[ObjectId],
    out: &mut dyn Write,
) -> Result<(), Error> {
    let (counts, _outcome) = objects_unthreaded(
        &db,
        &mut wants.iter().copied().map(Ok),
        &Discard,
        &AtomicBool::new(false),
        ObjectExpansion::TreeContents,
    )?;

    let mut entries_iter = iter_from_counts(counts, db, Box::new(Discard), Default::default());

    let entries: Vec<_> = InOrderIter::from(entries_iter.by_ref())
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect();

    let num_entries = entries.len() as u32;
    let pack_writer = FromEntriesIter::new(
        std::iter::once(Ok::<_, iter_from_counts::Error>(entries)),
        out,
        num_entries,
        Version::V2,
        gix_hash::Kind::Sha1,
    );

    for result in pack_writer {
        result?;
    }

    Ok(())
}

/// Errors from pack generation.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Count(#[from] gix_pack::data::output::count::objects::Error),
    #[error(transparent)]
    Entry(#[from] iter_from_counts::Error),
    #[error(transparent)]
    PackWrite(#[from] gix_pack::data::output::bytes::Error<iter_from_counts::Error>),
}
