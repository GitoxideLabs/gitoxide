use std::{
    io,
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, OptionExt, Result, ResultExt, message};
use gix_tempfile::{
    AutoRemove, ContainingDirectory, Handle,
    handle::{Closed, Writable},
};

use super::Outcome;

/// A nonempty pack and its index staged by [`crate::Bundle::prepare_to_directory()`].
///
/// Dropping this value discards both temporary files.
#[derive(Debug)]
#[must_use = "a prepared pack is discarded unless it is published"]
pub struct Prepared {
    index: crate::index::write::Outcome,
    files: StagedFiles,
    alloc_limit_bytes: Option<usize>,
}

impl Prepared {
    /// The checksums, pack size, and object count of the staged bundle.
    pub fn index(&self) -> &crate::index::write::Outcome {
        &self.index
    }

    /// Inspect the staged bundle before publication in freshly mapped files.
    /// They are unmapped when `inspect` returns.
    pub fn with_bundle<T>(&mut self, inspect: impl FnOnce(&crate::Bundle) -> Result<T>) -> Result<T> {
        let object_hash = self.index.data_hash.kind();
        let pack = self
            .files
            .data_file
            .with_mut(|file| {
                crate::data::File::at(file.path(), object_hash)
                    .map(|pack| pack.with_alloc_limit_bytes(self.alloc_limit_bytes))
            })
            .or_error()??;
        let index = self
            .files
            .index_file
            .with_mut(|file| crate::index::File::at(file.path(), object_hash))
            .or_error()??;
        inspect(&crate::Bundle { pack, index })
    }

    /// Exclusively acquire the destination `.keep`, then install the pack and its index.
    ///
    /// An existing marker causes an [`io::ErrorKind::AlreadyExists`] in the error chain and discards
    /// the staged files before either destination file is installed. This also applies to packs
    /// already present at the destination.
    ///
    /// On success, keep the returned guard alive until refs protect the objects. If installing the
    /// index fails after installing the pack file, the pack can be reused by a subsequent attempt.
    pub fn publish(self) -> Result<Published> {
        let keep_path = self.files.data_path.with_extension("keep");
        let keep = acquire_keep_file(&keep_path)
            .or_raise(|| message!("Could not acquire pack keep file at {}", keep_path.display()))?;
        let data_path = self.files.install(persist_noclobber, persist_noclobber)?;
        Ok(Published { data_path, _keep: keep })
    }
}

/// A published pack protected by a `.keep` marker owned by this value.
///
/// Dropping the guard removes its marker. Keep it alive until refs protect the objects.
#[derive(Debug)]
#[must_use = "hold the publication guard until refs protect the pack"]
pub struct Published {
    data_path: PathBuf,
    _keep: Handle<Closed>,
}

impl Published {
    /// The installed `.pack` path, whose sibling `.idx` can be opened with [`crate::Bundle::at()`].
    pub fn data_path(&self) -> &Path {
        &self.data_path
    }
}

pub(super) struct WriteOutcome {
    pub index: crate::index::write::Outcome,
    pub object_hash: gix_hash::Kind,
    pub pack_version: crate::data::Version,
    pub files: Option<StagedFiles>,
}

impl WriteOutcome {
    pub(super) fn into_prepared(self, alloc_limit_bytes: Option<usize>) -> Result<Option<Prepared>> {
        let Some(mut files) = self.files else {
            return Ok(None);
        };
        let pack_size = self.index.pack_size;
        files
            .data_file
            .with_mut(|file| {
                // The stream reader can read ahead. Only keep bytes through the indexed pack's trailer.
                let file = file.as_file();
                let length = file.metadata()?.len();
                if length > pack_size {
                    file.set_len(pack_size)?;
                } else if length < pack_size {
                    return Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Staged pack file is incomplete",
                    ));
                }
                Ok(())
            })
            .and_then(|res| res)
            .or_raise(|| message("Could not finalize staged pack length"))?;
        Ok(Some(Prepared {
            index: self.index,
            files,
            alloc_limit_bytes,
        }))
    }

    pub(super) fn publish_immediately(self) -> Result<Outcome> {
        let (data_path, index_path, keep_path) = match self.files {
            Some(files) => {
                let mut keep_path = None;
                let data_path = files.install(
                    |file, path| {
                        keep_path = create_keep_file(path)?;
                        persist_replacing(file, path)
                    },
                    persist_replacing,
                )?;
                let index_path = data_path.with_extension("idx");
                (Some(data_path), Some(index_path), keep_path)
            }
            None => (None, None, None),
        };
        Ok(Outcome {
            index: self.index,
            object_hash: self.object_hash,
            pack_version: self.pack_version,
            data_path,
            index_path,
            keep_path,
        })
    }
}

