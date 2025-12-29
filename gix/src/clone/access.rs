use crate::{bstr::{BString, ByteSlice}, clone::PrepareFetch, Repository};

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

    /// Set the `name` of the reference or object hash to check out, instead of the remote `HEAD`.
    /// If `None`, the `HEAD` will be used, which is the default.
    ///
    /// Note that `name` should be a partial name like `main` or `feat/one`, a full ref name, or a hex object hash.
    /// If a branch on the remote matches, it will automatically be retrieved even without a refspec.
    /// If an object hash is provided, it will be fetched and checked out if available on the remote.
    pub fn with_ref_name(mut self, name: Option<impl Into<crate::bstr::BString>>) -> Result<Self, crate::clone::Error> {
        self.ref_name = name
            .map(|n| -> Result<crate::clone::CloneRef, crate::clone::Error> {
                let s = n.into();
                // Try to parse as an object hash first (40 hex chars for SHA-1, 64 for SHA-256)
                // This check helps differentiate between hex ref names and actual object hashes
                let sha1_hex_len = gix_hash::Kind::Sha1.len_in_hex();
                let sha256_hex_len = sha1_hex_len * 2; // SHA-256 is twice the length of SHA-1
                let is_valid_oid_length = s.len() == sha1_hex_len || s.len() == sha256_hex_len;
                if is_valid_oid_length {
                    if let Ok(oid) = gix_hash::ObjectId::from_hex(s.as_ref()) {
                        return Ok(crate::clone::CloneRef::ObjectHash(oid));
                    }
                }
                // Otherwise, try as a partial ref name
                let partial_ref = <&gix_ref::PartialNameRef>::try_from(s.as_bstr())
                    .map_err(crate::clone::Error::ReferenceName)?;
                Ok(crate::clone::CloneRef::RefName(partial_ref.to_owned()))
            })
            .transpose()?;
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
