use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Instant,
};

use gix_error::{Class, ClassificationMarker, ErrorExt, ExnResult, Message, ResultExt, message, retryable};

use gix_features::progress::{Count, DynNestedProgress, Progress};

use crate::loose::Store;

///
pub mod integrity {
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
    /// Verification failures include [metadata](gix_error::Exn::metadata()) `object_id` (hex text), plus `kind` (object
    /// kind text) after lookup.
    pub fn verify_integrity(
        &self,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
    ) -> ExnResult<integrity::Statistics> {
        let mut buf = Vec::new();

        let mut num_objects = 0;
        let start = Instant::now();
        let mut progress = progress.add_child_with_id("Validating".into(), integrity::ProgressId::LooseObjects.into());
        progress.init(None, gix_features::progress::count("loose objects"));
        for id in self.iter() {
            let id = id.or_raise_erased(|| message("Could not enumerate loose objects"))?;
            let object = self
                .try_find(&id, &mut buf)
                .or_raise_erased(|| {
                    Message::new("Could not read loose object during verification").with("object_id", id.to_string())
                })?
                .ok_or_else(|| retryable("Objects were deleted during iteration - try again").raise_erased())?;
            let context = || {
                Message::new("Could not verify loose object")
                    .with("object_id", id.to_string())
                    .with("kind", object.kind.to_string())
            };
            gix_object::compute_hash(self.object_hash, object.kind, object.data)
                .and_then(|actual| actual.verify(&id))
                .or_raise_erased(context)?;
            object.decode().or_raise_erased(context)?;

            progress.inc();
            num_objects += 1;
            if should_interrupt.load(Ordering::SeqCst) {
                return Err(ClassificationMarker::with_source(
                    Class::Retryable,
                    std::io::Error::from(std::io::ErrorKind::Interrupted),
                )
                .raise_erased());
            }
        }
        progress.show_throughput(start);

        Ok(integrity::Statistics { num_objects })
    }
}
