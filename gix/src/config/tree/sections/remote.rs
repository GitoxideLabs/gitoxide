use crate::{
    config,
    config::tree::{Key, Remote, Section, SubSectionRequirement, http, keys},
};

const NAME_PARAMETER: Option<SubSectionRequirement> = Some(SubSectionRequirement::Parameter("name"));

impl Remote {
    /// The `remote.pushDefault` key
    pub const PUSH_DEFAULT: keys::RemoteName = keys::RemoteName::new_remote_name("pushDefault", &config::Tree::REMOTE);
    /// The `remote.<name>.tagOpt` key
    pub const TAG_OPT: TagOpt = TagOpt::new_with_validate("tagOpt", &config::Tree::REMOTE, validate::TagOpt)
        .with_subsection_requirement(Some(SubSectionRequirement::Parameter("name")));
    /// The `remote.<name>.url` key
    pub const URL: keys::Url =
        keys::Url::new_url("url", &config::Tree::REMOTE).with_subsection_requirement(NAME_PARAMETER);
    /// The `remote.<name>.pushUrl` key
    pub const PUSH_URL: keys::Url =
        keys::Url::new_url("pushUrl", &config::Tree::REMOTE).with_subsection_requirement(NAME_PARAMETER);
    /// The `remote.<name>.fetch` key
    pub const FETCH: keys::FetchRefSpec = keys::FetchRefSpec::new_fetch_refspec("fetch", &config::Tree::REMOTE)
        .with_subsection_requirement(NAME_PARAMETER);
    /// The `remote.<name>.push` key
    pub const PUSH: keys::PushRefSpec =
        keys::PushRefSpec::new_push_refspec("push", &config::Tree::REMOTE).with_subsection_requirement(NAME_PARAMETER);
    /// The `remote.<name>.proxy` key
    pub const PROXY: keys::String =
        keys::String::new_string("proxy", &config::Tree::REMOTE).with_subsection_requirement(NAME_PARAMETER);
    /// The `remote.<name>.proxyAuthMethod` key.
    pub const PROXY_AUTH_METHOD: http::ProxyAuthMethod =
        http::ProxyAuthMethod::new_proxy_auth_method("proxyAuthMethod", &config::Tree::REMOTE)
            .with_subsection_requirement(NAME_PARAMETER)
            .with_deviation("implemented like git, but never actually tried");
}

impl Section for Remote {
    fn name(&self) -> &str {
        "remote"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[
            &Self::PUSH_DEFAULT,
            &Self::TAG_OPT,
            &Self::URL,
            &Self::PUSH_URL,
            &Self::FETCH,
            &Self::PUSH,
            &Self::PROXY,
            &Self::PROXY_AUTH_METHOD,
        ]
    }
}

/// The `remote.<name>.tagOpt` key type.
pub type TagOpt = keys::Any<validate::TagOpt>;

mod tag_opts {
    use std::borrow::Cow;

    use crate::{
        bstr::{BStr, ByteSlice},
        config,
        config::tree::remote::TagOpt,
        remote,
    };

    impl TagOpt {
        /// Try to interpret `value` as tag option.
        ///
        /// # Note
        ///
        /// It's heavily biased towards the git command-line unfortunately, and the only
        /// value of its kind. Maybe in future more values will be supported which are less
        /// about passing them to a sub-process.
        pub fn try_into_tag_opt(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<remote::fetch::Tags, config::key::GenericErrorWithValue> {
            Ok(match value.as_ref().as_bytes() {
                b"--tags" => remote::fetch::Tags::All,
                b"--no-tags" => remote::fetch::Tags::None,
                _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
            })
        }
    }
}

pub mod validate {
    use std::{borrow::Cow, error::Error};

    use crate::{bstr::BStr, config::tree::keys::Validate};

    pub struct TagOpt;
    impl Validate for TagOpt {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            super::Remote::TAG_OPT.try_into_tag_opt(Cow::Borrowed(value))?;
            Ok(())
        }
    }
}
