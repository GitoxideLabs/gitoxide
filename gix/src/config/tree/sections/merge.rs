use crate::{
    config,
    config::{
        Tree,
        tree::{Key, Merge, Section, SubSectionRequirement, keys},
    },
};

impl Merge {
    /// The `merge.renameLimit` key.
    pub const RENAME_LIMIT: keys::UnsignedInteger = keys::UnsignedInteger::new_unsigned_integer(
        "renameLimit",
        &config::Tree::MERGE,
    )
    .with_note(
        "The limit is actually squared, so 1000 stands for up to 1 million diffs if fuzzy rename tracking is enabled",
    );
    /// The `merge.renames` key.
    #[cfg(feature = "merge")]
    pub const RENAMES: super::diff::Renames = super::diff::Renames::new_renames("renames", &config::Tree::MERGE);
    /// The `merge.renormalize` key
    pub const RENORMALIZE: keys::Boolean = keys::Boolean::new_boolean("renormalize", &Tree::MERGE);
    /// The `merge.default` key
    pub const DEFAULT: keys::String = keys::String::new_string("default", &Tree::MERGE);
    /// The `merge.<driver>.name` key.
    pub const DRIVER_NAME: keys::String = keys::String::new_string("name", &config::Tree::MERGE)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `merge.<driver>.driver` key.
    pub const DRIVER_COMMAND: keys::Program = keys::Program::new_program("driver", &config::Tree::MERGE)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `merge.<driver>.recursive` key.
    pub const DRIVER_RECURSIVE: keys::String = keys::String::new_string("recursive", &config::Tree::MERGE)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `merge.conflictStyle` key.
    #[cfg(feature = "merge")]
    pub const CONFLICT_STYLE: ConflictStyle =
        ConflictStyle::new_with_validate("conflictStyle", &config::Tree::MERGE, validate::ConflictStyle);
}

impl Section for Merge {
    fn name(&self) -> &str {
        "merge"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::RENAME_LIMIT,
            #[cfg(feature = "merge")]
            &Self::RENAMES,
            &Self::RENORMALIZE,
            &Self::DEFAULT,
            &Self::DRIVER_NAME,
            &Self::DRIVER_COMMAND,
            &Self::DRIVER_RECURSIVE,
            #[cfg(feature = "merge")]
            &Self::CONFLICT_STYLE,
        ]
    }
}

/// The `merge.conflictStyle` key.
#[cfg(feature = "merge")]
pub type ConflictStyle = keys::Any<validate::ConflictStyle>;

#[cfg(feature = "merge")]
mod conflict_style {
    use std::borrow::Cow;

    use gix_merge::blob::builtin_driver::text;

    use crate::{bstr::BStr, config, config::tree::sections::merge::ConflictStyle};

    impl ConflictStyle {
        /// Derive the diff algorithm identified by `name`, case-insensitively.
        pub fn try_into_conflict_style(
            &'static self,
            name: Cow<'_, BStr>,
        ) -> Result<text::ConflictStyle, config::key::GenericErrorWithValue> {
            let style = if name.as_ref() == "merge" {
                text::ConflictStyle::Merge
            } else if name.as_ref() == "diff3" {
                text::ConflictStyle::Diff3
            } else if name.as_ref() == "zdiff3" {
                text::ConflictStyle::ZealousDiff3
            } else {
                return Err(config::key::GenericErrorWithValue::from_value(self, name.into_owned()));
            };
            Ok(style)
        }
    }
}

#[cfg(feature = "merge")]
mod validate {
    use crate::{
        bstr::BStr,
        config::tree::{Merge, keys},
    };

    #[derive(Clone, Copy)]
    pub struct ConflictStyle;
    impl keys::Validate for ConflictStyle {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Merge::CONFLICT_STYLE.try_into_conflict_style(value.into())?;
            Ok(())
        }
    }
}
