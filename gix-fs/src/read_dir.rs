use std::{borrow::Cow, ffi::OsStr, fs::FileType, path::Path};

/// A directory entry adding precompose-unicode support to [`std::fs::DirEntry`].
pub type DirEntry = crate::precompose::DirEntry<std::fs::DirEntry>;

impl crate::precompose::DirEntryApi for std::fs::DirEntry {
    fn path(&self) -> Cow<'_, Path> {
        self.path().into()
    }

    fn file_name(&self) -> Cow<'_, OsStr> {
        self.file_name().into()
    }

    fn file_type(&self) -> std::io::Result<FileType> {
        self.file_type()
    }
}

pub(crate) mod function {
    use std::path::Path;

    /// List all entries in `path`, similar to [`std::fs::read_dir()`], and assure all available information
    /// adheres to the value of `precompose_unicode`.
    pub fn read_dir(
        path: &Path,
        precompose_unicode: bool,
    ) -> std::io::Result<impl Iterator<Item = std::io::Result<super::DirEntry>> + use<>> {
        std::fs::read_dir(path)
            .map(move |it| it.map(move |res| res.map(|entry| super::DirEntry::new(entry, precompose_unicode))))
    }
}
