use gix_object::bstr::{BStr, BString, ByteSlice};

use crate::{FullNameRef, PartialNameRef, store_impl::packed};

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
