use crate::store;

/// Options controlling physical reference-store optimization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Remove reflog entries older than this Unix timestamp during compaction.
    ///
    /// Backends that cannot expire reflogs return an error so retention is never silently ignored.
    pub expire_reflogs_before: Option<u64>,
    /// Keep at least this many newest reflog entries per reference regardless of age.
    ///
    /// This floor is meaningful only when [`Self::expire_reflogs_before`] requests expiry.
    pub keep_latest_reflog_entries: usize,
    /// Remove complete aggregate-storage files that are no longer authoritative.
    pub cleanup_abandoned: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            expire_reflogs_before: None,
            keep_latest_reflog_entries: 0,
            cleanup_abandoned: true,
        }
    }
}

impl crate::Store {
    /// Verify reference storage visible to this store.
    ///
    /// This validates physical storage and decodes every reference exposed by the backend.
    pub fn verify(&self) -> Result<(), store::BackendError> {
        match &self.inner {
            store::State::Files { store } => {
                let platform = store
                    .iter()
                    .map_err(|err| store::BackendError::new("open references for verification", err))?;
                for reference in platform
                    .all()
                    .map_err(|err| store::BackendError::new("iterate references for verification", err))?
                {
                    reference.map_err(|err| store::BackendError::new("verify a reference", err))?;
                }
                for reference in platform
                    .pseudo()
                    .map_err(|err| store::BackendError::new("iterate pseudo references for verification", err))?
                {
                    reference.map_err(|err| store::BackendError::new("verify a pseudo reference", err))?;
                }
                Ok(())
            }
        }
    }

    /// Optimize physical reference storage and optionally expire old reflog entries.
    ///
    /// The files backend keeps its existing layout. Requesting reflog expiry returns an error
    /// instead of silently ignoring the request.
    pub fn optimize(&self, options: Options, _lock_fail: gix_lock::acquire::Fail) -> Result<(), store::BackendError> {
        match &self.inner {
            store::State::Files { .. } => {
                if options.expire_reflogs_before.is_some() {
                    return Err(store::BackendError::new(
                        "expire reference logs",
                        UnsupportedReflogExpiry,
                    ));
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("reflog expiry through this reference-store API is not supported by the files backend")]
struct UnsupportedReflogExpiry;
