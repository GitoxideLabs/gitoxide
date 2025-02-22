use std::io::Write;

use crate::{entry, extension, write::util::CountBytes, State, Version};

/// A way to specify which of the optional extensions to write.
#[derive(Default, Debug, Copy, Clone)]
pub enum Extensions {
    /// Writes all available optional extensions to avoid losing any information.
    #[default]
    All,
    /// Only write the given optional extensions, with each extension being marked by a boolean flag.
    ///
    /// # Note: mandatory extensions
    ///
    /// Mandatory extensions, like `sdir` or other lower-case ones, may not be configured here as they need to be present
    /// or absent depending on the state of the index itself and for it to be valid.
    Given {
        /// Write the tree-cache extension, if present.
        tree_cache: bool,
        /// Write the end-of-index-entry extension.
        end_of_index_entry: bool,
    },
    /// Write no optional extension at all for what should be the smallest possible index
    None,
}

impl Extensions {
    /// Returns `Some(signature)` if it should be written out.
    pub fn should_write(&self, signature: extension::Signature) -> Option<extension::Signature> {
        match self {
            Extensions::None => None,
            Extensions::All => Some(signature),
            Extensions::Given {
                tree_cache,
                end_of_index_entry,
            } => match signature {
                extension::tree::SIGNATURE => tree_cache,
                extension::end_of_index_entry::SIGNATURE => end_of_index_entry,
                _ => &false,
            }
            .then(|| signature),
        }
    }
}

/// The options for use when [writing an index][State::write_to()].
///
/// Note that default options write either index V2 or V3 depending on the content of the entries.
#[derive(Debug, Default, Clone, Copy)]
pub struct Options {
    /// Configures which extensions to write.
    pub extensions: Extensions,
    /// Set the trailing hash of the produced index to all zeroes to save some time.
    ///
    /// This value is typically controlled by `index.skipHash` and is respected when the index is written
    /// via [`File::write()`](crate::File::write()) and [`File::write_to()`](crate::File::write_to()).
    /// Note that
    pub skip_hash: bool,
}

impl State {
    /// Serialize this instance to `out` with [`options`][Options].
    pub fn write_to(
        &self,
        out: impl std::io::Write,
        Options {
            extensions,
            skip_hash: _,
        }: Options,
    ) -> std::io::Result<Version> {
        let _span = gix_features::trace::detail!("gix_index::State::write()");
        let version = self.detect_required_version();

        let mut write = CountBytes::new(out);
        let num_entries: u32 = self
            .entries()
            .len()
            .try_into()
            .expect("definitely not 4billion entries");
        let removed_entries: u32 = self
            .entries()
            .iter()
            .filter(|e| e.flags.contains(entry::Flags::REMOVE))
            .count()
            .try_into()
            .expect("definitely not too many entries");

        let offset_to_entries = header(&mut write, version, num_entries - removed_entries)?;
        let offset_to_extensions = entries(&mut write, self, offset_to_entries)?;
        let (extension_toc, out) = self.write_extensions(write, offset_to_extensions, extensions)?;

        if num_entries > 0
            && extensions
                .should_write(extension::end_of_index_entry::SIGNATURE)
                .is_some()
            && !extension_toc.is_empty()
        {
            extension::end_of_index_entry::write_to(out, self.object_hash, offset_to_extensions, extension_toc)?;
        }

        Ok(version)
    }

    fn write_extensions<T>(
        &self,
        mut write: CountBytes<T>,
        offset_to_extensions: u32,
        extensions: Extensions,
    ) -> std::io::Result<(Vec<(extension::Signature, u32)>, T)>
    where
        T: std::io::Write,
    {
        type WriteExtFn<'a> = &'a dyn Fn(&mut dyn std::io::Write) -> Option<std::io::Result<extension::Signature>>;
        let extensions: &[WriteExtFn<'_>] = &[
            &|write| {
                extensions
                    .should_write(extension::tree::SIGNATURE)
                    .and_then(|signature| self.tree().map(|tree| tree.write_to(write).map(|_| signature)))
            },
            &|write| {
                self.is_sparse()
                    .then(|| extension::sparse::write_to(write).map(|_| extension::sparse::SIGNATURE))
            },
        ];

        let mut offset_to_previous_ext = offset_to_extensions;
        let mut out = Vec::with_capacity(5);
        for write_ext in extensions {
            if let Some(signature) = write_ext(&mut write).transpose()? {
                let offset_past_ext = write.count;
                let ext_size = offset_past_ext - offset_to_previous_ext - (extension::MIN_SIZE as u32);
                offset_to_previous_ext = offset_past_ext;
                out.push((signature, ext_size));
            }
        }
        Ok((out, write.inner))
    }
}

impl State {
    fn detect_required_version(&self) -> Version {
        self.entries
            .iter()
            .find_map(|e| e.flags.contains(entry::Flags::EXTENDED).then_some(Version::V3))
            .unwrap_or(Version::V2)
    }
}

fn header<T: std::io::Write>(
    out: &mut CountBytes<T>,
    version: Version,
    num_entries: u32,
) -> Result<u32, std::io::Error> {
    let version = match version {
        Version::V2 => 2_u32.to_be_bytes(),
        Version::V3 => 3_u32.to_be_bytes(),
        Version::V4 => 4_u32.to_be_bytes(),
    };

    out.write_all(crate::decode::header::SIGNATURE)?;
    out.write_all(&version)?;
    out.write_all(&num_entries.to_be_bytes())?;

    Ok(out.count)
}

fn entries<T: std::io::Write>(out: &mut CountBytes<T>, state: &State, header_size: u32) -> Result<u32, std::io::Error> {
    for entry in state.entries() {
        if entry.flags.contains(entry::Flags::REMOVE) {
            continue;
        }
        entry.write_to(&mut *out, state)?;
        match (out.count - header_size) % 8 {
            0 => {}
            n => {
                let eight_null_bytes = [0u8; 8];
                out.write_all(&eight_null_bytes[n as usize..])?;
            }
        }
    }

    Ok(out.count)
}

mod util {
    pub struct CountBytes<T> {
        pub count: u32,
        pub inner: T,
    }

    impl<T> CountBytes<T>
    where
        T: std::io::Write,
    {
        pub fn new(inner: T) -> Self {
            CountBytes { inner, count: 0 }
        }
    }

    impl<T> std::io::Write for CountBytes<T>
    where
        T: std::io::Write,
    {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let written = self.inner.write(buf)?;
            self.count = self
                .count
                .checked_add(u32::try_from(written).expect("we don't write 4GB buffers"))
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Cannot write indices larger than 4 gigabytes",
                    )
                })?;
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }
}
