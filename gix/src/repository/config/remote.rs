use crate::{
    bstr::{BStr, BString, ByteSlice},
    config::tree::{Remote, Section},
    remote,
};

/// Query configuration related to remotes.
impl crate::Repository {
    /// Returns a sorted list unique of symbolic names of remotes that
    /// we deem [trustworthy][crate::open::Options::filter_config_section()].
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("clone")?)?;
    /// let remote_names: Vec<_> = repo.remote_names().into_iter().map(|name| name.to_string()).collect();
    ///
    /// assert_eq!(remote_names, vec!["myself".to_owned(), "origin".to_owned()]);
    /// # Ok(()) }
    /// ```
    pub fn remote_names(&self) -> remote::Names {
        self.config
            .resolved
            .sections_by_name(Remote.name())
            .map(|it| {
                let filter = self.filter_config_section();
                it.filter(move |s| filter(s.meta()))
                    .filter_map(|section| section.header().subsection_name().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Obtain the branch-independent name for a remote for use in the given `direction`, or `None` if it could not be determined.
    ///
    /// For _fetching_, use the only configured remote, or default to `origin` if it exists.
    /// For _pushing_, use the `remote.pushDefault` trusted configuration key, or fall back to the rules for _fetching_.
    ///
    /// # Notes
    ///
    /// It's up to the caller to determine what to do if the current `head` is unborn or detached.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # use gix::bstr::ByteSlice;
    /// # mod doctest { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/doctest.rs")); }
    /// # let repo = doctest::open_repo(doctest::remote_repo_dir("clone")?)?;
    /// assert_eq!(
    ///     repo.remote_default_name(gix::remote::Direction::Fetch)
    ///         .expect("configured")
    ///         .as_bstr(),
    ///     "origin"
    /// );
    /// assert_eq!(
    ///     repo.remote_default_name(gix::remote::Direction::Push)
    ///         .expect("configured")
    ///         .as_bstr(),
    ///     "origin"
    /// );
    /// # Ok(()) }
    /// ```
    pub fn remote_default_name(&self, direction: remote::Direction) -> Option<BString> {
        let name = (direction == remote::Direction::Push)
            .then(|| {
                self.config
                    .resolved
                    .string_filter(Remote::PUSH_DEFAULT, &mut self.filter_config_section())
            })
            .flatten();
        name.or_else(|| {
            let names = self.remote_names();
            match names.len() {
                0 => None,
                1 => names.into_iter().next(),
                _more_than_one => {
                    let origin: &BStr = "origin".into();
                    names
                        .iter()
                        .any(|name| name.as_bstr() == origin)
                        .then(|| origin.to_owned())
                }
            }
        })
    }
}