#[derive(Debug)]
pub(super) struct StagedFiles {
    pub data_path: PathBuf,
    pub data_file: Handle<Writable>,
    pub index_file: Handle<Writable>,
    pub shared_repository_permissions: gix_fs::SharedPermissions,
}

impl StagedFiles {
    fn install(
        mut self,
        persist_pack: impl FnOnce(Handle<Writable>, &Path) -> Result<()>,
        persist_index: impl FnOnce(Handle<Writable>, &Path) -> Result<()>,
    ) -> Result<PathBuf> {
        let index_path = self.data_path.with_extension("idx");
        let permissions = self.shared_repository_permissions;
        set_staged_permissions(&mut self.index_file, permissions)
            .or_raise(|| message("Could not set temporary pack index permissions"))?;

        install_file(self.data_file, &self.data_path, permissions, |mut file, path| {
            set_staged_permissions(&mut file, permissions)
                .or_raise(|| message("Could not set temporary pack file permissions"))?;
            persist_pack(file, path)
        })
        .or_raise(|| message!("Could not install pack file at {}", self.data_path.display()))?;

        install_file(self.index_file, &index_path, permissions, persist_index)
            .inspect_err(|_err| {
                gix_trace::warn!(
                    "pack file at \"{}\" is retained despite failing to move the index file into place",
                    self.data_path.display()
                );
            })
            .or_raise(|| message!("Could not install pack index at {}", index_path.display()))?;
        Ok(self.data_path)
    }
}

fn set_staged_permissions(file: &mut Handle<Writable>, permissions: gix_fs::SharedPermissions) -> Result<()> {
    file.with_mut(|file| gix_fs::set_shared_repository_permissions(file.path(), permissions))
        .and_then(|res| res)
        .or_error()
}

fn install_file(
    file: Handle<Writable>,
    path: &Path,
    permissions: gix_fs::SharedPermissions,
    persist: impl FnOnce(Handle<Writable>, &Path) -> Result<()>,
) -> Result<()> {
    if path.is_file() {
        // Avoid replacing a file that might already be mapped, particularly on Windows.
        return gix_fs::set_shared_repository_permissions(path, permissions).or_error();
    }
    persist(file, path)
}

fn persist_replacing(file: Handle<Writable>, path: &Path) -> Result<()> {
    file.persist(path).or_error().map(|_| ())
}

fn persist_noclobber(file: Handle<Writable>, path: &Path) -> Result<()> {
    let file = file
        .take()
        .ok_or_raise(|| message("Could not access the staged pack file"))?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(()),
        // A direct bundle writer can publish while we hold the marker.
        Err(err) if err.error.kind() == io::ErrorKind::AlreadyExists && path.is_file() => Ok(()),
        Err(err) => Err(err.error.raise()),
    }
}

fn acquire_keep_file(path: &Path) -> io::Result<Handle<Closed>> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        gix_tempfile::mark_at_with_permissions(
            path,
            ContainingDirectory::Exists,
            AutoRemove::Tempfile,
            std::fs::Permissions::from_mode(0o600),
        )
    }
    #[cfg(not(unix))]
    gix_tempfile::mark_at(path, ContainingDirectory::Exists, AutoRemove::Tempfile)
}

fn create_keep_file(data_path: &Path) -> Result<Option<PathBuf>> {
    let keep_path = data_path.with_extension("keep");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Git's keep markers are private, independently of core.sharedRepository.
        options.mode(0o600);
    }
    match options.open(&keep_path) {
        Ok(_) => Ok(Some(keep_path)),
        // An existing marker belongs to another operation and must not be removed by our caller.
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => Ok(None),
        Err(err) => Err(err.and_raise(message("Could not create pack keep file"))),
    }
}
