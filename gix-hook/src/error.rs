/// The error type returned by this crate's fallible functions.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Returned by [`update_args()`](crate::update_args()), [`receive_stdin()`](crate::receive_stdin()),
    /// and [`reference_transaction_stdin()`](crate::reference_transaction_stdin()) when a ref name
    /// isn't one git itself could have produced.
    #[error(transparent)]
    InvalidRefName(#[from] gix_validate::reference::name::Error),
    /// Returned by [`hooks_path_from_config()`](crate::hooks_path_from_config()) when
    /// `core.hooksPath` can't be interpolated into a usable path.
    #[error(transparent)]
    ConfigPathInterpolate(#[from] gix_config::path::interpolate::Error),
    /// Returned by [`run()`](crate::run()) when the hook process couldn't be spawned or waited on.
    #[error(transparent)]
    Spawn(#[from] std::io::Error),
    /// Returned by [`pre_push_stdin()`](crate::pre_push_stdin()) when `local_ref` contains a
    /// control byte (`\0..=\x1F` or `\x7F`) - the same byte range
    /// [`gix_validate::reference::name()`] rejects, applied here even though `local_ref` isn't
    /// validated as a full ref name, since a raw newline would corrupt this function's own line
    /// framing and a raw NUL would otherwise reach argv/env unremarked.
    #[error("hook stdin field must not contain a control byte: {0:?}")]
    ControlByteInField(String),
    /// Returned by this crate's stdin parsers when a line doesn't have the expected
    /// space-separated field shape.
    #[error("malformed hook stdin line: {0:?}")]
    MalformedLine(String),
    /// Returned by this crate's stdin parsers when a field expected to be a hex object id isn't.
    #[error(transparent)]
    InvalidOid(#[from] gix_hash::decode::Error),
}
