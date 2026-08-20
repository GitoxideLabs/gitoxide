use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_features::progress::{Count, DynNestedProgress, Progress};

use crate::loose::Store;

///
pub mod integrity {
    /// The error returned by [`verify_integrity()`][super::Store::verify_integrity()].
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        ObjectDecode {
            source: gix_object::decode::Error,
            kind: gix_object::Kind,
            id: gix_hash::ObjectId,
        },
        ObjectHasher {
            source: gix_hash::hasher::Error,
            kind: gix_object::Kind,
            expected: gix_hash::ObjectId,
        },
        ObjectEncodeMismatch {
            source: gix_hash::verify::Error,
            kind: gix_object::Kind,
        },
        Retry,
        Interrupted,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::ObjectDecode { kind, id, .. } => write!(f, "{kind} object {id} could not be decoded"),
                Error::ObjectHasher { kind, expected, .. } => {
                    write!(f, "{kind} object {expected} could not be hashed")
                }
                Error::ObjectEncodeMismatch { kind, .. } => {
                    write!(f, "{kind} object wasn't re-encoded without change")
                }
                Error::Retry => f.write_str("Objects were deleted during iteration - try again"),
                Error::Interrupted => f.write_str("Interrupted"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::ObjectDecode { source, .. } => Some(source),
                Error::ObjectHasher { source, .. } => Some(source),
                Error::ObjectEncodeMismatch { source, .. } => Some(source),
                Error::Retry | Error::Interrupted => None,
            }
        }
    }

    /// The outcome returned by [`verify_integrity()`][super::Store::verify_integrity()].
    #[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct Statistics {
        /// The amount of loose objects we checked.
        pub num_objects: usize,
    }

    /// The progress ids used in [`verify_integrity()`][super::Store::verify_integrity()].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// The amount of loose objects that have been verified.
        LooseObjects,
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::LooseObjects => *b"VILO",
            }
        }
    }
}

impl Store {
    /// Check all loose objects for their integrity checking their hash matches the actual data and by decoding them fully.
    pub fn verify_integrity(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
    ) -> Result<integrity::Statistics, integrity::Error> {
        use gix_object::Write;
        let mut buf = Vec::new();
        let sink = crate::sink(self.object_hash);

        let mut num_objects = 0;
        let start = Instant::now();
        let mut progress = progress.add_child_with_id("Validating".into(), integrity::ProgressId::LooseObjects.into());
        progress.init(None, gix_features::progress::count("loose objects"));
        for id in self.iter().filter_map(Result::ok) {
            let object = self
                .try_find(&id, &mut buf)
                .map_err(|_| integrity::Error::Retry)?
                .ok_or(integrity::Error::Retry)?;
            sink.write_buf(object.kind, object.data)
                .map_err(|err| integrity::Error::ObjectHasher {
                    source: *err.downcast().expect("sink can only fail in hasher"),
                    kind: object.kind,
                    expected: id,
                })?
                .verify(&id)
                .map_err(|err| integrity::Error::ObjectEncodeMismatch {
                    source: err,
                    kind: object.kind,
                })?;
            object.decode().map_err(|err| integrity::Error::ObjectDecode {
                source: err,
                kind: object.kind,
                id,
            })?;

            progress.inc();
            num_objects += 1;
            if should_interrupt.load(Ordering::SeqCst) {
                return Err(integrity::Error::Interrupted);
            }
        }
        progress.show_throughput(start);

        Ok(integrity::Statistics { num_objects })
    }
}
