use crate::{
    config,
    config::tree::{Diff, Key, Section, SubSectionRequirement, keys},
};

impl Diff {
    /// The `diff.algorithm` key.
    pub const ALGORITHM: Algorithm = Algorithm::new_with_validate("algorithm", &config::Tree::DIFF, validate::Algorithm)
                                        .with_deviation("'patience' diff is not implemented and can default to 'histogram' if lenient config is used, and defaults to histogram if unset for fastest and best results");
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
    use std::borrow::Cow;

    use crate::{
        bstr::BStr,
        config,
        config::{
            diff::algorithm,
            key,
            tree::sections::diff::{Algorithm, Ignore},
        },
    };

    impl Ignore {
        /// See if `value` is an actual ignore
        pub fn try_into_ignore(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<gix_submodule::config::Ignore, key::GenericErrorWithValue> {
            gix_submodule::config::Ignore::try_from(value.as_ref())
                .map_err(|()| key::GenericErrorWithValue::from_value(self, value.into_owned()))
        }
    }

    impl Algorithm {
        /// Derive the diff algorithm identified by `name`, case-insensitively.
        pub fn try_into_algorithm(&self, name: Cow<'_, BStr>) -> Result<gix_diff::blob::Algorithm, algorithm::Error> {
            let algo = if name.eq_ignore_ascii_case(b"myers") || name.eq_ignore_ascii_case(b"default") {
                gix_diff::blob::Algorithm::Myers
            } else if name.eq_ignore_ascii_case(b"minimal") {
                gix_diff::blob::Algorithm::MyersMinimal
            } else if name.eq_ignore_ascii_case(b"histogram") {
                gix_diff::blob::Algorithm::Histogram
            } else if name.eq_ignore_ascii_case(b"patience") {
                return Err(config::diff::algorithm::Error::Unimplemented {
                    name: name.into_owned(),
                });
            } else {
                return Err(algorithm::Error::Unknown {
                    name: name.into_owned(),
                });
            };
            Ok(algo)
        }
    }
}

mod binary {
    use crate::config::tree::diff::Binary;

    impl Binary {
        /// Convert `value` into a tri-state boolean that can take the special value `auto`, resulting in `None`, or is a boolean.
        /// If `None` is given, it's treated as implicit boolean `true`, as this method is made to be used
        /// with [`gix_config::file::section::Body::value_implicit()`].
        pub fn try_into_binary(
            &'static self,
            value: Option<std::borrow::Cow<'_, crate::bstr::BStr>>,
        ) -> Result<Option<bool>, crate::config::key::GenericErrorWithValue> {
            Ok(match value {
                None => Some(true),
                Some(value) => {
                    if value.as_ref() == "auto" {
                        None
                    } else {
                        Some(
                            gix_config::Boolean::try_from(value.as_ref())
                                .map(|b| b.0)
                                .map_err(|err| {
                                    crate::config::key::GenericErrorWithValue::from_value(self, value.into_owned())
                                        .with_source(err)
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
        bstr::ByteSlice,
        config::{
            key::GenericError,
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
        pub fn try_into_renames(
            &'static self,
            value: Result<bool, gix_config::value::Error>,
        ) -> Result<Tracking, GenericError> {
            Ok(match value {
                Ok(true) => Tracking::Renames,
                Ok(false) => Tracking::Disabled,
                Err(err) => {
                    let value = &err.input;
                    match value.as_bytes() {
                        b"copy" | b"copies" => Tracking::RenamesAndCopies,
                        _ => return Err(GenericError::from_value(self, value.clone()).with_source(err)),
                    }
                }
            })
        }
    }
}

pub(super) mod validate {
    use crate::{
        bstr::BStr,
        config::tree::{Diff, keys},
    };

    #[derive(Copy, Clone)]
    pub struct Ignore;
    impl keys::Validate for Ignore {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            gix_submodule::config::Ignore::try_from(value)
                .map_err(|()| format!("Value '{value}' is not a valid submodule 'ignore' value"))?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Algorithm;
    impl keys::Validate for Algorithm {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Diff::ALGORITHM.try_into_algorithm(value.into())?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Renames;
    impl keys::Validate for Renames {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let boolean = gix_config::Boolean::try_from(value).map(|b| b.0);
            Diff::RENAMES.try_into_renames(boolean)?;
            Ok(())
        }
    }

    #[derive(Copy, Clone)]
    pub struct Binary;
    impl keys::Validate for Binary {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Diff::DRIVER_BINARY.try_into_binary(Some(value.into()))?;
            Ok(())
        }
    }
}
