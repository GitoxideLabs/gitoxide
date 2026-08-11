//! Sign and verify commits with Git-compatible external programs.

/// Commit signing with fully detailed options.
pub mod sign;
/// Commit signature verification with detailed options.
pub mod verify;

/// The format of the signature to create.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Format {
    /// An OpenPGP signature made with `gpg` by default.
    OpenPgp,
    /// An X.509 signature made with `gpgsm` by default.
    X509,
    /// An SSH signature made with `ssh-keygen` by default.
    Ssh,
}

impl Format {
    /// Detect the format from the signature's armor header, or return `None` if it is unsupported.
    pub fn from_signature(signature: &[u8]) -> Option<Self> {
        if signature.starts_with(b"-----BEGIN PGP SIGNATURE-----")
            || signature.starts_with(b"-----BEGIN PGP MESSAGE-----")
        {
            Some(Format::OpenPgp)
        } else if signature.starts_with(b"-----BEGIN SIGNED MESSAGE-----") {
            Some(Format::X509)
        } else if signature.starts_with(b"-----BEGIN SSH SIGNATURE-----") {
            Some(Format::Ssh)
        } else {
            None
        }
    }
}
