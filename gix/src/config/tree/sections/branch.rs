use crate::config::tree::{Branch, Key, Section, keys, traits::SubSectionRequirement};

const NAME_PARAMETER: Option<SubSectionRequirement> = Some(SubSectionRequirement::Parameter("name"));

impl Branch {
    /// The `branch.<name>.merge` key.
    pub const MERGE: Merge = Merge::new_with_validate(
        "merge",
        &crate::config::Tree::BRANCH,
        keys::validate::FullNameRef::new(),
    )
    .with_subsection_requirement(NAME_PARAMETER);
    /// The `branch.<name>.pushRemote` key.
    pub const PUSH_REMOTE: keys::RemoteName =
        keys::RemoteName::new_remote_name("pushRemote", &crate::config::Tree::BRANCH)
            .with_subsection_requirement(NAME_PARAMETER);
    /// The `branch.<name>.remote` key.
    pub const REMOTE: keys::RemoteName = keys::RemoteName::new_remote_name("remote", &crate::config::Tree::BRANCH)
        .with_subsection_requirement(NAME_PARAMETER);
}

impl Section for Branch {
    fn name(&self) -> &str {
        "branch"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::MERGE, &Self::PUSH_REMOTE, &Self::REMOTE]
    }
}

/// The `branch.<name>.merge` key.
pub type Merge = keys::Any<keys::validate::FullNameRef>;

mod merge {
    use gix_ref::FullName;

    use crate::config::tree::branch::Merge;

    impl Merge {
        /// Return the validated full ref name from `value` if it is valid.
        pub fn try_into_fullrefname(
            value: impl gix_utils::AsBStr,
        ) -> Result<FullName, gix_validate::reference::name::Error> {
            value.as_bstr().to_owned().try_into()
        }
    }
}
