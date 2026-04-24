use std::rc::Rc;

use crate::{submodule, Repository};

impl Repository {
    /// Open the `.gitmodules` file as present in the worktree, or return `None` if no such file is available.
    /// Symlinked worktree `.gitmodules` files are silently ignored so content outside the repository
    /// cannot become active submodule configuration by being linked into the worktree.
    /// Note that git configuration is also contributing to the result based on the current snapshot.
    ///
    /// Note that his method will not look in other places, like the index or the `HEAD` tree.
    // TODO(submodule): make it use an updated snapshot instead once we have `config()`.
    pub fn open_modules_file(&self) -> Result<Option<gix_submodule::File>, submodule::open_modules_file::Error> {
        let path = match self.modules_path() {
            Some(path) => path,
            None => return Ok(None),
        };
        // TODO(ErrorKind): we want to use `ErrorKind::FilesystemLoop`, which otherwise happens
        //                  when doing `gix_fs::options_no_follow()`,
        //                  so we could catch NotFound along with it and save the extra check.
        let metadata = match std::fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.into()),
        };
        if metadata.file_type().is_symlink() {
            return Ok(None);
        }
        let buf = std::fs::read(&path)?;
        Ok(Some(gix_submodule::File::from_bytes(
            &buf,
            path,
            &self.config.resolved,
        )?))
    }

    /// Return a shared [`.gitmodules` file](submodule::File) which is updated automatically if the in-memory snapshot
    /// has become stale as the underlying file on disk has changed. The snapshot based on the file on disk is shared across all
    /// clones of this repository.
    ///
    /// If a file on disk isn't present, we will try to load it from the index, and finally from the current tree.
    /// In the latter two cases, the result will not be cached in this repository instance as we can't detect freshness anymore,
    /// so time this method is called a new [modules file](submodule::ModulesSnapshot) will be created.
    ///
    /// Note that git configuration is also contributing to the result based on the current snapshot.
    ///
    // TODO(submodule): make it use an updated snapshot instead once we have `config()`.
    pub fn modules(&self) -> Result<Option<submodule::ModulesSnapshot>, submodule::modules::Error> {
        match self.modules.recent_snapshot(
            || {
                self.modules_path()
                    .and_then(|path| path.metadata().and_then(|m| m.modified()).ok())
            },
            || self.open_modules_file(),
        )? {
            Some(m) => Ok(Some(m)),
            None => {
                let id = match self.try_index()?.and_then(|index| {
                    index
                        .entry_by_path(submodule::MODULES_FILE.into())
                        .map(|entry| entry.id)
                }) {
                    Some(id) => id,
                    None => match self
                        .head()?
                        .try_peel_to_id()?
                        .map(|id| -> Result<Option<_>, submodule::modules::Error> {
                            Ok(id
                                .object()?
                                .peel_to_commit()?
                                .tree()?
                                .find_entry(submodule::MODULES_FILE)
                                .map(|entry| entry.inner.oid.to_owned()))
                        })
                        .transpose()?
                        .flatten()
                    {
                        Some(id) => id,
                        None => return Ok(None),
                    },
                };
                Ok(Some(gix_features::threading::OwnShared::new(
                    gix_submodule::File::from_bytes(&self.find_object(id)?.data, None, &self.config.resolved)
                        .map_err(submodule::open_modules_file::Error::from)?
                        .into(),
                )))
            }
        }
    }

    /// Return the list of available submodules, or `None` if there is no submodule configuration.
    #[doc(alias = "git2")]
    pub fn submodules(&self) -> Result<Option<impl Iterator<Item = crate::Submodule<'_>>>, submodule::modules::Error> {
        let modules = match self.modules()? {
            None => return Ok(None),
            Some(m) => m,
        };
        let shared_state = Rc::new(submodule::SharedState::new(self, modules));
        Ok(Some(
            shared_state
                .modules
                .names()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
                .into_iter()
                .map(move |name| crate::Submodule {
                    state: shared_state.clone(),
                    name,
                }),
        ))
    }
}
