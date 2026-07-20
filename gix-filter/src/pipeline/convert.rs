use std::{io::Read, path::Path};

use bstr::BStr;

use crate::{Pipeline, driver, eol, ident, pipeline::util::Configuration, worktree};

///
pub mod configuration {
    use bstr::BString;

    /// Errors related to the configuration of filter attributes.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        UnknownEncoding { name: BString },
        InvalidEncoding,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::UnknownEncoding { name } => write!(f, "The encoding named '{name}' isn't available"),
                Error::InvalidEncoding => f.write_str("Encodings must be names, like UTF-16, and cannot be booleans."),
            }
        }
    }

    impl std::error::Error for Error {}
}

///
pub mod to_git {
    /// A function that fills `buf` `fn(&mut buf)` with the data stored in the index of the file that should be converted.
    pub type IndexObjectFn<'a> = dyn FnMut(&mut Vec<u8>) -> Result<Option<()>, gix_object::find::Error> + 'a;

    /// The error returned by [Pipeline::convert_to_git()][super::Pipeline::convert_to_git()].
    // TODO(review): these implementations hand-preserve `#[error(transparent)]` semantics for the
    //                first four variants: `Display` passes the formatter through and `source()`
    //                forwards to the inner error's source, exactly like the `thiserror`-generated
    //                code did. The same pattern is used in `to_worktree::Error` below, for
    //                `driver::apply::Error::Init`, and for the `PacketlineDecode` variants in
    //                `driver::process::client` and `driver::process::server`.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Eol(crate::eol::convert_to_git::Error),
        Worktree(crate::worktree::encode_to_git::Error),
        Driver(crate::driver::apply::Error),
        Configuration(super::configuration::Error),
        ReadProcessOutputToBuffer(std::io::Error),
        OutOfMemory(std::collections::TryReserveError),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Eol(err) => std::fmt::Display::fmt(err, f),
                Error::Worktree(err) => std::fmt::Display::fmt(err, f),
                Error::Driver(err) => std::fmt::Display::fmt(err, f),
                Error::Configuration(err) => std::fmt::Display::fmt(err, f),
                Error::ReadProcessOutputToBuffer(_) => f.write_str("Copy of driver process output to memory failed"),
                Error::OutOfMemory(_) => f.write_str("Could not allocate buffer"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Eol(err) => err.source(),
                Error::Worktree(err) => err.source(),
                Error::Driver(err) => err.source(),
                Error::Configuration(err) => err.source(),
                Error::ReadProcessOutputToBuffer(err) => Some(err),
                Error::OutOfMemory(err) => Some(err),
            }
        }
    }

    impl From<crate::eol::convert_to_git::Error> for Error {
        fn from(err: crate::eol::convert_to_git::Error) -> Self {
            Error::Eol(err)
        }
    }

    impl From<crate::worktree::encode_to_git::Error> for Error {
        fn from(err: crate::worktree::encode_to_git::Error) -> Self {
            Error::Worktree(err)
        }
    }

    impl From<crate::driver::apply::Error> for Error {
        fn from(err: crate::driver::apply::Error) -> Self {
            Error::Driver(err)
        }
    }

    impl From<super::configuration::Error> for Error {
        fn from(err: super::configuration::Error) -> Self {
            Error::Configuration(err)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::ReadProcessOutputToBuffer(err)
        }
    }

    impl From<std::collections::TryReserveError> for Error {
        fn from(err: std::collections::TryReserveError) -> Self {
            Error::OutOfMemory(err)
        }
    }
}

///
pub mod to_worktree {
    use crate::driver;

    /// Options for converting Git data to its worktree representation.
    #[derive(Default, Debug, Copy, Clone)]
    pub struct Options {
        /// Whether process filters may delay their response.
        pub can_delay: driver::apply::Delay,
        /// How to handle a configured worktree encoding that isn't available or cannot encode the input.
        pub unknown_encoding: UnknownEncoding,
    }

    /// How to handle a configured worktree encoding that isn't available or cannot encode the input.
    #[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
    pub enum UnknownEncoding {
        /// Emit a warning as trace, ignore the encoding, and leave prior conversions intact.
        #[default]
        Ignore,
        /// Return an error.
        Fail,
    }

    /// The error returned by [Pipeline::convert_to_worktree()][super::Pipeline::convert_to_worktree()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Ident(crate::ident::apply::Error),
        Eol(crate::eol::convert_to_worktree::Error),
        Worktree(crate::worktree::encode_to_worktree::Error),
        Driver(crate::driver::apply::Error),
        Configuration(super::configuration::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Ident(err) => std::fmt::Display::fmt(err, f),
                Error::Eol(err) => std::fmt::Display::fmt(err, f),
                Error::Worktree(err) => std::fmt::Display::fmt(err, f),
                Error::Driver(err) => std::fmt::Display::fmt(err, f),
                Error::Configuration(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Ident(err) => err.source(),
                Error::Eol(err) => err.source(),
                Error::Worktree(err) => err.source(),
                Error::Driver(err) => err.source(),
                Error::Configuration(err) => err.source(),
            }
        }
    }

    impl From<crate::ident::apply::Error> for Error {
        fn from(err: crate::ident::apply::Error) -> Self {
            Error::Ident(err)
        }
    }

    impl From<crate::eol::convert_to_worktree::Error> for Error {
        fn from(err: crate::eol::convert_to_worktree::Error) -> Self {
            Error::Eol(err)
        }
    }

    impl From<crate::worktree::encode_to_worktree::Error> for Error {
        fn from(err: crate::worktree::encode_to_worktree::Error) -> Self {
            Error::Worktree(err)
        }
    }

    impl From<crate::driver::apply::Error> for Error {
        fn from(err: crate::driver::apply::Error) -> Self {
            Error::Driver(err)
        }
    }

    impl From<super::configuration::Error> for Error {
        fn from(err: super::configuration::Error) -> Self {
            Error::Configuration(err)
        }
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
            false,
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
    /// [`Options::can_delay`](to_worktree::Options::can_delay) defines if long-running processes can delay their response, and if they *choose* to the caller has to
    /// specifically deal with it by interacting with the [`driver_state`][Pipeline::driver_state_mut()] directly.
    ///
    /// The reason `src` is a buffer is to indicate that `git` generally doesn't do well streaming data, so it should be small enough
    /// to be performant while being held in memory. This is typically the case, especially if `git-lfs` is used as intended.
    pub fn convert_to_worktree<'input>(
        &mut self,
        src: &'input [u8],
        rela_path: &BStr,
        attributes: &mut dyn FnMut(&BStr, &mut gix_attributes::search::Outcome),
        to_worktree::Options {
            can_delay,
            unknown_encoding,
        }: to_worktree::Options,
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
            unknown_encoding == to_worktree::UnknownEncoding::Ignore,
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
            match worktree::encode_to_worktree(src, encoding, dest) {
                Ok(()) => bufs.swap(),
                Err(_err) if unknown_encoding == to_worktree::UnknownEncoding::Ignore => {
                    gix_trace::warn!(err = %_err, "Ignoring failed worktree encoding");
                }
                Err(err) => return Err(err.into()),
            }
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
