///
pub mod init;

mod find;
///
#[cfg(all(not(feature = "wasm"), feature = "streaming-input"))]
pub mod write;

///
pub mod verify {
    use std::sync::atomic::AtomicBool;

    use gix_features::progress::DynNestedProgress;

    ///
    pub mod integrity {
        /// Returned by [`Bundle::verify_integrity()`][crate::Bundle::verify_integrity()].
        pub struct Outcome {
            /// The computed checksum of the index which matched the stored one.
            pub actual_index_checksum: gix_hash::ObjectId,
            /// The packs traversal outcome
            pub pack_traverse_outcome: crate::index::traverse::Statistics,
        }
    }

    use crate::Bundle;

    impl Bundle {
        /// Similar to [`crate::index::File::verify_integrity()`] but more convenient to call as the presence of the
        /// pack file is a given.
        pub fn verify_integrity<C, F>(
            &self,
            progress: &mut dyn DynNestedProgress,
            should_interrupt: &AtomicBool,
            options: crate::index::verify::integrity::Options<F>,
        ) -> Result<integrity::Outcome, crate::index::traverse::Error>
        where
            C: crate::cache::DecodeEntry,
            F: Fn() -> C + Send + Clone,
        {
            self.index
                .verify_integrity(
                    Some(crate::index::verify::PackContext {
                        data: &self.pack,
                        options,
                    }),
                    progress,
                    should_interrupt,
                )
                .map(|o| integrity::Outcome {
                    actual_index_checksum: o.actual_index_checksum,
                    pack_traverse_outcome: o.pack_traverse_statistics.expect("pack is set"),
                })
        }
    }
}
