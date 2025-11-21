use crate::{
    config,
    config::tree::{keys, Core, Key, Section},
};

impl Core {
    /// The `core.abbrev` key.
    pub const ABBREV: Abbrev = Abbrev::new_with_validate("abbrev", &config::Tree::CORE, validate::Abbrev);
    /// The `core.bare` key.
    pub const BARE: keys::Boolean = keys::Boolean::new_boolean("bare", &config::Tree::CORE);
    /// The `core.bigFileThreshold` key.
    pub const BIG_FILE_THRESHOLD: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("bigFileThreshold", &config::Tree::CORE);
    /// The `core.checkStat` key.
    pub const CHECK_STAT: CheckStat =
        CheckStat::new_with_validate("checkStat", &config::Tree::CORE, validate::CheckStat);
    /// The `core.deltaBaseCacheLimit` key.
    pub const DELTA_BASE_CACHE_LIMIT: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("deltaBaseCacheLimit", &config::Tree::CORE)
            .with_environment_override("GIX_PACK_CACHE_MEMORY")
            .with_note("if unset, we default to a small 64 slot fixed-size cache that holds at most 64 full delta base objects of any size. Set to 0 to deactivate it entirely");
    /// The `core.disambiguate` key.
    pub const DISAMBIGUATE: Disambiguate =
        Disambiguate::new_with_validate("disambiguate", &config::Tree::CORE, validate::Disambiguate);
    /// The `core.editor` key.
    pub const EDITOR: keys::Program = keys::Program::new_program("editor", &config::Tree::CORE);
    /// The `core.fileMode` key.
    pub const FILE_MODE: keys::Boolean = keys::Boolean::new_boolean("fileMode", &config::Tree::CORE);
    /// The `core.ignoreCase` key.
    pub const IGNORE_CASE: keys::Boolean = keys::Boolean::new_boolean("ignoreCase", &config::Tree::CORE);
    /// The `core.filesRefLockTimeout` key.
    pub const FILES_REF_LOCK_TIMEOUT: keys::LockTimeout =
        keys::LockTimeout::new_lock_timeout("filesRefLockTimeout", &config::Tree::CORE);
    /// The `core.packedRefsTimeout` key.
    pub const PACKED_REFS_TIMEOUT: keys::LockTimeout =
        keys::LockTimeout::new_lock_timeout("packedRefsTimeout", &config::Tree::CORE);
    /// The `core.multiPackIndex` key.
    pub const MULTIPACK_INDEX: keys::Boolean = keys::Boolean::new_boolean("multiPackIndex", &config::Tree::CORE);
    /// The `core.logAllRefUpdates` key.
    pub const LOG_ALL_REF_UPDATES: LogAllRefUpdates =
        LogAllRefUpdates::new_with_validate("logAllRefUpdates", &config::Tree::CORE, validate::LogAllRefUpdates);
    /// The `core.precomposeUnicode` key.
    ///
    /// Needs application to use [`env::args_os`][crate::env::args_os()] to conform all input paths before they are used.
    pub const PRECOMPOSE_UNICODE: keys::Boolean = keys::Boolean::new_boolean("precomposeUnicode", &config::Tree::CORE)
        .with_note("application needs to conform all program input by using gix::env::args_os()");
    /// The `core.protectHFS` key.
    pub const PROTECT_HFS: keys::Boolean = keys::Boolean::new_boolean("protectHFS", &config::Tree::CORE);
    /// The `core.protectNTFS` key.
    pub const PROTECT_NTFS: keys::Boolean = keys::Boolean::new_boolean("protectNTFS", &config::Tree::CORE);
    /// The `core.repositoryFormatVersion` key.
    pub const REPOSITORY_FORMAT_VERSION: keys::UnsignedInteger =
        keys::UnsignedInteger::new_unsigned_integer("repositoryFormatVersion", &config::Tree::CORE);
    /// The `core.symlinks` key.
    pub const SYMLINKS: keys::Boolean = keys::Boolean::new_boolean("symlinks", &config::Tree::CORE);
    /// The `core.trustCTime` key.
    pub const TRUST_C_TIME: keys::Boolean = keys::Boolean::new_boolean("trustCTime", &config::Tree::CORE);
    /// The `core.worktree` key.
    pub const WORKTREE: keys::Any = keys::Any::new("worktree", &config::Tree::CORE)
        .with_environment_override("GIT_WORK_TREE")
        .with_deviation("Command-line overrides also work, and they act lie an environment override. If set in the git configuration file, relative paths are relative to it.");
    /// The `core.askPass` key.
    pub const ASKPASS: keys::Executable = keys::Executable::new_executable("askPass", &config::Tree::CORE)
        .with_environment_override("GIT_ASKPASS")
        .with_note("fallback is 'SSH_ASKPASS'");
    /// The `core.excludesFile` key.
    pub const EXCLUDES_FILE: keys::Path = keys::Path::new_path("excludesFile", &config::Tree::CORE);
    /// The `core.attributesFile` key.
    pub const ATTRIBUTES_FILE: keys::Path =
        keys::Path::new_path("attributesFile", &config::Tree::CORE)
            .with_deviation("for checkout - it's already queried but needs building of attributes group, and of course support during checkout");
    /// The `core.sshCommand` key.
    pub const SSH_COMMAND: keys::Executable = keys::Executable::new_executable("sshCommand", &config::Tree::CORE)
        .with_environment_override("GIT_SSH_COMMAND");
    /// The `core.useReplaceRefs` key.
    pub const USE_REPLACE_REFS: keys::Boolean = keys::Boolean::new_boolean("useReplaceRefs", &config::Tree::CORE)
        .with_environment_override("GIT_NO_REPLACE_OBJECTS");
    /// The `core.commitGraph` key.
    pub const COMMIT_GRAPH: keys::Boolean = keys::Boolean::new_boolean("commitGraph", &config::Tree::CORE);
    /// The `core.safecrlf` key.
    #[cfg(feature = "attributes")]
    pub const SAFE_CRLF: SafeCrlf = SafeCrlf::new_with_validate("safecrlf", &config::Tree::CORE, validate::SafeCrlf);
    /// The `core.autocrlf` key.
    #[cfg(feature = "attributes")]
    pub const AUTO_CRLF: AutoCrlf = AutoCrlf::new_with_validate("autocrlf", &config::Tree::CORE, validate::AutoCrlf);
    /// The `core.eol` key.
    #[cfg(feature = "attributes")]
    pub const EOL: Eol = Eol::new_with_validate("eol", &config::Tree::CORE, validate::Eol);
    /// The `core.checkRoundTripEncoding` key.
    #[cfg(feature = "attributes")]
    pub const CHECK_ROUND_TRIP_ENCODING: CheckRoundTripEncoding = CheckRoundTripEncoding::new_with_validate(
        "checkRoundTripEncoding",
        &config::Tree::CORE,
        validate::CheckRoundTripEncoding,
    );
}

