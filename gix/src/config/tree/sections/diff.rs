use crate::{
    config,
    config::tree::{Diff, Key, Section, SubSectionRequirement, keys},
};

impl Diff {
    /// The `diff.algorithm` key.
    pub const ALGORITHM: Algorithm =
        Algorithm::new_with_validate("algorithm", &config::Tree::DIFF, validate::Algorithm)
            .with_default(b"myers")
            .with_deviation(
                "'patience' diff is not implemented and can default to 'histogram' if lenient config is used",
            );
    /// The `diff.renameLimit` key.
    pub const RENAME_LIMIT: keys::UnsignedInteger = keys::UnsignedInteger::new_unsigned_integer(
        "renameLimit",
        &config::Tree::DIFF,
    )
    .with_note(
        "The limit is actually squared, so 1000 stands for up to 1 million diffs if fuzzy rename tracking is enabled",
    );

    /// The `diff.ignoreSubmodules` key.
    pub const IGNORE_SUBMODULES: Ignore =
        Ignore::new_with_validate("ignoreSubmodules", &config::Tree::DIFF, validate::Ignore)
            .with_note("This setting affects only the submodule status, and thus the repository status in general.");

    /// The `diff.renames` key.
    pub const RENAMES: Renames = Renames::new_renames("renames", &config::Tree::DIFF);

    /// The `diff.<driver>.command` key.
    pub const DRIVER_COMMAND: keys::Program = keys::Program::new_program("command", &config::Tree::DIFF)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `diff.<driver>.textconv` key.
    pub const DRIVER_TEXTCONV: keys::Program = keys::Program::new_program("textconv", &config::Tree::DIFF)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `diff.<driver>.algorithm` key.
    pub const DRIVER_ALGORITHM: Algorithm =
        Algorithm::new_with_validate("algorithm", &config::Tree::DIFF, validate::Algorithm)
            .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));
    /// The `diff.<driver>.binary` key.
    pub const DRIVER_BINARY: Binary = Binary::new_with_validate("binary", &config::Tree::DIFF, validate::Binary)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("driver")));

    /// The `diff.external` key.
    pub const EXTERNAL: keys::Program =
        keys::Program::new_program("external", &config::Tree::DIFF).with_environment_override("GIT_EXTERNAL_DIFF");
}

impl Section for Diff {
    fn name(&self) -> &str {
        "diff"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::ALGORITHM,
            &Self::IGNORE_SUBMODULES,
            &Self::RENAME_LIMIT,
            &Self::RENAMES,
            &Self::DRIVER_COMMAND,
            &Self::DRIVER_TEXTCONV,
            &Self::DRIVER_ALGORITHM,
            &Self::DRIVER_BINARY,
            &Self::EXTERNAL,
        ]
    }
}

/// The `diff.algorithm` key.
pub type Algorithm = keys::Any<validate::Algorithm>;

/// The `diff.ignoreSubmodules` key.
pub type Ignore = keys::Any<validate::Ignore>;

/// The `diff.renames` key.
pub type Renames = keys::Any<validate::Renames>;

/// The `diff.<driver>.binary` key.
pub type Binary = keys::Any<validate::Binary>;

mod algorithm {
    use crate::{
        Error, Result,
        bstr::ByteSlice,
        config,
        config::{
            diff::algorithm,
            tree::sections::diff::{Algorithm, Ignore},
        },
    };

    impl Ignore {
        /// See if `value` is an actual ignore
        pub fn try_into_ignore(&'static self, value: impl gix_utils::AsBStr) -> Result<gix_submodule::config::Ignore> {
            let value = value.as_bstr();
            gix_submodule::config::Ignore::try_from(value.as_bstr()).map_err(|()| {
                Error::from_error(config::key::error_with_value(
                    self,
                    "Invalid configuration value",
                    value,
                ))
            })
        }
    }

