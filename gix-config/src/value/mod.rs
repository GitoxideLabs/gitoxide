pub use gix_config_value::Error;

mod normalize;
pub use normalize::normalize;

/// Git's locale-independent definition of whitespace.
pub(crate) fn is_git_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}