impl Section for Core {
    fn name(&self) -> &str {
        "core"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::ABBREV,
            &Self::BARE,
            &Self::BIG_FILE_THRESHOLD,
            &Self::CHECK_STAT,
            &Self::DELTA_BASE_CACHE_LIMIT,
            &Self::DISAMBIGUATE,
            &Self::EDITOR,
            &Self::FILE_MODE,
            &Self::IGNORE_CASE,
            &Self::FILES_REF_LOCK_TIMEOUT,
            &Self::PACKED_REFS_TIMEOUT,
            &Self::MULTIPACK_INDEX,
            &Self::LOG_ALL_REF_UPDATES,
            &Self::PRECOMPOSE_UNICODE,
            &Self::REPOSITORY_FORMAT_VERSION,
            &Self::SYMLINKS,
            &Self::TRUST_C_TIME,
            &Self::WORKTREE,
            &Self::PROTECT_HFS,
            &Self::PROTECT_NTFS,
            &Self::ASKPASS,
            &Self::EXCLUDES_FILE,
            &Self::ATTRIBUTES_FILE,
            &Self::SSH_COMMAND,
            &Self::USE_REPLACE_REFS,
            &Self::COMMIT_GRAPH,
            #[cfg(feature = "attributes")]
            &Self::SAFE_CRLF,
            #[cfg(feature = "attributes")]
            &Self::AUTO_CRLF,
            #[cfg(feature = "attributes")]
            &Self::EOL,
            #[cfg(feature = "attributes")]
            &Self::CHECK_ROUND_TRIP_ENCODING,
        ]
    }
}

