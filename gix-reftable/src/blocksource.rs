use std::{path::Path, sync::Arc};

use crate::error::Error;

/// In-memory source for reading reftable blocks.
#[derive(Clone, Debug)]
pub struct BlockSource {
    data: Arc<[u8]>,
}

impl BlockSource {
    /// Open a block source from the given file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, Error> {
        let data = std::fs::read(path)?;
        Ok(Self {
            data: Arc::from(data.into_boxed_slice()),
        })
    }

    /// Create a source from owned bytes.
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data: Arc::from(data.into_boxed_slice()),
        }
    }

    /// Size in bytes.
    pub fn size(&self) -> u64 {
        self.data.len() as u64
    }

    /// Read a byte range.
    pub fn read(&self, offset: u64, size: u32) -> Result<&[u8], Error> {
        let start = usize::try_from(offset).map_err(|_| Error::Malformed("offset overflow"))?;
        if start >= self.data.len() {
            return Ok(&[]);
        }
        let end = start.saturating_add(size as usize).min(self.data.len());
        Ok(&self.data[start..end])
    }

    /// Access raw bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.data
    }
}
