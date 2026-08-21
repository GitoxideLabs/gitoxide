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
}
