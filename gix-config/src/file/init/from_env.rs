use bstr::ByteSlice;

use crate::{File, KeyRef, file, file::init};

/// Represents the errors that may occur when calling [`File::from_env()`].
pub type Error = gix_error::Exn;

/// Instantiation from environment variables
impl File {
    /// Generates a config from `GIT_CONFIG_*` environment variables or returns `Ok(None)` if no configuration was found.
    /// See [`git-config`'s documentation] for more information on the environment variables in question.
    ///
    /// With `options` configured, it's possible to resolve `include.path` or `includeIf.<condition>.path` directives as well.
    ///
    /// [`git-config`'s documentation]: https://git-scm.com/docs/git-config#Documentation/git-config.txt-GITCONFIGCOUNT
    pub fn from_env(options: init::Options<'_>) -> Result<Option<File>, Error> {
        use gix_error::{ErrorExt, NotFoundError, OptionExt, ResultExt, ValidationError, message};
        use std::env;
        let count: usize = match env::var("GIT_CONFIG_COUNT") {
            Ok(v) => v.parse::<usize>().or_raise_erased(|| {
                ValidationError::new_with_input("GIT_CONFIG_COUNT was not a positive integer", v)
            })?,
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
                env::var_os(format!("GIT_CONFIG_KEY_{i}"))
                    .ok_or_raise_erased(|| NotFoundError::new(format!("GIT_CONFIG_KEY_{i} was not set")))?,
            )
            .or_raise_erased(|| {
                ValidationError::new(format!("Configuration key at index {i} contained illformed UTF-8"))
            })?;
            let value = env::var_os(format!("GIT_CONFIG_VALUE_{i}"))
                .ok_or_raise_erased(|| NotFoundError::new(format!("GIT_CONFIG_VALUE_{i} was not set")))?;
            let key = KeyRef::parse_unvalidated(key.as_ref()).ok_or_else(|| {
                ValidationError::new_with_input(format!("GIT_CONFIG_KEY_{i} was set to an invalid value"), key.clone())
                    .raise_erased()
            })?;

            config
                .section_mut_or_create_new_inner(key.section_name, key.subsection_name)
                .or_erased()?
                .push(
                    key.value_name,
                    Some(
                        gix_path::os_str_into_bstr(&value)
                            .or_raise_erased(|| {
                                ValidationError::new(format!(
                                    "Configuration value at index {i} contained illformed UTF-8"
                                ))
                            })?
                            .as_bytes()
                            .into(),
                    ),
                )
                .or_erased()?;
        }

        let mut buf = Vec::new();
        init::includes::resolve(&mut config, &mut buf, options)
            .or_raise_erased(|| message("Could not resolve includes in environment configuration"))?;
        Ok(Some(config))
    }
}
