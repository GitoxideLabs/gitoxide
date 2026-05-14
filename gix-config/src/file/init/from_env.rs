use std::convert::TryFrom;

use bstr::ByteSlice;

use crate::{File, KeyRef, file, file::init, parse::section, path::interpolate};

/// Represents the errors that may occur when calling [`File::from_env()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Configuration {kind} at index {index} contained illformed UTF-8")]
    IllformedUtf8 { index: usize, kind: &'static str },
    #[error("GIT_CONFIG_COUNT was not a positive integer: {}", .input)]
    InvalidConfigCount { input: String },
    #[error("GIT_CONFIG_KEY_{} was not set", .key_id)]
    InvalidKeyId { key_id: usize },
    #[error("GIT_CONFIG_KEY_{} was set to an invalid value: {}", .key_id, .key_val)]
    InvalidKeyValue { key_id: usize, key_val: String },
    #[error("GIT_CONFIG_VALUE_{} was not set", .value_id)]
    InvalidValueId { value_id: usize },
    #[error(transparent)]
    PathInterpolationError(#[from] interpolate::Error),
    #[error(transparent)]
    Includes(#[from] init::includes::Error),
    #[error(transparent)]
    Section(#[from] section::header::Error),
    #[error(transparent)]
    ValueName(#[from] section::value_name::Error),
}

/// Instantiation from environment variables
impl File<'static> {
    /// Generates a config from `GIT_CONFIG_*` environment variables or returns `Ok(None)` if no configuration was found.
    /// See [`git-config`'s documentation] for more information on the environment variables in question.
    ///
    /// With `options` configured, it's possible to resolve `include.path` or `includeIf.<condition>.path` directives as well.
    ///
    /// [`git-config`'s documentation]: https://git-scm.com/docs/git-config#Documentation/git-config.txt-GITCONFIGCOUNT
    pub fn from_env(options: init::Options<'_>) -> Result<Option<File<'static>>, Error> {
        use std::env;
        let count: usize = match env::var("GIT_CONFIG_COUNT") {
            Ok(v) => v.parse().map_err(|_| Error::InvalidConfigCount { input: v })?,
            Err(_) => return Ok(None),
        };

        if count == 0 {
            return Ok(None);
        }

        let meta = file::Metadata {
            path: None,
            source: crate::Source::Env,
            level: 0,
            trust: gix_sec::Trust::Full,
        };
        let mut config = File::new(meta);
        for i in 0..count {
            let key = gix_path::os_string_into_bstring(
                env::var_os(format!("GIT_CONFIG_KEY_{i}")).ok_or(Error::InvalidKeyId { key_id: i })?,
            )
            .map_err(|_| Error::IllformedUtf8 { index: i, kind: "key" })?;
            let value = env::var_os(format!("GIT_CONFIG_VALUE_{i}")).ok_or(Error::InvalidValueId { value_id: i })?;
            let key = KeyRef::parse_unvalidated(key.as_ref()).ok_or_else(|| Error::InvalidKeyValue {
                key_id: i,
                key_val: key.to_string(),
            })?;

            config
                .section_mut_or_create_new(key.section_name, key.subsection_name)?
                .push(
                    section::ValueName::try_from(key.value_name.to_owned())?,
                    Some(
                        gix_path::os_str_into_bstr(&value)
                            .map_err(|_| Error::IllformedUtf8 {
                                index: i,
                                kind: "value",
                            })?
                            .as_bytes()
                            .into(),
                    ),
                );
        }

        let mut buf = Vec::new();
        init::includes::resolve(&mut config, &mut buf, options)?;
        Ok(Some(config))
    }
}
