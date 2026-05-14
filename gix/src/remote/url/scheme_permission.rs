use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    bstr::{BStr, BString, ByteSlice},
    config,
    config::tree::{Protocol, gitoxide},
};

/// All allowed values of the `protocol.allow` key.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Allow {
    /// Allow use this protocol.
    Always,
    /// Forbid using this protocol
    Never,
    /// Only supported if the `GIT_PROTOCOL_FROM_USER` is unset or is set to `1`.
    User,
}

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

impl<'a> TryFrom<Cow<'a, BStr>> for Allow {
    type Error = BString;

    fn try_from(v: Cow<'a, BStr>) -> Result<Self, Self::Error> {
        Ok(match v.as_ref().as_bytes() {
            b"never" => Allow::Never,
            b"always" => Allow::Always,
            b"user" => Allow::User,
            unknown => return Err(unknown.into()),
        })
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SchemePermission {
    /// `None`, env-var is unset or wasn't queried, otherwise true if `GIT_PROTOCOL_FROM_USER` is `1`.
    user_allowed: Option<bool>,
    /// The general allow value from `protocol.allow`.
    allow: Option<Allow>,
    /// Per scheme allow information
    allow_per_scheme: BTreeMap<gix_url::Scheme, Allow>,
}

/// Init
impl SchemePermission {
    /// NOTE: _intentionally without leniency_
    pub fn from_config(
        config: &gix_config::File<'static>,
        mut filter: fn(&gix_config::file::Metadata) -> bool,
    ) -> Result<Self, config::protocol::allow::Error> {
        let allow: Option<Allow> = config
            .string_filter("protocol.allow", &mut filter)
            .map(|value| Protocol::ALLOW.try_into_allow(value, None))
            .transpose()?;

        let mut saw_user = allow == Some(Allow::User);
        let allow_per_scheme = match config.sections_by_name_and_filter("protocol", &mut filter) {
            Some(it) => {
                let mut map = BTreeMap::default();
                for (section, scheme) in it.filter_map(|section| {
                    section.header().subsection_name().and_then(|scheme| {
                        scheme
                            .to_str()
                            .ok()
                            .map(|scheme| (section, gix_url::Scheme::from(scheme)))
                    })
                }) {
                    if let Some(value) = section
                        .value("allow")
                        .map(|value| Protocol::ALLOW.try_into_allow(value, Some(scheme.as_str())))
                        .transpose()?
                    {
                        saw_user |= value == Allow::User;
                        map.insert(scheme, value);
                    }
                }
                map
            }
            None => Default::default(),
        };

        let user_allowed = saw_user.then(|| {
            config
                .string_filter(gitoxide::Allow::PROTOCOL_FROM_USER, &mut filter)
                .is_none_or(|val| val.as_ref() == "1")
        });
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
        self.allow_per_scheme.get(scheme).or(self.allow.as_ref()).map_or_else(
            || {
                use gix_url::Scheme::*;
                match scheme {
                    File | Git | Ssh | Http | Https => true,
                    Ext(_) => false,
                    // TODO: figure out what 'ext' really entails, and what 'other' protocols are which aren't representable for us yet
                }
            },
            |allow| allow.to_bool(self.user_allowed),
        )
    }
}