/// The `core.checkStat` key.
pub type CheckStat = keys::Any<validate::CheckStat>;

/// The `core.abbrev` key.
pub type Abbrev = keys::Any<validate::Abbrev>;

/// The `core.logAllRefUpdates` key.
pub type LogAllRefUpdates = keys::Any<validate::LogAllRefUpdates>;

/// The `core.disambiguate` key.
pub type Disambiguate = keys::Any<validate::Disambiguate>;

#[cfg(feature = "attributes")]
mod filter {
    use super::validate;
    use crate::config::tree::keys;

    /// The `core.safecrlf` key.
    pub type SafeCrlf = keys::Any<validate::SafeCrlf>;

    /// The `core.autocrlf` key.
    pub type AutoCrlf = keys::Any<validate::AutoCrlf>;

    /// The `core.eol` key.
    pub type Eol = keys::Any<validate::Eol>;

    /// The `core.checkRoundTripEncoding` key.
    pub type CheckRoundTripEncoding = keys::Any<validate::CheckRoundTripEncoding>;

    mod check_round_trip_encoding {
        use std::borrow::Cow;

        use crate::{
            bstr::{BStr, ByteSlice},
            config,
            config::tree::{core::CheckRoundTripEncoding, Key},
        };

        impl CheckRoundTripEncoding {
            /// Convert `value` into a list of encodings, which are either space or coma separated. Fail if an encoding is unknown.
            /// If `None`, the default is returned.
            pub fn try_into_encodings(
                &'static self,
                value: Option<Cow<'_, BStr>>,
            ) -> Result<Vec<&'static gix_filter::encoding::Encoding>, config::encoding::Error> {
                Ok(match value {
                    None => vec![gix_filter::encoding::SHIFT_JIS],
                    Some(value) => {
                        let mut out = Vec::new();
                        for encoding in value
                            .as_ref()
                            .split(|b| *b == b',' || *b == b' ')
                            .filter(|e| !e.trim().is_empty())
                        {
                            out.push(
                                gix_filter::encoding::Encoding::for_label(encoding.trim()).ok_or_else(|| {
                                    config::encoding::Error {
                                        key: self.logical_name().into(),
                                        value: value.as_ref().to_owned(),
                                        encoding: encoding.into(),
                                    }
                                })?,
                            );
                        }
                        out
                    }
                })
            }
        }
    }

    mod eol {
        use std::borrow::Cow;

        use crate::{
            bstr::{BStr, ByteSlice},
            config,
            config::tree::core::Eol,
        };

        impl Eol {
            /// Convert `value` into the default end-of-line mode.
            ///
            /// ### Deviation
            ///
            /// git will allow any value and silently leaves it unset, we will fail if the value is not known.
            pub fn try_into_eol(
                &'static self,
                value: Cow<'_, BStr>,
            ) -> Result<gix_filter::eol::Mode, config::key::GenericErrorWithValue> {
                Ok(match value.to_str_lossy().as_ref() {
                    "lf" => gix_filter::eol::Mode::Lf,
                    "crlf" => gix_filter::eol::Mode::CrLf,
                    "native" => gix_filter::eol::Mode::default(),
                    _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
                })
            }
        }
    }

    mod safecrlf {
        use std::borrow::Cow;

        use gix_filter::pipeline::CrlfRoundTripCheck;

        use crate::{bstr::BStr, config, config::tree::core::SafeCrlf};

        impl SafeCrlf {
            /// Convert `value` into the safe-crlf enumeration, if possible.
            pub fn try_into_safecrlf(
                &'static self,
                value: Cow<'_, BStr>,
            ) -> Result<CrlfRoundTripCheck, config::key::GenericErrorWithValue> {
                if value.as_ref() == "warn" {
                    return Ok(CrlfRoundTripCheck::Warn);
                }
                let value = gix_config::Boolean::try_from(value.as_ref()).map_err(|err| {
                    config::key::GenericErrorWithValue::from_value(self, value.into_owned()).with_source(err)
                })?;
                Ok(if value.into() {
                    CrlfRoundTripCheck::Fail
                } else {
                    CrlfRoundTripCheck::Skip
                })
            }
        }
    }

    mod autocrlf {
        use std::borrow::Cow;

        use gix_filter::eol;

        use crate::{bstr::BStr, config, config::tree::core::AutoCrlf};

        impl AutoCrlf {
            /// Convert `value` into the safe-crlf enumeration, if possible.
            pub fn try_into_autocrlf(
                &'static self,
                value: Cow<'_, BStr>,
            ) -> Result<eol::AutoCrlf, config::key::GenericErrorWithValue> {
                if value.as_ref() == "input" {
                    return Ok(eol::AutoCrlf::Input);
                }
                let value = gix_config::Boolean::try_from(value.as_ref()).map_err(|err| {
                    config::key::GenericErrorWithValue::from_value(self, value.into_owned()).with_source(err)
                })?;
                Ok(if value.into() {
                    eol::AutoCrlf::Enabled
                } else {
                    eol::AutoCrlf::Disabled
                })
            }
        }
    }
}
#[cfg(feature = "attributes")]
pub use filter::*;

