use std::{
    io,
    path::{Path, PathBuf},
};

/// Keep the marker present until its notification is consumed, even across native coalescing.
pub(super) struct Cookie {
    pub(super) path: PathBuf,
}

impl Cookie {
    pub(super) fn create(directory: &Path) -> io::Result<Self> {
        let mut nonce = [0; 16];
        getrandom::fill(&mut nonce).map_err(io::Error::other)?;
        let path = directory.join(format!("gix-{:032x}", u128::from_ne_bytes(nonce)));
        // Exclusive creation neither overwrites stale cookies nor follows an existing symlink.
        // A fresh random name prevents delayed events from satisfying a later request.
        std::fs::File::create_new(&path)?;
        Ok(Self { path })
    }
}

impl Drop for Cookie {
    fn drop(&mut self) {
        if let Err(_error) = std::fs::remove_file(&self.path) {
            gix_trace::warn!(error = %_error, "could not remove filesystem synchronization cookie");
        }
    }
}
