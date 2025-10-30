/// The error returned by the [`parse()`][crate::parse()] function.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Empty refspecs are invalid")]
    Empty,
    #[error("Negative refspecs cannot have destinations as they exclude sources")]
    NegativeWithDestination,
    #[error("Negative specs must not be empty")]
    NegativeEmpty,
    #[error("Negative specs must be object hashes")]
    NegativeObjectHash,
    #[error("Negative specs must be full ref names, starting with \"refs/\"")]
    NegativePartialName,
    #[error("Negative glob patterns are not allowed")]
    NegativeGlobPattern,
    #[error("Fetch destinations must be ref-names, like 'HEAD:refs/heads/branch'")]
    InvalidFetchDestination,
    #[error("Cannot push into an empty destination")]
    PushToEmpty,
    #[error("glob patterns may only involved a single '*' character, found {pattern:?}")]
    PatternUnsupported { pattern: bstr::BString },
    #[error("Both sides of the specification need a pattern, like 'a/*:b/*'")]
    PatternUnbalanced,
    #[error(transparent)]
    ReferenceName(#[from] gix_validate::reference::name::Error),
    #[error(transparent)]
    RevSpec(#[from] gix_revision::spec::parse::Error),
}

/// Define how the parsed refspec should be used.
#[derive(PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Hash, Debug)]
pub enum Operation {
    /// The `src` side is local and the `dst` side is remote.
    Push,
    /// The `src` side is remote and the `dst` side is local.
    Fetch,
}

pub(crate) mod function {
    use bstr::{BStr, ByteSlice};

    use crate::{
        parse::{Error, Operation},
        types::Mode,
        RefSpecRef,
    };

    /// Parse `spec` for use in `operation` and return it if it is valid.
    pub fn parse(mut spec: &BStr, operation: Operation) -> Result<RefSpecRef<'_>, Error> {
        fn fetch_head_only(mode: Mode) -> RefSpecRef<'static> {
            RefSpecRef {
                mode,
                op: Operation::Fetch,
                src: Some("HEAD".into()),
                dst: None,
            }
        }

        let mode = match spec.first() {
            Some(&b'^') => {
                spec = &spec[1..];
                Mode::Negative
            }
            Some(&b'+') => {
                spec = &spec[1..];
                Mode::Force
            }
            Some(_) => Mode::Normal,
            None => {
                return match operation {
                    Operation::Push => Err(Error::Empty),
                    Operation::Fetch => Ok(fetch_head_only(Mode::Normal)),
                }
            }
        };

        let (mut src, dst) = match spec.find_byte(b':') {
            Some(pos) => {
                if mode == Mode::Negative {
                    return Err(Error::NegativeWithDestination);
                }

                let (src, dst) = spec.split_at(pos);
                let dst = &dst[1..];
                let src = (!src.is_empty()).then(|| src.as_bstr());
                let dst = (!dst.is_empty()).then(|| dst.as_bstr());
                match (src, dst) {
                    (None, None) => match operation {
                        Operation::Push => (None, None),
                        Operation::Fetch => (Some("HEAD".into()), None),
                    },
                    (None, Some(dst)) => match operation {
                        Operation::Push => (None, Some(dst)),
                        Operation::Fetch => (Some("HEAD".into()), Some(dst)),
                    },
                    (Some(src), None) => match operation {
                        Operation::Push => return Err(Error::PushToEmpty),
                        Operation::Fetch => (Some(src), None),
                    },
                    (Some(src), Some(dst)) => (Some(src), Some(dst)),
                }
            }
            None => {
                let src = (!spec.is_empty()).then_some(spec);
                if Operation::Fetch == operation && mode != Mode::Negative && src.is_none() {
                    return Ok(fetch_head_only(mode));
                } else {
                    (src, None)
                }
            }
        };

        if let Some(spec) = src.as_mut() {
            if *spec == "@" {
                *spec = "HEAD".into();
            }
        }
        let is_one_sided = dst.is_none();
        let (src, src_had_pattern) = validated(src, operation == Operation::Push && dst.is_some(), is_one_sided)?;
        let (dst, dst_had_pattern) = validated(dst, false, false)?;
        // For one-sided refspecs, we don't need to check for pattern balance
        if !is_one_sided && mode != Mode::Negative && src_had_pattern != dst_had_pattern {
            return Err(Error::PatternUnbalanced);
        }

        if mode == Mode::Negative {
            match src {
                Some(spec) => {
                    if src_had_pattern {
                        return Err(Error::NegativeGlobPattern);
                    } else if looks_like_object_hash(spec) {
                        return Err(Error::NegativeObjectHash);
                    } else if !spec.starts_with(b"refs/") && spec != "HEAD" {
                        return Err(Error::NegativePartialName);
                    }
                }
                None => return Err(Error::NegativeEmpty),
            }
        }

        Ok(RefSpecRef {
            op: operation,
            mode,
            src,
            dst,
        })
    }

    fn looks_like_object_hash(spec: &BStr) -> bool {
        spec.len() >= gix_hash::Kind::shortest().len_in_hex() && spec.iter().all(u8::is_ascii_hexdigit)
    }

    fn validated(spec: Option<&BStr>, allow_revspecs: bool, is_one_sided: bool) -> Result<(Option<&BStr>, bool), Error> {
        match spec {
            Some(spec) => {
                let glob_count = spec.iter().filter(|b| **b == b'*').take(2).count();
                if glob_count > 1 {
                    // For one-sided refspecs, allow any number of globs without validation
                    if !is_one_sided {
                        return Err(Error::PatternUnsupported { pattern: spec.into() });
                    }
                }
                // Check if there are any globs (one or more asterisks)
                let has_globs = glob_count > 0;
                if has_globs {
                    // For one-sided refspecs, skip validation of glob patterns
                    if !is_one_sided {
                        let mut buf = smallvec::SmallVec::<[u8; 256]>::with_capacity(spec.len());
                        buf.extend_from_slice(spec);
                        let glob_pos = buf.find_byte(b'*').expect("glob present");
                        buf[glob_pos] = b'a';
                        gix_validate::reference::name_partial(buf.as_bstr())?;
                    }
                } else {
                    gix_validate::reference::name_partial(spec)
                        .map_err(Error::from)
                        .or_else(|err| {
                            if allow_revspecs {
                                gix_revision::spec::parse(spec, &mut super::revparse::Noop)?;
                                Ok(spec)
                            } else {
                                Err(err)
                            }
                        })?;
                }
                Ok((Some(spec), has_globs))
            }
            None => Ok((None, false)),
        }
    }
}

