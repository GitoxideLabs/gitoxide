use gix_error::bail;

use crate::{
    Result,
    bstr::{BString, ByteSlice},
    config,
    config::tree::{Key, Notes, Section, keys},
};

impl Notes {
    /// The `notes.displayRef` key, overridden by `GIT_NOTES_DISPLAY_REF`.
    pub const DISPLAY_REF: DisplayRef =
        DisplayRef::new_with_validate("displayRef", &config::Tree::NOTES, validate::DisplayRef)
            .with_environment_override("GIT_NOTES_DISPLAY_REF");
}

impl Section for Notes {
    fn name(&self) -> &str {
        "notes"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DISPLAY_REF]
    }
}

/// The `notes.displayRef` key.
pub type DisplayRef = keys::Any<validate::DisplayRef>;

impl DisplayRef {
    /// Parse and validate `value` as literal notes references or glob patterns.
    ///
    /// An item containing `*`, `?`, or `[` is a glob; all other items must be fully qualified references.
    pub fn try_into_display_refs(&'static self, value: impl gix_utils::AsBStr) -> Result<Vec<BString>> {
        let value = value.as_bstr();
        let refs = value
            .split(|byte| *byte == b':')
            .filter(|value| !value.is_empty())
            .map(BString::from)
            .collect::<Vec<_>>();
        let is_valid = refs.iter().all(|reference| {
            let Some(pattern) = gix_glob::Pattern::from_bytes_without_negation(reference) else {
                return false;
            };
            pattern.has_wildcard() || <&gix_ref::FullNameRef>::try_from(reference.as_bstr()).is_ok()
        });
        if !is_valid {
            bail!(config::key::error_with_value(
                self,
                "Invalid configuration value",
                value,
            ));
        }
        Ok(refs)
    }
}

/// Validation for `notes` configuration keys.
pub mod validate {

    use crate::{Result, bstr::BStr, config::tree::keys::Validate};

    /// Validate values for [`super::DisplayRef`].
    #[derive(Clone, Copy)]
    pub struct DisplayRef;

    impl Validate for DisplayRef {
        fn validate(&self, value: &BStr) -> Result {
            super::Notes::DISPLAY_REF.try_into_display_refs(value)?;
            Ok(())
        }
    }
}
