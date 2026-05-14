use crate::config::tree::{Branch, Key, Section, keys, traits::SubSectionRequirement};

const NAME_PARAMETER: Option<SubSectionRequirement> = Some(SubSectionRequirement::Parameter("name"));

impl Branch {
    /// The `branch.<name>.merge` key.
    pub const MERGE: Merge = Merge::new_with_validate("merge", &crate::config::Tree::BRANCH, validate::FullNameRef)
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
pub type Merge = keys::Any<validate::FullNameRef>;

mod merge {
    use std::borrow::Cow;

    use gix_ref::FullNameRef;

    use crate::{bstr::BStr, config::tree::branch::Merge};

    impl Merge {
        /// Return the validated full ref name from `value` if it is valid.
        pub fn try_into_fullrefname(
            value: Cow<'_, BStr>,
        ) -> Result<Cow<'_, FullNameRef>, gix_validate::reference::name::Error> {
            match value {
                Cow::Borrowed(v) => v.try_into().map(Cow::Borrowed),
                Cow::Owned(v) => v.try_into().map(Cow::Owned),
            }
        }
    }
}

///
pub mod validate {
    use crate::{
        bstr::BStr,
        config::tree::{branch::Merge, keys},
    };

    pub struct FullNameRef;
    impl keys::Validate for FullNameRef {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Merge::try_into_fullrefname(value.into())?;
            Ok(())
        }
    }
}
