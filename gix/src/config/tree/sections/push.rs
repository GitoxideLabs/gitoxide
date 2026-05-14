use crate::{
    config,
    config::tree::{Key, Push, Section, keys},
};

impl Push {
    /// The `push.default` key
    pub const DEFAULT: Default = Default::new_with_validate("default", &config::Tree::PUSH, validate::Default);
}

impl Section for Push {
    fn name(&self) -> &str {
        "push"
    }

    fn keys(&self) -> &[&dyn Key] {
        &[&Self::DEFAULT]
    }
}

/// The `remote.<name>.tagOpt` key type.
pub type Default = keys::Any<validate::Default>;

mod default {
    use std::borrow::Cow;

    use crate::{
        bstr::{BStr, ByteSlice},
        config,
        config::tree::push::Default,
        push,
    };

    impl Default {
        /// Try to interpret `value` as `push.default`.
        pub fn try_into_default(
            &'static self,
            value: Cow<'_, BStr>,
        ) -> Result<push::Default, config::key::GenericErrorWithValue> {
            Ok(match value.as_ref().as_bytes() {
                b"nothing" => push::Default::Nothing,
                b"current" => push::Default::Current,
                b"upstream" | b"tracking" => push::Default::Upstream,
                b"simple" => push::Default::Simple,
                b"matching" => push::Default::Matching,
                _ => return Err(config::key::GenericErrorWithValue::from_value(self, value.into_owned())),
            })
        }
    }
}

mod validate {
    #[derive(Clone, Copy)]
    pub struct Default;
    use std::{borrow::Cow, error::Error};

    use crate::{bstr::BStr, config::tree::keys::Validate};

    impl Validate for Default {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            super::Push::DEFAULT.try_into_default(Cow::Borrowed(value))?;
            Ok(())
        }
    }
}
