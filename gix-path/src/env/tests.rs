use std::path::{Path, PathBuf};

pub(super) struct CurrentDir(PathBuf);

/// This is a copy from the respective type in `gix-testtools` - deduplicate if it can ever be a dependency again.
impl CurrentDir {
    pub(super) fn set(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let previous = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(CurrentDir(previous))
    }
}

impl Drop for CurrentDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("previous current directory is still accessible");
    }
}
