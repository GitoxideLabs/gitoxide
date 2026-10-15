use std::{io::Read, rc::Rc};

use crate::error::{ErrorExt, ResultExt, message};
use crate::{Repository, Result, submodule};
use gix_fs::FileOrSymlink;

impl Repository {
    /// Open the `.gitmodules` file as present in the worktree, or return `None` if no such file is available.
    /// Symlinked worktree `.gitmodules` files are silently ignored so content outside the repository
    /// cannot become active submodule configuration by being linked into the worktree.
    /// Note that git configuration is also contributing to the result based on the current snapshot.
    /// Only sections accepted by the repository's configuration filter contribute overrides.
    ///
    /// Note that his method will not look in other places, like the index or the `HEAD` tree.
    #[gix_macros::unstable(reason = "Returns a `gix-submodule` configuration file.")]
    pub fn open_modules_file(&self) -> Result<Option<gix_submodule::File>> {
        let path = match self.modules_path() {
            Some(path) => path,
            None => return Ok(None),
        };
        let mut file = match gix_fs::open_read_only_no_follow(&path) {
            Ok(FileOrSymlink::File(file)) => file,
            Ok(FileOrSymlink::Symlink) => return Ok(None),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(err.and_raise(message("Could not open '.gitmodules' file"))),
        };
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)
            .or_raise(|| message("Could not read '.gitmodules' file"))?;
        Ok(Some(self.modules_from_bytes(&buf, Some(path))?))
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
    #[gix_macros::unstable(reason = "Returns a `gix-fs` shared snapshot of a `gix-submodule` configuration file.")]
    pub fn modules(&self) -> Result<Option<submodule::ModulesSnapshot>> {
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
                        .map(|id| -> Result<Option<_>> {
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
                Ok(Some(gix_parallel::OwnShared::new(
                    self.modules_from_bytes(&self.find_object(id)?.data, None)?.into(),
                )))
            }
        }
    }

    fn modules_from_bytes(&self, bytes: &[u8], path: Option<std::path::PathBuf>) -> Result<gix_submodule::File> {
        let mut overrides = gix_config::File::new(self.config.resolved.meta_owned());
        for section in self
            .config
            .resolved
            .sections_by_name_and_filter("submodule", self.filter_config_section())
            .into_iter()
            .flatten()
        {
            overrides.push_section(section.to_owned())?;
        }
        gix_submodule::File::from_bytes(bytes, path, &overrides)
    }

    /// Return the list of available submodules, or `None` if there is no submodule configuration.
    #[doc(alias = "git2")]
    #[gix_macros::unstable(reason = "Returns handles exposing `gix-submodule` configuration and state types.")]
    pub fn submodules(&self) -> Result<Option<impl Iterator<Item = crate::Submodule<'_>>>> {
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
