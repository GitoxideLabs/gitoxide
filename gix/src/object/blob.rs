use crate::{Blob, ObjectDetached};

///
#[cfg(feature = "blob-diff")]
pub mod diff {
    use gix_diff::blob::platform::prepare_diff::Operation;

    use crate::bstr::ByteSlice;

    /// A platform to keep temporary information to perform line diffs on modified blobs.
    ///
    pub struct Platform<'a> {
        /// The cache holding diffable data related to our blobs.
        pub resource_cache: &'a mut gix_diff::blob::Platform,
    }

    ///
    pub mod init {
        /// The error returned by [`object::tree::diff::Change::diff`](crate::object::tree::diff::Change::diff()).
        pub type Error = gix_diff::blob::platform::set_resource::Error;
    }

    ///
    pub mod lines {
        use crate::bstr::BStr;

        /// The error returned by [Platform::lines()](super::Platform::lines()).
        // TODO(review): kept concrete. Generic over the caller-supplied hunk-processing error `E`;
        //                a type alias to `gix_error::Error` can't carry that type parameter, and doing
        //                so would erase the caller's own error type inside `ProcessHunk(E)`, which is
        //                the entire point of this type being generic.
        #[derive(Debug, thiserror::Error)]
        #[expect(missing_docs)]
        pub enum Error<E>
        where
            E: std::error::Error + Send + Sync + 'static,
        {
            #[error(transparent)]
            ProcessHunk(E),
            #[error(transparent)]
            PrepareDiff(#[from] gix_diff::blob::platform::prepare_diff::Error),
        }

        /// A change to a hunk of lines.
        pub enum Change<'a, 'data> {
            /// Lines were added.
            Addition {
                /// The lines themselves without terminator.
                lines: &'a [&'data BStr],
            },
            /// Lines were removed.
            Deletion {
                /// The lines themselves without terminator.
                lines: &'a [&'data BStr],
            },
            /// Lines have been replaced.
            Modification {
                /// The replaced lines without terminator.
                lines_before: &'a [&'data BStr],
                /// The new lines without terminator.
                lines_after: &'a [&'data BStr],
            },
        }
    }

    impl Platform<'_> {
        /// Perform a diff on lines between the old and the new version of a blob, passing each hunk of lines to `process_hunk`.
        /// The diffing algorithm is determined by the `diff.algorithm` configuration, or individual diff drivers.
        /// Note that `process_hunk` is not called if one of the involved resources are binary, but that can be determined
        /// by introspecting the outcome.
        // TODO: more tests (only tested insertion right now)
        pub fn lines<FnH, E>(
            &mut self,
            mut process_hunk: FnH,
        ) -> Result<gix_diff::blob::platform::prepare_diff::Outcome<'_>, lines::Error<E>>
        where
            FnH: FnMut(lines::Change<'_, '_>) -> Result<(), E>,
            E: std::error::Error + Send + Sync + 'static,
        {
            self.resource_cache.options.skip_internal_diff_if_external_is_configured = false;

            let prep = self.resource_cache.prepare_diff()?;
            match prep.operation {
                Operation::InternalDiff { algorithm } => {
                    let input = prep.interned_input();
                    let diff = gix_diff::blob::diff_with_slider_heuristics(algorithm, &input);
                    let mut err = None;
                    let mut lines = Vec::new();

                    for hunk in diff.hunks() {
                        if err.is_some() {
                            break;
                        }
                        let before = hunk.before;
                        let after = hunk.after;
                        lines.clear();
                        lines.extend(
                            input.before[before.start as usize..before.end as usize]
                                .iter()
                                .map(|&line| input.interner[line].as_bstr()),
                        );
                        let end_of_before = lines.len();
                        lines.extend(
                            input.after[after.start as usize..after.end as usize]
                                .iter()
                                .map(|&line| input.interner[line].as_bstr()),
                        );
                        let hunk_before = &lines[..end_of_before];
                        let hunk_after = &lines[end_of_before..];
                        if hunk_after.is_empty() {
                            err = process_hunk(lines::Change::Deletion { lines: hunk_before }).err();
                        } else if hunk_before.is_empty() {
                            err = process_hunk(lines::Change::Addition { lines: hunk_after }).err();
                        } else {
                            err = process_hunk(lines::Change::Modification {
                                lines_before: hunk_before,
                                lines_after: hunk_after,
                            })
                            .err();
                        }
                    }

                    if let Some(err) = err {
                        return Err(lines::Error::ProcessHunk(err));
                    }
                }
                Operation::ExternalCommand { .. } => {
                    unreachable!("we disabled that")
                }
                Operation::SourceOrDestinationIsBinary => {}
            }
            Ok(prep)
        }

        /// Count the amount of removed and inserted lines efficiently.
        /// Note that nothing will happen if one of the inputs is binary, and `None` will be returned.
        pub fn line_counts(
            &mut self,
        ) -> Result<Option<gix_diff::blob::DiffLineStats>, gix_diff::blob::platform::prepare_diff::Error> {
            self.resource_cache.options.skip_internal_diff_if_external_is_configured = false;

            let prep = self.resource_cache.prepare_diff()?;
            match prep.operation {
                Operation::InternalDiff { algorithm } => {
                    let input = prep.interned_input();
                    let diff = gix_diff::blob::Diff::compute(algorithm, &input);
                    Ok(Some(gix_diff::blob::DiffLineStats {
                        removals: diff.count_removals(),
                        insertions: diff.count_additions(),
                        before: input.before.len(),
                        after: input.after.len(),
                        similarity: 0.0,
                    }))
                }
                Operation::ExternalCommand { .. } => {
                    unreachable!("we disabled that")
                }
                Operation::SourceOrDestinationIsBinary => Ok(None),
            }
        }
    }
}

/// Remove Lifetime
impl Blob<'_> {
    /// Create an owned instance of this object, copying our data in the process.
    pub fn detached(&self) -> ObjectDetached {
        ObjectDetached {
            id: self.id,
            kind: gix_object::Kind::Blob,
            data: self.data.clone(),
        }
    }

    /// Sever the connection to the `Repository` and turn this instance into a standalone object.
    pub fn detach(self) -> ObjectDetached {
        self.into()
    }

    /// Retrieve this instance's data, leaving its own data empty.
    ///
    /// This method works around the immovability of members of this type.
    pub fn take_data(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.data)
    }
}
