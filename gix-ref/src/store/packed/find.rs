use gix_object::bstr::{BStr, BString};

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
        let mut encountered_parse_failure = false;
        a.binary_search_by_key(&full_name.as_ref(), |b: &u8| {
            let ofs = std::ptr::from_ref::<u8>(b) as usize - a.as_ptr() as usize;
            let line = packed::decode::record_at_offset(a, ofs);
            // The binary search only needs the name bytes for ordered
            // comparison; skip ref-name and hex-hash validation here and let
            // the final match site re-parse the record via `decode::reference`
            // (which validates fully). This saves the `log₂(n)` per-query.
            match packed::decode::name_at_record_start(line, self.object_hash) {
                Some(name) => name,
                None => {
                    encountered_parse_failure = true;
                    &[]
                }
            }
        })
        .map(|pos| packed::decode::record_start_at_offset(a, pos))
        .map_err(|pos| {
            (
                encountered_parse_failure,
                packed::decode::record_start_at_offset(a, pos),
            )
        })
    }
}

mod error {
    use std::convert::Infallible;

    /// The error returned by [`find()`][super::packed::Buffer::find()]
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        RefnameValidation(crate::name::Error),
        Parse,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::RefnameValidation(_) => f.write_str("The ref name or path is not a valid ref name"),
                Error::Parse => f.write_str("The reference could not be parsed"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::RefnameValidation(err) => Some(err),
                Error::Parse => None,
            }
        }
    }

    impl From<crate::name::Error> for Error {
        fn from(err: crate::name::Error) -> Self {
            Error::RefnameValidation(err)
        }
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
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Find(super::Error),
        NotFound,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Find(_) => f.write_str("The find operation failed"),
                Error::NotFound => f.write_str("The reference did not exist even though that was expected"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Find(err) => Some(err),
                Error::NotFound => None,
            }
        }
    }

    impl From<super::Error> for Error {
        fn from(err: super::Error) -> Self {
            Error::Find(err)
        }
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
