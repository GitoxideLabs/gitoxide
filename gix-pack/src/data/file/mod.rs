mod init;
///
pub mod verify;

///
pub mod decode;

/// The bytes used as header in a pack data file.
pub type Header = [u8; super::header::SIZE];
