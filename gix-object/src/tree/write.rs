use std::io;

use bstr::{BString, ByteSlice};

use crate::{
    encode::SPACE,
    tree::{Entry, EntryRef},
    Kind, Tree, TreeRef,
};

/// The Error used in [`Tree::write_to()`][crate::WriteTo::write_to()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Nullbytes are invalid in file paths as they are separators: {name:?}")]
    NullbyteInFilename { name: BString },
}

impl From<Error> for io::Error {
    fn from(err: Error) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

/// Serialization
impl crate::WriteTo for Tree {
    /// Serialize this tree to `out` in the git internal format.
    fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        debug_assert_eq!(
            &self.entries,
            &{
                let mut entries_sorted = self.entries.clone();
                entries_sorted.sort();
                entries_sorted
            },
            "entries for serialization must be sorted by filename"
        );
        let mut buf = Default::default();
        for Entry { mode, filename, oid } in &self.entries {
            out.write_all(mode.as_bytes(&mut buf))?;
            out.write_all(SPACE)?;

            if filename.find_byte(0).is_some() {
                return Err(Error::NullbyteInFilename {
                    name: (*filename).to_owned(),
                }
                .into());
            }
            out.write_all(filename)?;
            out.write_all(b"\0")?;

            out.write_all(oid.as_bytes())?;
        }
        Ok(())
    }

    fn kind(&self) -> Kind {
        Kind::Tree
    }

    fn size(&self) -> u64 {
        let mut buf = Default::default();
        self.entries
            .iter()
            .map(|Entry { mode, filename, oid }| {
                (mode.as_bytes(&mut buf).len() + 1 + filename.len() + 1 + oid.as_bytes().len()) as u64
            })
            .sum()
    }
}

/// Serialization
impl crate::WriteTo for TreeRef<'_> {
    /// Serialize this tree to `out` in the git internal format.
    fn write_to(&self, out: &mut dyn io::Write) -> io::Result<()> {
        debug_assert_eq!(
            &{
                let mut entries_sorted = self.entries.clone();
                entries_sorted.sort();
                entries_sorted
            },
            &self.entries,
            "entries for serialization must be sorted by filename"
        );
        let mut buf = Default::default();
        for EntryRef { mode, filename, oid } in &self.entries {
            out.write_all(mode.as_bytes(&mut buf))?;
            out.write_all(SPACE)?;

            if filename.find_byte(0).is_some() {
                return Err(Error::NullbyteInFilename {
                    name: (*filename).to_owned(),
                }
                .into());
            }
            out.write_all(filename)?;
            out.write_all(b"\0")?;

            out.write_all(oid.as_bytes())?;
        }
        Ok(())
    }

    fn kind(&self) -> Kind {
        Kind::Tree
    }

    fn size(&self) -> u64 {
        let mut buf = Default::default();
        self.entries
            .iter()
            .map(|EntryRef { mode, filename, oid }| {
                (mode.as_bytes(&mut buf).len() + 1 + filename.len() + 1 + oid.as_bytes().len()) as u64
            })
            .sum()
    }
}
