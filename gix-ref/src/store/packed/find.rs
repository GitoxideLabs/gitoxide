use std::sync::atomic::Ordering;

use gix_object::bstr::{BStr, BString, ByteSlice};

use crate::{FullNameRef, PartialNameRef, store_impl::packed};

/// Number of `try_find_full_name` calls served by binary search before
/// the next call eagerly builds a name → offset index for the rest of the
/// buffer's lifetime. The threshold trades a small one-time build cost
/// against `log₂(n)`-per-call binary searches; below it, single-shot
/// lookups (typical CLI commands) pay nothing extra.
pub(super) const INDEX_BUILD_AFTER_LOOKUPS: usize = 8;

/// packed-refs specific functionality
impl packed::Buffer {
    /// Find a reference with the given `name` and return it.
    ///
    /// Note that it will look it up verbatim and does not deal with namespaces or special prefixes like
    /// `main-worktree/` or `worktrees/<name>/`, as this is left to the caller.
    pub fn try_find<'a, Name, E>(&self, name: Name) -> Result<Option<packed::Reference<'_>>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        let name = name.try_into()?;
        let mut buf = BString::default();
        for inbetween in &["", "tags", "heads", "remotes"] {
            let (name, was_absolute) = if name.looks_like_full_name(false) {
                let name = FullNameRef::new_unchecked(name.as_bstr());
                let name = match transform_full_name_for_lookup(name) {
                    None => return Ok(None),
                    Some(name) => name,
                };
                (name, true)
            } else {
                let full_name = name.construct_full_name_ref(inbetween, &mut buf, false);
                (full_name, false)
            };
            match self.try_find_full_name(name)? {
                Some(r) => return Ok(Some(r)),
                None if was_absolute => return Ok(None),
                None => continue,
            }
        }
        Ok(None)
    }

    pub(crate) fn try_find_full_name(&self, name: &FullNameRef) -> Result<Option<packed::Reference<'_>>, Error> {
        // Fast path: a built index turns the lookup into a single HashMap
        // probe (plus an `decode::reference` re-parse at the matched offset).
        if let Some(index) = self.name_index.get() {
            return self.lookup_via_index(name, index);
        }
        // Cold path: count this lookup. After enough binary searches we
        // amortize the index build cost, so build it now and serve this call
        // from the index too.
        let prev_lookups = self.lookup_count.fetch_add(1, Ordering::Relaxed);
        if prev_lookups + 1 >= INDEX_BUILD_AFTER_LOOKUPS {
            let index = self.name_index.get_or_init(|| self.build_name_index());
            return self.lookup_via_index(name, index);
        }
        self.try_find_full_name_via_binary_search(name)
    }

    fn try_find_full_name_via_binary_search(&self, name: &FullNameRef) -> Result<Option<packed::Reference<'_>>, Error> {
        match self.binary_search_by(name.as_bstr()) {
            Ok(line_start) => {
                let mut input = &self.as_ref()[line_start..];
                Ok(Some(
                    packed::decode::reference(&mut input, self.object_hash).map_err(|_| Error::Parse)?,
                ))
            }
            Err((parse_failure, _)) => {
                if parse_failure {
                    Err(Error::Parse)
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn lookup_via_index(
        &self,
        name: &FullNameRef,
        index: &packed::NameIndex,
    ) -> Result<Option<packed::Reference<'_>>, Error> {
        match index.by_name.get(name.as_bstr()) {
            Some(&offset) => {
                let mut input = &self.as_ref()[offset..];
                Ok(Some(
                    packed::decode::reference(&mut input, self.object_hash).map_err(|_| Error::Parse)?,
                ))
            }
            None => {
                if index.encountered_parse_failure {
                    Err(Error::Parse)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Walk the entire buffer once, collecting `(name → record-start offset)`
    /// pairs for every well-formed record. Malformed lines are skipped to
    /// the next newline but flip [`packed::NameIndex::encountered_parse_failure`]
    /// so the lookup path still reports them via [`Error::Parse`] on a miss.
    fn build_name_index(&self) -> packed::NameIndex {
        let body = self.as_ref();
        let body_start = body.as_ptr() as usize;
        let mut by_name = std::collections::HashMap::with_capacity(estimate_record_count(body));
        let mut encountered_parse_failure = false;
        let mut cursor = body;
        loop {
            if cursor.is_empty() {
                break;
            }
            let record_offset = cursor.as_ptr() as usize - body_start;
            let mut after = cursor;
            match packed::decode::reference(&mut after, self.object_hash) {
                Ok(r) => {
                    by_name.insert(r.name.as_bstr().to_owned(), record_offset);
                    cursor = after;
                }
                Err(_) => {
                    encountered_parse_failure = true;
                    // Skip to the next line so a single malformed record
                    // doesn't prevent us from indexing the rest.
                    cursor = match cursor.iter().position(|&b| b == b'\n') {
                        Some(pos) => &cursor[pos + 1..],
                        None => &[],
                    };
                }
            }
        }
        packed::NameIndex {
            by_name,
            encountered_parse_failure,
        }
    }

    /// Find a reference with the given `name` and return it.
    pub fn find<'a, Name, E>(&self, name: Name) -> Result<packed::Reference<'_>, existing::Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        match self.try_find(name) {
            Ok(Some(r)) => Ok(r),
            Ok(None) => Err(existing::Error::NotFound),
            Err(err) => Err(existing::Error::Find(err)),
        }
    }

    /// Perform a binary search where `Ok(pos)` is the beginning of the line that matches `name` perfectly and `Err(pos)`
    /// is the beginning of the line at which `name` could be inserted to still be in sort order.
    pub(in crate::store_impl::packed) fn binary_search_by(&self, full_name: &BStr) -> Result<usize, (bool, usize)> {
        let a = self.as_ref();
        let search_start_of_record = |ofs: usize| {
            a[..ofs]
                .rfind(b"\n")
                .and_then(|pos| {
                    let candidate = pos + 1;
                    a.get(candidate).and_then(|b| {
                        if *b == b'^' {
                            a[..pos].rfind(b"\n").map(|pos| pos + 1)
                        } else {
                            Some(candidate)
                        }
                    })
                })
                .unwrap_or(0)
        };
        let mut encountered_parse_failure = false;
        a.binary_search_by_key(&full_name.as_ref(), |b: &u8| {
            let ofs = std::ptr::from_ref::<u8>(b) as usize - a.as_ptr() as usize;
            let mut line = &a[search_start_of_record(ofs)..];
            packed::decode::reference(&mut line, self.object_hash)
                .map(|r| r.name.as_bstr().as_bytes())
                .inspect_err(|_err| {
                    encountered_parse_failure = true;
                })
                .unwrap_or(&[])
        })
        .map(search_start_of_record)
        .map_err(|pos| (encountered_parse_failure, search_start_of_record(pos)))
    }
}

mod error {
    use std::convert::Infallible;

    /// The error returned by [`find()`][super::packed::Buffer::find()]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The ref name or path is not a valid ref name")]
        RefnameValidation(#[from] crate::name::Error),
        #[error("The reference could not be parsed")]
        Parse,
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!("this impl is needed to allow passing a known valid partial path as parameter")
        }
    }
}
pub use error::Error;

///
pub mod existing {

    /// The error returned by [`find_existing()`][super::packed::Buffer::find()]
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The find operation failed")]
        Find(#[from] super::Error),
        #[error("The reference did not exist even though that was expected")]
        NotFound,
    }
}

/// A rough upper-bound estimate on the number of records in a packed-refs
/// buffer, used only to preallocate the HashMap built by
/// [`packed::Buffer::build_name_index`]. The shortest possible record is
/// `<hash><space><name>\n` which is at least `hex_len + 4` bytes (1-char name).
/// Overestimating is fine — the HashMap just grows naturally.
fn estimate_record_count(body: &[u8]) -> usize {
    // Conservative average of ~50 bytes/record gives a sensible initial
    // capacity without scanning the file.
    body.len() / 50
}

pub(crate) fn transform_full_name_for_lookup(name: &FullNameRef) -> Option<&FullNameRef> {
    match name.category_and_short_name() {
        Some((c, sn)) => {
            use crate::Category::*;
            Some(match c {
                MainRef | LinkedRef { .. } => FullNameRef::new_unchecked(sn),
                Tag | RemoteBranch | LocalBranch | Bisect | Rewritten | Note => name,
                MainPseudoRef | PseudoRef | LinkedPseudoRef { .. } | WorktreePrivate => return None,
            })
        }
        None => Some(name),
    }
}
