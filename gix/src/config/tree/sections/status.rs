use crate::{
    config,
    config::tree::{Key, Section, keys, sections::Status},
};

impl Status {
    /// The `status.showUntrackedFiles` key
    pub const SHOW_UNTRACKED_FILES: ShowUntrackedFiles = ShowUntrackedFiles::new_with_validate(
        "showUntrackedFiles",
        &config::Tree::STATUS,
        validate::ShowUntrackedFiles,
    );
    /// The `status.renameLimit` key.
    pub const RENAME_LIMIT: keys::UnsignedInteger = keys::UnsignedInteger::new_unsigned_integer(
        "renameLimit",
        &config::Tree::MERGE,
    )
    .with_note(
        "The limit is actually squared, so 1000 stands for up to 1 million diffs if fuzzy rename tracking is enabled",
    );
    /// The `status.renames` key.
    pub const RENAMES: super::diff::Renames = super::diff::Renames::new_renames("renames", &config::Tree::MERGE);
}

/// The `status.showUntrackedFiles` key.
pub type ShowUntrackedFiles = keys::Any<validate::ShowUntrackedFiles>;

mod show_untracked_files {
    use std::borrow::Cow;

    use crate::{bstr::BStr, config, config::tree::status::ShowUntrackedFiles, status};

    impl ShowUntrackedFiles {
        pub fn try_into_show_untracked_files(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<status::UntrackedFiles, config::key::GenericErrorWithValue> {
            use crate::bstr::ByteSlice;
            Ok(match value.as_ref().as_bytes() {
                b"no" => status::UntrackedFiles::None,
                b"normal" => status::UntrackedFiles::Collapsed,
                b"all" => status::UntrackedFiles::Files,
                _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
            })
        }
    }
}

impl Section for Status {
    fn name(&self) -> &str {
        "status"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::SHOW_UNTRACKED_FILES, &Self::RENAMES, &Self::RENAME_LIMIT]
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct ShowUntrackedFiles;
    impl keys::Validate for ShowUntrackedFiles {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Status::SHOW_UNTRACKED_FILES.try_into_show_untracked_files(value.into())?;
            Ok(())
        }
    }
}