#[cfg(feature = "revision")]
mod disambiguate {
    use std::borrow::Cow;

    use crate::{
        bstr::{BStr, ByteSlice},
        config,
        config::tree::core::Disambiguate,
        revision::spec::parse::ObjectKindHint,
    };

    impl Disambiguate {
        /// Convert a disambiguation marker into the respective enum.
        pub fn try_into_object_kind_hint(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<Option<ObjectKindHint>, config::key::GenericErrorWithValue> {
            let hint = match value.as_ref().as_bytes() {
                b"none" => return Ok(None),
                b"commit" => ObjectKindHint::Commit,
                b"committish" => ObjectKindHint::Committish,
                b"tree" => ObjectKindHint::Tree,
                b"treeish" => ObjectKindHint::Treeish,
                b"blob" => ObjectKindHint::Blob,
                _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
            };
            Ok(Some(hint))
        }
    }
}

mod log_all_ref_updates {
    use crate::{config, config::tree::core::LogAllRefUpdates};

    impl LogAllRefUpdates {
        /// Returns the mode for ref-updates as parsed from `value`. If `value` is not a boolean, we try
        /// to interpret the string value instead. For correctness, this two step process is necessary as
        /// the interpretation of booleans in special in `git-config`, i.e. we can't just treat it as string.
        pub fn try_into_ref_updates(
            &'static self,
            value: Option<Result<bool, gix_config::value::Error>>,
        ) -> Result<Option<gix_ref::store::WriteReflog>, config::key::GenericErrorWithValue> {
            match value {
                Some(Ok(bool)) => Ok(Some(if bool {
                    gix_ref::store::WriteReflog::Normal
                } else {
                    gix_ref::store::WriteReflog::Disable
                })),
                Some(Err(err)) => match err.input {
                    val if val.eq_ignore_ascii_case(b"always") => Ok(Some(gix_ref::store::WriteReflog::Always)),
                    val => Err(config::key::GenericErrorWithValue::from_value(self, val)),
                },
                None => Ok(None),
            }
        }
    }
}

mod check_stat {
    use std::borrow::Cow;

    use crate::{
        bstr::{BStr, ByteSlice},
        config,
        config::tree::core::CheckStat,
    };

