use std::ffi::OsString;

use crate::{Defaults, MagicSignature, SearchMode};

///
pub mod from_environment {
    /// The error returned by [Defaults::from_environment()](super::Defaults::from_environment()).
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        ParseValue(gix_config_value::Error),
        MixedGlobAndNoGlob,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::ParseValue(err) => std::fmt::Display::fmt(err, f),
                Error::MixedGlobAndNoGlob => f.write_str("Glob and no-glob settings are mutually exclusive"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::ParseValue(err) => err.source(),
                Error::MixedGlobAndNoGlob => None,
            }
        }
    }

    impl From<gix_config_value::Error> for Error {
        fn from(err: gix_config_value::Error) -> Self {
            Error::ParseValue(err)
        }
    }
}

impl Defaults {
    /// Initialize this instance using information from the environment as
    /// [per the official documentation](https://git-scm.com/book/en/v2/Git-Internals-Environment-Variables) *(look for `PATHSPECS`)*,
    /// calling `var(variable_name)` for each variable that should be obtained.
    ///
    /// Used environment variables are `GIT_GLOB_PATHSPECS`, `GIT_NOGLOB_PATHSPECS`, `GIT_LITERAL_PATHSPECS` and `GIT_ICASE_PATHSPECS`.
    /// Note that there are lot of failure modes, and instead of offering lenient parsing, the caller may ignore errors and
    /// use other defaults instead.
    ///
    /// ### Deviation
    ///
    /// Instead of failing if `GIT_LITERAL_PATHSPECS` is used with glob globals, we ignore these. Also our implementation allows global
    /// `icase` settings in combination with this setting.
    pub fn from_environment(var: &mut dyn FnMut(&str) -> Option<OsString>) -> Result<Self, from_environment::Error> {
        let mut env_bool = |name: &str| -> Result<Option<bool>, gix_config_value::Error> {
            var(name)
                .map(|val| gix_config_value::Boolean::try_from(val).map(|b| b.0))
                .transpose()
        };

        let literal = env_bool("GIT_LITERAL_PATHSPECS")?.unwrap_or_default();
        let signature = env_bool("GIT_ICASE_PATHSPECS")?
            .and_then(|val| val.then_some(MagicSignature::ICASE))
            .unwrap_or_default();
        if literal {
            return Ok(Defaults {
                signature,
                search_mode: SearchMode::Literal,
                literal,
            });
        }
        let glob = env_bool("GIT_GLOB_PATHSPECS")?;
        let mut search_mode = glob
            .and_then(|glob| glob.then_some(SearchMode::PathAwareGlob))
            .unwrap_or_default();
        search_mode = env_bool("GIT_NOGLOB_PATHSPECS")?
            .map(|no_glob| {
                if glob.unwrap_or_default() && no_glob {
                    Err(from_environment::Error::MixedGlobAndNoGlob)
                } else {
                    Ok(SearchMode::Literal)
                }
            })
            .transpose()?
            .unwrap_or(search_mode);

        Ok(Defaults {
            signature,
            search_mode,
            literal,
        })
    }
}
