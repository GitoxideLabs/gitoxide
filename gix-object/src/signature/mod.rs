//! Discover, sign, and verify Git object signatures.

use std::ops::Range;

use bstr::{BStr, BString, ByteSlice};

/// Object signing with external-program options.
#[cfg(feature = "signature")]
pub mod sign;
/// Object signature verification with external-program options.
#[cfg(feature = "signature")]
pub mod verify;

/// A borrowed armored signature and its detected format.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SignatureRef<'a> {
    /// The signature format detected from the armor marker.
    pub format: Format,
    /// The complete armored signature, from its marker through end of object.
    pub data: &'a BStr,
}

/// Exact object bytes covered by a [signature](SignatureRef).
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SignedData<'a> {
    data: &'a [u8],
    excluded: Range<usize>,
}

impl<'a> SignedData<'a> {
    pub(crate) fn new(data: &'a [u8], excluded: Range<usize>) -> Self {
        SignedData { data, excluded }
    }

    pub(crate) fn segments(&self) -> [&[u8]; 2] {
        [&self.data[..self.excluded.start], &self.data[self.excluded.end..]]
    }

    /// Return an exact copy of the bytes covered by the signature.
    pub fn to_bstring(&self) -> BString {
        let [before, after] = self.segments();
        let mut out = BString::from(before);
        out.extend_from_slice(after);
        out
    }
}

impl From<SignedData<'_>> for BString {
    fn from(value: SignedData<'_>) -> Self {
        value.to_bstring()
    }
}

/// Find the last supported armor marker at a line boundary, matching Git's `parse_signed_buffer()`.
pub(crate) fn find(data: &[u8]) -> Option<(usize, Format)> {
    let mut found = None;
    let mut offset = 0;
    while offset < data.len() {
        if let Some(format) = Format::from_signature(&data[offset..]) {
            found = Some((offset, format));
        }
        offset = data[offset..]
            .find_byte(b'\n')
            .map_or(data.len(), |newline| offset + newline + 1);
    }
    found
}

/// A Git-supported signature format.
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
