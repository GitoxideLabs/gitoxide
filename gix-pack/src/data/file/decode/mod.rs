use gix_error::{ResourceExhaustionKind, Result, ResultExt, bail};

///
pub mod entry;
///
pub mod header;

/// Reserve fallibly before growing the output buffer.
pub(crate) fn try_resize(out: &mut Vec<u8>, len: usize) -> Result {
    let additional = len.saturating_sub(out.len());
    out.try_reserve(additional)
        .or_raise(|| allocation_error(ResourceExhaustionKind::AllocationFailure))?;
    out.resize(len, 0);
    Ok(())
}

/// Brent's cycle detection keeps one exponentially spaced checkpoint instead of allocating
/// a visited-offset set on every header or object lookup.
struct DeltaCycle {
    checkpoint: u64,
    distance: usize,
    interval: usize,
}

impl DeltaCycle {
    fn new(offset: u64) -> Self {
        Self {
            checkpoint: offset,
            distance: 0,
            interval: 1,
        }
    }

    fn check(&mut self, offset: u64) -> Result {
        if offset == self.checkpoint {
            bail!(gix_error::corruption("A pack delta chain contains a cycle"));
        }
        self.distance += 1;
        if self.distance == self.interval {
            self.checkpoint = offset;
            self.interval = self.interval.saturating_mul(2);
            self.distance = 0;
        }
        Ok(())
    }
}

/// A ref-delta base that could not be resolved.
///
/// Its source is a classification-only [`gix_error::ClassificationMarker`].
/// Use [`gix_error::classify()`] or `is_not_found()` on [`gix_error::Exn`] and [`gix_error::Error`] to check the
/// classification, without depending on the concrete diagnostic type. Downcast to this type for the base object ID.
#[derive(Debug)]
pub struct DeltaBaseUnresolved(
    /// The object ID named by the ref-delta.
    pub gix_hash::ObjectId,
);

impl std::fmt::Display for DeltaBaseUnresolved {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "A delta chain could not be followed as the ref base with id {} could not be found",
            self.0
        )
    }
}

impl std::error::Error for DeltaBaseUnresolved {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &gix_error::ClassificationMarker::NOT_FOUND })
    }
}

#[cold]
pub(super) fn allocation_error(kind: gix_error::ResourceExhaustionKind) -> gix_error::Message {
    gix_error::resource_exhaustion(kind, "Entry too large to fit in memory")
}
