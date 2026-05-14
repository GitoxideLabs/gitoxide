pub use gix_features::fs::read_dir::DirEntry;

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
