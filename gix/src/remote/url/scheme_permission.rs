use gix_error::ResultExt;

use std::collections::BTreeMap;

use crate::{
    bstr::{BStr, BString, ByteSlice},
    config::tree::{Protocol, gitoxide},
};

/// All allowed values of the `protocol.allow` key.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Allow {
    /// Allow use this protocol.
    Always,
    /// Forbid using this protocol
    Never,
    /// Only supported if `GIT_PROTOCOL_FROM_USER` is unset or evaluates to true.
    User,
}

/// The error returned when obtaining transport permissions from configuration.
pub type Error = gix_error::Error;

impl Allow {
    /// Return true if we represent something like 'allow == true'.
    pub fn to_bool(self, user_allowed: Option<bool>) -> bool {
        match self {
            Allow::Always => true,
            Allow::Never => false,
            Allow::User => user_allowed.unwrap_or(true),
        }
    }
}

impl TryFrom<&BStr> for Allow {
    type Error = BString;

    fn try_from(v: &BStr) -> Result<Self, Self::Error> {
        Ok(match v.as_bytes() {
            b"never" => Allow::Never,
            b"always" => Allow::Always,
            b"user" => Allow::User,
            unknown => return Err(unknown.into()),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SchemePermission {
    /// `None` if the env-var is unset, otherwise its parsed boolean value.
    user_allowed: Option<bool>,
    /// The general allow value from `protocol.allow`.
    allow: Option<Allow>,
    /// Per scheme allow information
    allow_per_scheme: BTreeMap<String, Allow>,
}

/// Init
impl SchemePermission {
    /// NOTE: _intentionally without leniency_
    pub fn from_config(
        config: &gix_config::File,
        mut filter: fn(&gix_config::file::Metadata) -> bool,
    ) -> Result<Self, Error> {
        if let Some(allow_protocol) = config.string_filter(gitoxide::Allow::PROTOCOL, &mut filter) {
            return Ok(SchemePermission {
                user_allowed: None,
                allow: Some(Allow::Never),
                allow_per_scheme: allow_protocol
                    .split(|b| *b == b':')
                    .filter_map(|name| name.to_str().ok().map(|name| (name.to_owned(), Allow::Always)))
                    .collect(),
            });
        }
        let allow: Option<Allow> = config
            .string_filter("protocol.allow", &mut filter)
            .map(|value| Protocol::ALLOW.try_into_allow(value, None))
            .transpose()?;

        let allow_per_scheme = match config.sections_by_name_and_filter("protocol", &mut filter) {
            Some(it) => {
                let mut map = BTreeMap::default();
                for (section, scheme) in it.filter_map(|section| {
                    section
                        .header()
                        .subsection_name()
                        .and_then(|scheme| scheme.to_str().ok().map(|scheme| (section, scheme)))
                }) {
                    if let Some(value) = section
                        .value("allow")
                        .map(|value| Protocol::ALLOW.try_into_allow(value, Some(scheme)))
                        .transpose()?
                    {
                        map.insert(scheme.to_owned(), value);
                    }
                }
                map
            }
            None => Default::default(),
        };

        let user_allowed = gitoxide::Allow::PROTOCOL_FROM_USER
            .enrich_error(config.boolean_filter(gitoxide::Allow::PROTOCOL_FROM_USER, &mut filter))
            .or_erased()?;
        Ok(SchemePermission {
            allow,
            allow_per_scheme,
            user_allowed,
        })
    }
}

/// Access
impl SchemePermission {
    pub fn allow(&self, scheme: &gix_url::Scheme) -> bool {
        self.allow_per_scheme
            .get(scheme.as_str())
            .or(self.allow.as_ref())
            .map_or_else(
                || {
                    use gix_url::Scheme::*;
                    match scheme {
                        File | Git | Ssh | Http | Https => true,
                        Ext => false,
                        Helper(name) | HelperUrl(name) => match name.as_str() {
                            // Git applies its known-safe defaults by transport name even when that name selects a
                            // remote helper, so e.g. `ssh::address` inherits the default policy of `ssh`.
                            "http" | "https" | "git" | "ssh" => true,
                            "ext" => false,
                            _ => Allow::User.to_bool(self.user_allowed),
                        },
                    }
                },
                |allow| allow.to_bool(self.user_allowed),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{Allow, SchemePermission};

    #[test]
    fn helper_forms_use_the_transport_name_for_permissions() {
        let permissions = SchemePermission {
            user_allowed: None,
            allow: None,
            allow_per_scheme: [("foo".into(), Allow::Always), ("ext".into(), Allow::Always)].into(),
        };

        assert!(permissions.allow(&gix_url::Scheme::Helper("foo".into())));
        assert!(permissions.allow(&gix_url::Scheme::HelperUrl("foo".into())));
        assert!(permissions.allow(&gix_url::Scheme::Ext));
    }

    #[test]
    fn unknown_helpers_default_to_user_while_ext_remains_denied() {
        for scheme in [
            gix_url::Scheme::Helper("foo".into()),
            gix_url::Scheme::HelperUrl("foo".into()),
        ] {
            let permissions = |user_allowed| SchemePermission {
                user_allowed,
                allow: None,
                allow_per_scheme: Default::default(),
            };
            assert!(
                permissions(None).allow(&scheme),
                "by default, extra helpers are allowed"
            );
            assert!(permissions(Some(true)).allow(&scheme));
            assert!(!permissions(Some(false)).allow(&scheme), "but they can be disallowed");
        }

        let permissions = SchemePermission {
            user_allowed: None,
            allow: None,
            allow_per_scheme: Default::default(),
        };
        assert!(
            !permissions.allow(&gix_url::Scheme::Ext),
            "extension commands are disallowed, they can invoke any command otherwise"
        );
        assert!(!permissions.allow(&gix_url::Scheme::HelperUrl("ext".into())));

        let no_user = SchemePermission {
            user_allowed: Some(false),
            ..permissions
        };
        assert!(no_user.allow(&gix_url::Scheme::Helper("ssh".into())));
        assert!(no_user.allow(&gix_url::Scheme::HelperUrl("https".into())));
        assert!(
            !no_user.allow(&gix_url::Scheme::Helper("file".into())),
            "file::address invokes git-remote-file, which has the default user policy and is denied when GIT_PROTOCOL_FROM_USER is false"
        );
    }

    #[test]
    fn allow_protocol_overrides_all_other_policy() {
        let config = gix_config::File::from_bytes_no_includes(
            br#"[gitoxide "allow"]
protocol = foo
[protocol]
allow = always
[protocol "ext"]
allow = always
"#,
            gix_config::file::Metadata::default(),
            Default::default(),
        )
        .expect("valid test configuration");
        let permissions = SchemePermission::from_config(&config, |_| true).expect("valid permissions");

        assert!(
            permissions.allow(&gix_url::Scheme::Helper("foo".into())),
            "listed helpers are allowed"
        );
        assert!(
            !permissions.allow(&gix_url::Scheme::Helper("Foo".into())),
            "protocol names are case-sensitive"
        );
        assert!(
            !permissions.allow(&gix_url::Scheme::Ext),
            "the allowlist overrides protocol.ext.allow=always"
        );
        assert!(
            !permissions.allow(&gix_url::Scheme::Https),
            "the allowlist overrides known-safe defaults"
        );
    }
}