    impl CheckStat {
        /// Returns true if the full set of stat entries should be checked, and it's just as lenient as git.
        pub fn try_into_checkstat(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<bool, config::key::GenericErrorWithValue> {
            Ok(match value.as_ref().as_bytes() {
                b"minimal" => false,
                b"default" => true,
                _ => {
                    return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned()));
                }
            })
        }
    }
}

mod abbrev {
    use std::borrow::Cow;

    use config::abbrev::Error;

    use crate::{
        bstr::{BStr, ByteSlice},
        config,
        config::tree::core::Abbrev,
    };

    impl Abbrev {
        /// Convert the given `hex_len_str` into the amount of characters that a short hash should have.
        /// If `None` is returned, the correct value can be determined based on the amount of objects in the repo.
        pub fn try_into_abbreviation(
            &'static self,
            hex_len_str: Cow<'_, BStr>,
            object_hash: gix_hash::Kind,
        ) -> Result<Option<usize>, Error> {
            let max = object_hash.len_in_hex() as u8;
            if hex_len_str.trim().is_empty() {
                return Err(Error {
                    value: hex_len_str.into_owned(),
                    max,
                });
            }
            if hex_len_str.trim().eq_ignore_ascii_case(b"auto") {
                Ok(None)
            } else {
                let value_bytes = hex_len_str.as_ref();
                if let Ok(false) = gix_config::Boolean::try_from(value_bytes).map(Into::into) {
                    Ok(object_hash.len_in_hex().into())
                } else {
                    let value = gix_config::Integer::try_from(value_bytes)
                        .map_err(|_| Error {
                            value: hex_len_str.clone().into_owned(),
                            max,
                        })?
                        .to_decimal()
                        .ok_or_else(|| Error {
                            value: hex_len_str.clone().into_owned(),
                            max,
                        })?;
                    if value < 4 || value as usize > object_hash.len_in_hex() {
                        return Err(Error {
                            value: hex_len_str.clone().into_owned(),
                            max,
                        });
                    }
                    Ok(Some(value as usize))
                }
            }
        }
    }
}

mod validate {
    use crate::{bstr::BStr, config::tree::keys};

    pub struct Disambiguate;
    impl keys::Validate for Disambiguate {
        #[cfg_attr(not(feature = "revision"), allow(unused_variables))]
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            #[cfg(feature = "revision")]
            super::Core::DISAMBIGUATE.try_into_object_kind_hint(value.into())?;
            Ok(())
        }
    }

    pub struct LogAllRefUpdates;
    impl keys::Validate for LogAllRefUpdates {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::LOG_ALL_REF_UPDATES
                .try_into_ref_updates(Some(gix_config::Boolean::try_from(value).map(|b| b.0)))?;
            Ok(())
        }
    }

    pub struct CheckStat;
    impl keys::Validate for CheckStat {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::CHECK_STAT.try_into_checkstat(value.into())?;
            Ok(())
        }
    }

    pub struct Abbrev;
    impl keys::Validate for Abbrev {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            // TODO: when there is options, validate against all hashes and assure all fail to trigger a validation failure.
            super::Core::ABBREV.try_into_abbreviation(value.into(), gix_hash::Kind::Sha1)?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    pub struct SafeCrlf;
    #[cfg(feature = "attributes")]
    impl keys::Validate for SafeCrlf {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::SAFE_CRLF.try_into_safecrlf(value.into())?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    pub struct AutoCrlf;
    #[cfg(feature = "attributes")]
    impl keys::Validate for AutoCrlf {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::AUTO_CRLF.try_into_autocrlf(value.into())?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    pub struct Eol;
    #[cfg(feature = "attributes")]
    impl keys::Validate for Eol {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::EOL.try_into_eol(value.into())?;
            Ok(())
        }
    }

    #[cfg(feature = "attributes")]
    pub struct CheckRoundTripEncoding;
    #[cfg(feature = "attributes")]
    impl keys::Validate for CheckRoundTripEncoding {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            super::Core::CHECK_ROUND_TRIP_ENCODING.try_into_encodings(Some(value.into()))?;
            Ok(())
        }
    }
}