mod revparse {
    use bstr::BStr;
    use gix_revision::spec::parse::delegate::{
        Kind, Navigate, PeelTo, PrefixHint, ReflogLookup, Revision, SiblingBranch, Traversal,
    };

    pub(crate) struct Noop;

    impl Revision for Noop {
        fn find_ref(&mut self, _name: &BStr) -> Option<()> {
            Some(())
        }

        fn disambiguate_prefix(&mut self, _prefix: gix_hash::Prefix, _hint: Option<PrefixHint<'_>>) -> Option<()> {
            Some(())
        }

        fn reflog(&mut self, _query: ReflogLookup) -> Option<()> {
            Some(())
        }

        fn nth_checked_out_branch(&mut self, _branch_no: usize) -> Option<()> {
            Some(())
        }

        fn sibling_branch(&mut self, _kind: SiblingBranch) -> Option<()> {
            Some(())
        }
    }

    impl Navigate for Noop {
        fn traverse(&mut self, _kind: Traversal) -> Option<()> {
            Some(())
        }

        fn peel_until(&mut self, _kind: PeelTo<'_>) -> Option<()> {
            Some(())
        }

        fn find(&mut self, _regex: &BStr, _negated: bool) -> Option<()> {
            Some(())
        }

        fn index_lookup(&mut self, _path: &BStr, _stage: u8) -> Option<()> {
            Some(())
        }
    }

    impl Kind for Noop {
        fn kind(&mut self, _kind: gix_revision::spec::Kind) -> Option<()> {
            Some(())
        }
    }

    impl gix_revision::spec::parse::Delegate for Noop {
        fn done(&mut self) {}
    }
}
