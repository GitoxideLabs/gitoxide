use std::path::Path;

/// Additional context for use with [`convert_to_git`][super::convert_to_git()].
#[derive(Default, Copy, Clone)]
pub struct Options<'a> {
    /// How to perform round-trip checks.
    pub round_trip_check: Option<RoundTripCheck<'a>>,
    /// Configuration related to EOL.
    pub config: crate::eol::Configuration,
}

/// The kind of round-trip check to perform when converting line endings to `git`, i.e. `CRLF` to `LF`.
#[derive(Debug, Copy, Clone)]
pub enum RoundTripCheck<'a> {
    /// Fail with an error if conversion isn't round-trip safe.
    Fail {
        /// The repository-relative path of the file to check. Used in case of error.
        rela_path: &'a Path,
    },
    /// Emit a warning using `gix_trace::warn!`, but don't fail.
    ///
    /// Note that the parent application has to setup tracing to make these events visible, along with a parent `span!`.
    Warn {
        /// The repository-relative path of the file to check. Used in case of error.
        rela_path: &'a Path,
    },
}

/// The error returned by [convert_to_git()][super::convert_to_git()].
pub type Error = gix_error::Exn<gix_error::Message>;

/// A function that writes a buffer like `fn(&mut buf)` with by tes of an object in the index that is the one that should be converted.
pub type IndexObjectFn<'a> = dyn FnMut(&mut Vec<u8>) -> Result<Option<()>, gix_error::Exn> + 'a;

pub(crate) mod function {
    use bstr::ByteSlice;

    use crate::{
        clear_and_set_capacity,
        eol::{
            AttributesDigest, Stats,
            convert_to_git::{Error, IndexObjectFn, Options, RoundTripCheck},
        },
    };

    /// Given a `src` buffer, change it `git` (`\n`) line endings and store the result in `buf`.
    /// Return `true` if `buf` was written or `false` if nothing had to be done.
    /// Depending on the state in `buf`, `index_object` is called to write the version of `src` as stored in the index
    /// into the buffer and if it is a blob, or return `Ok(None)` if no such object exists.
    ///
    /// *If renormalization is desired*, let it return `Ok(None)` at all times to not let it have any influence over the
    /// outcome of this function. Otherwise, it will check if the in-index buffer already has newlines that it would now
    /// want to change, and avoid doing so as what's in Git should be what's desired (except for when *renormalizing*).
    /// If `round_trip_check` is not `None`, round-tripping will be validated and handled accordingly.
    pub fn convert_to_git(
        src: &[u8],
        digest: AttributesDigest,
        buf: &mut Vec<u8>,
        index_object: &mut IndexObjectFn<'_>,
        Options {
            round_trip_check,
            config,
        }: Options<'_>,
    ) -> Result<bool, Error> {
        use gix_error::{ErrorExt, ResultExt, message};

        if digest == AttributesDigest::Binary || src.is_empty() {
            return Ok(false);
        }

        let stats = Stats::from_bytes(src);
        let mut convert_crlf_to_lf = stats.crlf > 0;
        if digest.is_auto_text() {
            // In this mode, we are supposed to figure out ourselves if we should convert or not.
            if stats.is_binary() {
                return Ok(false);
            }

            if let Some(()) =
                index_object(buf).or_raise(|| message("Could not obtain index object to check line endings for"))?
            {
                let has_crlf_in_index = buf
                    .find_byte(b'\r')
                    .map(|_| Stats::from_bytes(buf))
                    .filter(|s| !s.is_binary() && s.crlf > 0)
                    .is_some();
                if has_crlf_in_index {
                    convert_crlf_to_lf = false;
                }
            }
        }

        if let Some(round_trip_check) = round_trip_check {
            let mut new_stats = stats;
            // simulate to-git conversion/git-add
            if convert_crlf_to_lf {
                new_stats.lone_lf += new_stats.crlf;
                new_stats.crlf = 0;
            }
            // simulate worktree checkout
            if new_stats.will_convert_lf_to_crlf(digest, config) {
                new_stats.crlf += new_stats.lone_lf;
                new_stats.lone_lf = 0;
            }
            if stats.crlf > 0 && new_stats.crlf == 0 {
                // CRLF would not be restored by checkout
                match round_trip_check {
                    RoundTripCheck::Fail { rela_path } => {
                        return Err(message!("CRLF would be replaced by LF in '{}'", rela_path.display()).raise());
                    }
                    #[allow(unused_variables, reason = "Used when tracing is enabled at compile time.")]
                    RoundTripCheck::Warn { rela_path } => {
                        gix_trace::warn!(
                            "in the working copy of '{}', CRLF will be replaced by LF next time git touches it",
                            rela_path.display()
                        );
                    }
                }
            } else if stats.lone_lf > 0 && new_stats.lone_lf == 0 {
                // CRLF would be added by checkout
                match round_trip_check {
                    RoundTripCheck::Fail { rela_path } => {
                        return Err(message!("LF would be replaced by CRLF in '{}'", rela_path.display()).raise());
                    }
                    #[allow(unused_variables, reason = "Used when tracing is enabled at compile time.")]
                    RoundTripCheck::Warn { rela_path } => {
                        gix_trace::warn!(
                            "in the working copy of '{}', LF will be replaced by CRLF next time git touches it",
                            rela_path.display()
                        );
                    }
                }
            }
        }

        if !convert_crlf_to_lf {
            return Ok(false);
        }

        clear_and_set_capacity(buf, src.len() - stats.crlf).or_raise(|| message("Could not allocate buffer"))?;
        if stats.lone_cr == 0 {
            buf.extend(src.iter().filter(|b| **b != b'\r'));
        } else {
            let mut bytes = src.iter().peekable();
            while let Some(b) = bytes.next() {
                if !(*b == b'\r' && bytes.peek() == Some(&&b'\n')) {
                    buf.push(*b);
                }
            }
        }
        Ok(true)
    }
}
