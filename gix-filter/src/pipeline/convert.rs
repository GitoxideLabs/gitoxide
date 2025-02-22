use std::{io::Read, path::Path};

use bstr::BStr;

use crate::{driver, eol, ident, pipeline::util::Configuration, worktree, Pipeline};

///
pub mod configuration {
    use bstr::BString;

    /// Errors related to the configuration of filter attributes.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The encoding named '{name}' isn't available")]
        UnknownEncoding { name: BString },
        #[error("Encodings must be names, like UTF-16, and cannot be booleans.")]
        InvalidEncoding,
    }
}

///
pub mod to_git {
    /// A function that fills `buf` `fn(&mut buf)` with the data stored in the index of the file that should be converted.
    pub type IndexObjectFn<'a> = dyn FnMut(&mut Vec<u8>) -> Result<Option<()>, gix_object::find::Error> + 'a;

    /// The error returned by [Pipeline::convert_to_git()][super::Pipeline::convert_to_git()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Eol(#[from] crate::eol::convert_to_git::Error),
        #[error(transparent)]
        Worktree(#[from] crate::worktree::encode_to_git::Error),
        #[error(transparent)]
        Driver(#[from] crate::driver::apply::Error),
        #[error(transparent)]
        Configuration(#[from] super::configuration::Error),
        #[error("Copy of driver process output to memory failed")]
        ReadProcessOutputToBuffer(#[from] std::io::Error),
        #[error("Could not allocate buffer")]
        OutOfMemory(#[from] std::collections::TryReserveError),
    }
}

///
pub mod to_worktree {
    /// The error returned by [Pipeline::convert_to_worktree()][super::Pipeline::convert_to_worktree()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Worktree(#[from] crate::worktree::encode_to_worktree::Error),
        #[error(transparent)]
        Driver(#[from] crate::driver::apply::Error),
        #[error(transparent)]
        Configuration(#[from] super::configuration::Error),
        #[error("Could not allocate buffer")]
        OutOfMemory(#[from] std::collections::TryReserveError),
    }
}

/// Access
impl Pipeline {
    /// Convert a `src` stream (to be found at `rela_path`) to a representation suitable for storage in `git`
    /// based on the `attributes` at `rela_path` which is passed as first argument..
    /// When converting to `crlf`, and depending on the configuration, `index_object` might be called to obtain the index
    /// version of `src` if available. It can return `Ok(None)` if this information isn't available.
    pub fn convert_to_git<R>(
        &mut self,
        mut src: R,
        rela_path: &Path,
        attributes: &mut dyn FnMut(&BStr, &mut gix_attributes::search::Outcome),
        index_object: &mut to_git::IndexObjectFn<'_>,
    ) -> Result<ToGitOutcome<'_, R>, to_git::Error>
    where
        R: std::io::Read,
    {
        let bstr_rela_path = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(rela_path));
        let Configuration {
            driver,
            digest,
            _attr_digest: _,
            encoding,
            apply_ident_filter,
        } = Configuration::at_path(
            bstr_rela_path.as_ref(),
            &self.options.drivers,
            &mut self.attrs,
            attributes,
            self.options.eol_config,
        )?;

        let mut in_src_buffer = false;
        // this is just an approximation, but it's as good as it gets without reading the actual input.
        let would_convert_eol = eol::convert_to_git(
            b"\r\n",
            digest,
            &mut self.bufs.dest,
            &mut |_| Ok(None),
            eol::convert_to_git::Options {
                round_trip_check: None,
                config: self.options.eol_config,
            },
        )?;

        if let Some(driver) = driver {
            if let Some(mut read) = self.processes.apply(
                driver,
                &mut src,
                driver::Operation::Clean,
                self.context.with_path(bstr_rela_path.as_ref()),
            )? {
                if !apply_ident_filter && encoding.is_none() && !would_convert_eol {
                    // Note that this is not typically a benefit in terms of saving memory as most filters
                    // aren't expected to make the output file larger. It's more about who is waiting for the filter's
                    // output to arrive, which won't be us now. For `git-lfs` it definitely won't matter though.
                    return Ok(ToGitOutcome::Process(read));
                }
                self.bufs.clear();
                read.read_to_end(&mut self.bufs.src)?;
                in_src_buffer = true;
            }
        }
        if !in_src_buffer && (apply_ident_filter || encoding.is_some() || would_convert_eol) {
            self.bufs.clear();
            src.read_to_end(&mut self.bufs.src)?;
            in_src_buffer = true;
        }

        if let Some(encoding) = encoding {
            worktree::encode_to_git(
                &self.bufs.src,
                encoding,
                &mut self.bufs.dest,
                if self.options.encodings_with_roundtrip_check.contains(&encoding) {
                    worktree::encode_to_git::RoundTripCheck::Fail
                } else {
                    worktree::encode_to_git::RoundTripCheck::Skip
                },
            )?;
            self.bufs.swap();
        }

        if eol::convert_to_git(
            &self.bufs.src,
            digest,
            &mut self.bufs.dest,
            &mut |buf| index_object(buf),
            eol::convert_to_git::Options {
                round_trip_check: self.options.crlf_roundtrip_check.to_eol_roundtrip_check(rela_path),
                config: self.options.eol_config,
            },
        )? {
            self.bufs.swap();
        }

        if apply_ident_filter && ident::undo(&self.bufs.src, &mut self.bufs.dest)? {
            self.bufs.swap();
        }
        Ok(if in_src_buffer {
            ToGitOutcome::Buffer(&self.bufs.src)
        } else {
            ToGitOutcome::Unchanged(src)
        })
    }

    /// Convert a `src` buffer located at `rela_path` (in the index) from what's in `git` to the worktree representation,
    /// asking for `attributes` with `rela_path` as first argument to configure the operation automatically.
    /// `can_delay` defines if long-running processes can delay their response, and if they *choose* to the caller has to
    /// specifically deal with it by interacting with the [`driver_state`][Pipeline::driver_state_mut()] directly.
    ///
    /// The reason `src` is a buffer is to indicate that `git` generally doesn't do well streaming data, so it should be small enough
    /// to be performant while being held in memory. This is typically the case, especially if `git-lfs` is used as intended.
    pub fn convert_to_worktree<'input>(
        &mut self,
        src: &'input [u8],
        rela_path: &BStr,
        attributes: &mut dyn FnMut(&BStr, &mut gix_attributes::search::Outcome),
        can_delay: driver::apply::Delay,
    ) -> Result<ToWorktreeOutcome<'input, '_>, to_worktree::Error> {
        let Configuration {
            driver,
            digest,
            _attr_digest: _,
            encoding,
            apply_ident_filter,
        } = Configuration::at_path(
            rela_path,
            &self.options.drivers,
            &mut self.attrs,
            attributes,
            self.options.eol_config,
        )?;

        let mut bufs = self.bufs.use_foreign_src(src);
        let (src, dest) = bufs.src_and_dest();
        if apply_ident_filter && ident::apply(src, self.options.object_hash, dest)? {
            bufs.swap();
        }

        let (src, dest) = bufs.src_and_dest();
        if eol::convert_to_worktree(src, digest, dest, self.options.eol_config)? {
            bufs.swap();
        }

        if let Some(encoding) = encoding {
            let (src, dest) = bufs.src_and_dest();
            worktree::encode_to_worktree(src, encoding, dest)?;
            bufs.swap();
        }

        if let Some(driver) = driver {
            let (mut src, _dest) = bufs.src_and_dest();
            if let Some(maybe_delayed) = self.processes.apply_delayed(
                driver,
                &mut src,
                driver::Operation::Smudge,
                can_delay,
                self.context.with_path(rela_path),
            )? {
                return Ok(ToWorktreeOutcome::Process(maybe_delayed));
            }
        }

        Ok(match bufs.ro_src {
            Some(src) => ToWorktreeOutcome::Unchanged(src),
            None => ToWorktreeOutcome::Buffer(bufs.src),
        })
    }
}

/// The result of a conversion with zero or more filters to be stored in git.
pub enum ToGitOutcome<'pipeline, R> {
    /// The original input wasn't changed and the reader is still available for consumption.
    Unchanged(R),
    /// An external filter (and only that) was applied and its results *have to be consumed*.
    Process(Box<dyn std::io::Read + 'pipeline>),
    /// A reference to the result of one or more filters of which one didn't support streaming.
    ///
    /// This can happen if an `eol`, `working-tree-encoding` or `ident` filter is applied, possibly on top of an external filter.
    Buffer(&'pipeline [u8]),
}

/// The result of a conversion with zero or more filters.
///
/// ### Panics
///
/// If `std::io::Read` is used on it and the output is delayed, a panic will occur. The caller is responsible for either disallowing delayed
/// results or if allowed, handle them. Use [`is_delayed()][Self::is_delayed()].
pub enum ToWorktreeOutcome<'input, 'pipeline> {
    /// The original input wasn't changed and the original buffer is present
    Unchanged(&'input [u8]),
    /// A reference to the result of one or more filters of which one didn't support streaming.
    ///
    /// This can happen if an `eol`, `working-tree-encoding` or `ident` filter is applied, possibly on top of an external filter.
    Buffer(&'pipeline [u8]),
    /// An external filter (and only that) was applied and its results *have to be consumed*. Note that the output might be delayed,
    /// which requires special handling to eventually receive it.
    Process(driver::apply::MaybeDelayed<'pipeline>),
}

impl ToWorktreeOutcome<'_, '_> {
    /// Return true if this outcome is delayed. In that case, one isn't allowed to use [`Read`] or cause a panic.
    pub fn is_delayed(&self) -> bool {
        matches!(
            self,
            ToWorktreeOutcome::Process(driver::apply::MaybeDelayed::Delayed(_))
        )
    }

    /// Returns `true` if the input buffer was actually changed, or `false` if it is returned directly.
    pub fn is_changed(&self) -> bool {
        !matches!(self, ToWorktreeOutcome::Unchanged(_))
    }

    /// Return a buffer if we contain one, or `None` otherwise.
    ///
    /// This method is useful only if it's clear that no driver is available, which may cause a stream to be returned and not a buffer.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ToWorktreeOutcome::Unchanged(b) | ToWorktreeOutcome::Buffer(b) => Some(b),
            ToWorktreeOutcome::Process(_) => None,
        }
    }

    /// Return a stream to read the drivers output from, if possible.
    ///
    /// Note that this is only the case if the driver process was applied last *and* didn't delay its output.
    pub fn as_read(&mut self) -> Option<&mut (dyn std::io::Read + '_)> {
        match self {
            ToWorktreeOutcome::Process(driver::apply::MaybeDelayed::Delayed(_))
            | ToWorktreeOutcome::Unchanged(_)
            | ToWorktreeOutcome::Buffer(_) => None,
            ToWorktreeOutcome::Process(driver::apply::MaybeDelayed::Immediate(read)) => Some(read),
        }
    }
}

impl std::io::Read for ToWorktreeOutcome<'_, '_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ToWorktreeOutcome::Unchanged(b) => b.read(buf),
            ToWorktreeOutcome::Buffer(b) => b.read(buf),
            ToWorktreeOutcome::Process(driver::apply::MaybeDelayed::Delayed(_)) => {
                panic!("BUG: must not try to read delayed output")
            }
            ToWorktreeOutcome::Process(driver::apply::MaybeDelayed::Immediate(r)) => r.read(buf),
        }
    }
}

impl<R> std::io::Read for ToGitOutcome<'_, R>
where
    R: std::io::Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            ToGitOutcome::Unchanged(r) => r.read(buf),
            ToGitOutcome::Process(r) => r.read(buf),
            ToGitOutcome::Buffer(r) => r.read(buf),
        }
    }
}

impl<'a, R> ToGitOutcome<'a, R>
where
    R: std::io::Read,
{
    /// If we contain a buffer, and not a stream, return it.
    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            ToGitOutcome::Unchanged(_) | ToGitOutcome::Process(_) => None,
            ToGitOutcome::Buffer(b) => Some(b),
        }
    }

    /// Return a stream to read the drivers output from. This is only possible if there is only a driver, and no other filter.
    pub fn as_read(&mut self) -> Option<&mut (dyn std::io::Read + '_)> {
        match self {
            ToGitOutcome::Process(read) => Some(read),
            ToGitOutcome::Unchanged(read) => Some(read),
            ToGitOutcome::Buffer(_) => None,
        }
    }

    /// Returns `true` if the input buffer was actually changed, or `false` if it is returned directly.
    pub fn is_changed(&self) -> bool {
        !matches!(self, ToGitOutcome::Unchanged(_))
    }
}