    impl Algorithm {
        /// Derive the diff algorithm identified by `name`, case-insensitively.
        pub fn try_into_algorithm(
            &self,
            name: impl gix_utils::AsBStr,
        ) -> std::result::Result<gix_diff::blob::Algorithm, algorithm::Error> {
            let name = name.as_bstr();
            let algo = if name.eq_ignore_ascii_case(b"myers") || name.eq_ignore_ascii_case(b"default") {
                gix_diff::blob::Algorithm::Myers
            } else if name.eq_ignore_ascii_case(b"minimal") {
                gix_diff::blob::Algorithm::MyersMinimal
            } else if name.eq_ignore_ascii_case(b"histogram") {
                gix_diff::blob::Algorithm::Histogram
            } else if name.eq_ignore_ascii_case(b"patience") {
                return Err(config::diff::algorithm::Error::Unimplemented { name: name.into() });
            } else {
                return Err(algorithm::Error::Unknown { name: name.into() });
            };
            Ok(algo)
        }
    }
}

mod binary {
    use gix_error::ResultExt;

    use crate::{Result, bstr::ByteSlice, config::tree::diff::Binary};

    impl Binary {
        /// Convert `value` into a tri-state boolean that can take the special value `auto`, resulting in `None`, or is a boolean.
        /// If `None` is given, it's treated as implicit boolean `true`, as this method is made to be used
        /// with [`gix_config::file::section::BodyRef::value_implicit()`].
        pub fn try_into_binary(&'static self, value: Option<impl gix_utils::AsBStr>) -> Result<Option<bool>> {
            Ok(match value {
                None => Some(true),
                Some(value) => {
                    let value = value.as_bstr();
                    if value.as_bstr() == "auto" {
                        None
                    } else {
                        Some(
                            gix_config::Boolean::try_from(value.as_bstr())
                                .map(|b| b.0)
                                .or_raise(|| {
                                    crate::config::key::error_with_value(self, "Invalid configuration value", value)
                                })?,
                        )
                    }
                }
            })
        }
    }
}

mod renames {
    use crate::{
        ExnMessageResult, Result,
        bstr::ByteSlice,
        config::{
            key,
            tree::{Section, keys, sections::diff::Renames},
        },
        diff::rename::Tracking,
    };

    impl Renames {
        /// Create a new instance.
        pub const fn new_renames(name: &'static str, section: &'static dyn Section) -> Self {
            keys::Any::new_with_validate(name, section, super::validate::Renames)
        }
        /// Try to convert the configuration into a valid rename tracking variant. Use `value` and if it's an error, interpret
        /// the boolean as string
        pub fn try_into_renames(&'static self, value: ExnMessageResult<Option<bool>>) -> Result<Option<Tracking>> {
            Ok(match value {
                Ok(Some(true)) => Some(Tracking::Renames),
                Ok(Some(false)) => Some(Tracking::Disabled),
                Ok(None) => None,
                Err(err) => {
                    let Some(gix_error::MetadataValue::Bytes(value)) = err.error().values.get("input") else {
                        return Err(err.into());
                    };
                    match value.as_bytes() {
                        b"copy" | b"copies" => Some(Tracking::RenamesAndCopies),
                        _ => {
                            let context = key::error_with_value(self, "Invalid configuration value", value.as_bstr());
                            return Err(err.raise(context).into());
                        }
                    }
                }
            })
        }
    }
}

pub(super) mod validate {
    use gix_error::{ErrorExt, ResultExt, message};

    use crate::{
        ExnResult,
        bstr::BStr,
        config::tree::{Diff, keys},
    };

    #[derive(Copy, Clone)]
    pub struct Ignore;
    impl keys::Validate for Ignore {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_submodule::config::Ignore::try_from(value)
                .map_err(|()| message!("Value '{value}' is not a valid submodule 'ignore' value").raise_erased())?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Algorithm;
    impl keys::Validate for Algorithm {
        fn validate(&self, value: &BStr) -> ExnResult {
            Diff::ALGORITHM.try_into_algorithm(value).or_erased()?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Renames;
    impl keys::Validate for Renames {
        fn validate(&self, value: &BStr) -> ExnResult {
            let boolean = gix_config::Boolean::try_from(value).map(|b| Some(b.0));
            Diff::RENAMES.try_into_renames(boolean).or_erased()?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Binary;
    impl keys::Validate for Binary {
        fn validate(&self, value: &BStr) -> ExnResult {
            Diff::DRIVER_BINARY.try_into_binary(Some(value)).or_erased()?;
            Ok(())
        }
    }
}
