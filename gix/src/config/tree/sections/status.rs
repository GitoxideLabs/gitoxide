use crate::{
    config,
    config::tree::{Key, Section, keys, sections::Status},
};

impl Status {
    /// The `status.showUntrackedFiles` key. Boolean values select `normal` when true and `no` when false.
    pub const SHOW_UNTRACKED_FILES: ShowUntrackedFiles = ShowUntrackedFiles::new_with_validate(
        "showUntrackedFiles",
        &config::Tree::STATUS,
        validate::ShowUntrackedFiles,
    );
    /// The `status.renameLimit` key.
    pub const RENAME_LIMIT: keys::UnsignedInteger = keys::UnsignedInteger::new_unsigned_integer(
        "renameLimit",
        &config::Tree::STATUS,
    )
    .with_note(
        "The limit is actually squared, so 1000 stands for up to 1 million diffs if fuzzy rename tracking is enabled",
    );
    /// The `status.renames` key.
    pub const RENAMES: super::diff::Renames = super::diff::Renames::new_renames("renames", &config::Tree::STATUS);
}

/// The `status.showUntrackedFiles` key.
pub type ShowUntrackedFiles = keys::Any<validate::ShowUntrackedFiles>;

mod show_untracked_files {
    use gix_error::bail;

    use crate::{Result, bstr::ByteSlice, config, config::tree::status::ShowUntrackedFiles, status};

    impl ShowUntrackedFiles {
        pub fn try_into_show_untracked_files(
            &'static self,
            value: impl gix_utils::AsBStr,
        ) -> Result<status::UntrackedFiles> {
            let value = value.as_bstr();
            if let Ok(boolean) = gix_config::Boolean::try_from(value) {
                return Ok(if boolean.0 {
                    status::UntrackedFiles::Collapsed
                } else {
                    status::UntrackedFiles::None
                });
            }
            Ok(match value.as_bstr().as_bytes() {
                b"normal" => status::UntrackedFiles::Collapsed,
                b"all" => status::UntrackedFiles::Files,
                _ => {
                    bail!(config::key::error_with_value(
                        self,
                        "Invalid configuration value",
                        value,
                    ));
                }
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
    use crate::{Result, bstr::BStr, config::tree::keys};

    #[derive(Clone, Copy)]
    pub struct ShowUntrackedFiles;
    impl keys::Validate for ShowUntrackedFiles {
        fn validate(&self, value: &BStr) -> Result {
            super::Status::SHOW_UNTRACKED_FILES.try_into_show_untracked_files(value)?;
            Ok(())
        }
    }
}
