use crate::{bstr::BString, clone::PrepareFetch, Repository};

/// Builder
impl PrepareFetch {
    /// Use `f` to apply arbitrary changes to the remote that is about to be used to fetch a pack.
    ///
    /// The passed in `remote` will be un-named and pre-configured to be a default remote as we know it from git-clone.
    /// It is not yet present in the configuration of the repository,
    /// but each change it will eventually be written to the configuration prior to performing a the fetch operation,
    /// _all changes done in `f()` will be persisted_.
    ///
    /// It can also be used to configure additional options, like those for fetching tags. Note that
    /// [`with_fetch_tags()`](crate::Remote::with_fetch_tags()) should be called here to configure the clone as desired.
    /// Otherwise, a clone is configured to be complete and fetches all tags, not only those reachable from all branches.
    pub fn configure_remote(
        mut self,
        f: impl FnMut(crate::Remote<'_>) -> Result<crate::Remote<'_>, Box<dyn std::error::Error + Send + Sync>> + 'static,
    ) -> Self {
        self.configure_remote = Some(Box::new(f));
        self
    }

    /// Set the remote's name to the given value after it was configured using the function provided via
    /// [`configure_remote()`](Self::configure_remote()).
    ///
    /// If not set here, it defaults to `origin` or the value of `clone.defaultRemoteName`.
    pub fn with_remote_name(mut self, name: impl Into<BString>) -> Result<Self, crate::remote::name::Error> {
        self.remote_name = Some(crate::remote::name::validated(name)?);
        Ok(self)
    }

    /// Make this clone a shallow one with the respective choice of shallow-ness.
    pub fn with_shallow(mut self, shallow: crate::remote::fetch::Shallow) -> Self {
        self.shallow = shallow;
        self
    }

    /// Apply the given configuration `values` right before readying the actual fetch from the remote.
    /// The configuration is marked with [source API](gix_config::Source::Api), and will not be written back, it's
    /// retained only in memory.
    pub fn with_in_memory_config_overrides(mut self, values: impl IntoIterator<Item = impl Into<BString>>) -> Self {
        self.config_overrides = values.into_iter().map(Into::into).collect();
        self
    }

    /// Set the `name` of the reference to check out, instead of the remote `HEAD`.
    /// If `None`, the `HEAD` will be used, which is the default.
    ///
    /// Note that `name` should be a partial name like `main` or `feat/one`, but can be a full ref name.
    /// If a branch on the remote matches, it will automatically be retrieved even without a refspec.
    pub fn with_ref_name<'a, Name, E>(mut self, name: Option<Name>) -> Result<Self, E>
    where
        Name: TryInto<&'a gix_ref::PartialNameRef, Error = E>,
    {
        self.ref_name = name.map(TryInto::try_into).transpose()?.map(ToOwned::to_owned);
        Ok(self)
    }
}

/// Consumption
impl PrepareFetch {
    /// Persist the contained repository as is even if an error may have occurred when fetching from the remote.
    pub fn persist(mut self) -> Repository {
        self.repo.take().expect("present and consumed once")
    }
}

impl Drop for PrepareFetch {
    fn drop(&mut self) {
        if let Some(repo) = self.repo.take() {
            std::fs::remove_dir_all(repo.workdir().unwrap_or_else(|| repo.path())).ok();
        }
    }
}

impl From<PrepareFetch> for Repository {
    fn from(prep: PrepareFetch) -> Self {
        prep.persist()
    }
}
