#![allow(clippy::result_large_err)]
use std::{
    borrow::Cow,
    error::Error,
    fmt::{Debug, Formatter},
};

use gix_config::KeyRef;

use crate::{
    bstr::BStr,
    config,
    config::tree::{Key, Link, Note, Section, SubSectionRequirement},
};

/// Implements a value without any constraints, i.e. a any value.
pub struct Any<T: Validate = validate::All> {
    /// The key of the value in the git configuration.
    pub name: &'static str,
    /// The parent section of the key.
    pub section: &'static dyn Section,
    /// The subsection requirement to use.
    pub subsection_requirement: Option<SubSectionRequirement>,
    /// A link to other resources that might be eligible as value.
    pub link: Option<Link>,
    /// A note about this key.
    pub note: Option<Note>,
    /// The way validation and transformation should happen.
    validate: T,
}

/// Init
impl Any<validate::All> {
    /// Create a new instance from `name` and `section`
    pub const fn new(name: &'static str, section: &'static dyn Section) -> Self {
        Any::new_with_validate(name, section, validate::All)
    }
}

/// Init other validate implementations
impl<T: Validate> Any<T> {
    /// Create a new instance from `name` and `section`
    pub const fn new_with_validate(name: &'static str, section: &'static dyn Section, validate: T) -> Self {
        Any {
            name,
            section,
            subsection_requirement: Some(SubSectionRequirement::Never),
            link: None,
            note: None,
            validate,
        }
    }
}

/// Builder
impl<T: Validate> Any<T> {
    /// Set the subsection requirement to non-default values.
    pub const fn with_subsection_requirement(mut self, requirement: Option<SubSectionRequirement>) -> Self {
        self.subsection_requirement = requirement;
        self
    }

    /// Associate an environment variable with this key.
    ///
    /// This is mainly useful for enriching error messages.
    pub const fn with_environment_override(mut self, var: &'static str) -> Self {
        self.link = Some(Link::EnvironmentOverride(var));
        self
    }

    /// Set a link to another key which serves as fallback to provide a value if this key is not set.
    pub const fn with_fallback(mut self, key: &'static dyn Key) -> Self {
        self.link = Some(Link::FallbackKey(key));
        self
    }

    /// Attach an informative message to this key.
    pub const fn with_note(mut self, message: &'static str) -> Self {
        self.note = Some(Note::Informative(message));
        self
    }

    /// Inform about a deviation in how this key is interpreted.
    pub const fn with_deviation(mut self, message: &'static str) -> Self {
        self.note = Some(Note::Deviation(message));
        self
    }
}

/// Conversion
impl<T: Validate> Any<T> {
    /// Try to convert `value` into a refspec suitable for the `op` operation.
    pub fn try_into_refspec(
        &'static self,
        value: std::borrow::Cow<'_, BStr>,
        op: gix_refspec::parse::Operation,
    ) -> Result<gix_refspec::RefSpec, config::refspec::Error> {
        gix_refspec::parse(value.as_ref(), op)
            .map(|spec| spec.to_owned())
            .map_err(|err| config::refspec::Error::from_value(self, value.into_owned()).with_source(err))
    }

    /// Try to interpret `value` as UTF-8 encoded string.
    pub fn try_into_string(&'static self, value: Cow<'_, BStr>) -> Result<std::string::String, config::string::Error> {
        use crate::bstr::ByteVec;
        Vec::from(value.into_owned()).into_string().map_err(|err| {
            let utf8_err = err.utf8_error().clone();
            config::string::Error::from_value(self, err.into_vec().into()).with_source(utf8_err)
        })
    }
}

impl<T: Validate> Debug for Any<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.logical_name().fmt(f)
    }
}

impl<T: Validate> std::fmt::Display for Any<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.logical_name())
    }
}

impl<T: Validate> Key for Any<T> {
    fn name(&self) -> &str {
        self.name
    }

    fn validate(&self, value: &BStr) -> Result<(), config::tree::key::validate::Error> {
        Ok(self.validate.validate(value)?)
    }

    fn section(&self) -> &dyn Section {
        self.section
    }

    fn subsection_requirement(&self) -> Option<&SubSectionRequirement> {
        self.subsection_requirement.as_ref()
    }

    fn link(&self) -> Option<&Link> {
        self.link.as_ref()
    }

    fn note(&self) -> Option<&Note> {
        self.note.as_ref()
    }
}

impl<T: Validate> gix_config::AsKey for Any<T> {
    fn as_key(&self) -> gix_config::KeyRef<'_> {
        self.try_as_key().expect("infallible")
    }

    fn try_as_key(&self) -> Option<KeyRef<'_>> {
        let section_name = self.section.parent().map_or_else(|| self.section.name(), Section::name);
        let subsection_name = if self.section.parent().is_some() {
            Some(self.section.name().into())
        } else {
            None
        };
        let value_name = self.name;
        gix_config::KeyRef {
            section_name,
            subsection_name,
            value_name,
        }
        .into()
    }
}

/// A key which represents a date.
pub type Time = Any<validate::Time>;

/// The `core.(filesRefLockTimeout|packedRefsTimeout)` keys, or any other lock timeout for that matter.
pub type LockTimeout = Any<validate::LockTimeout>;

/// Keys specifying durations in milliseconds.
pub type DurationInMilliseconds = Any<validate::DurationInMilliseconds>;

/// A key which represents any unsigned integer.
pub type UnsignedInteger = Any<validate::UnsignedInteger>;

/// A key that represents a remote name, either as url or symbolic name.
pub type RemoteName = Any<validate::RemoteName>;

/// A key that represents a boolean value.
pub type Boolean = Any<validate::Boolean>;

/// A key that represents an executable program, shell script or shell commands.
///
/// Once obtained with [trusted_program()](crate::config::Snapshot::trusted_program())
/// one can run it with [command::prepare()](gix_command::prepare), possibly after
/// [obtaining](crate::Repository::command_context) and [setting](gix_command::Prepare::with_context)
/// a git [command context](gix_command::Context) (depending on the commands needs).
pub type Program = Any<validate::Program>;

/// A key that represents an executable program as identified by name or path.
///
/// Once obtained with [trusted_program()](crate::config::Snapshot::trusted_program())
/// one can run it with [command::prepare()](gix_command::prepare), possibly after
/// [obtaining](crate::Repository::command_context) and [setting](gix_command::Prepare::with_context)
/// a git [command context](gix_command::Context) (depending on the commands needs).
pub type Executable = Any<validate::Executable>;

/// A key that represents a path (to a resource).
pub type Path = Any<validate::Path>;

/// A key that represents a URL.
pub type Url = Any<validate::Url>;

/// A key that represents a UTF-8 string.
pub type String = Any<validate::String>;

/// A key that represents a `RefSpec` for pushing.
pub type PushRefSpec = Any<validate::PushRefSpec>;

/// A key that represents a `RefSpec` for fetching.
pub type FetchRefSpec = Any<validate::FetchRefSpec>;

mod duration {
    use std::time::Duration;

    use crate::{
        config,
        config::tree::{keys::DurationInMilliseconds, Section},
    };

    impl DurationInMilliseconds {
        /// Create a new instance.
        pub const fn new_duration(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::DurationInMilliseconds)
        }

        /// Return a valid duration as parsed from an integer that is interpreted as milliseconds.
        pub fn try_into_duration(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<std::time::Duration, config::duration::Error> {
            let value = value.map_err(|err| config::duration::Error::from(self).with_source(err))?;
            Ok(match value {
                val if val < 0 => Duration::from_secs(u64::MAX),
                val => Duration::from_millis(val.try_into().expect("i64 to u64 always works if positive")),
            })
        }
    }
}

mod lock_timeout {
    use std::time::Duration;

    use gix_lock::acquire::Fail;

    use crate::{
        config,
        config::tree::{keys::LockTimeout, Section},
    };

    impl LockTimeout {
        /// Create a new instance.
        pub const fn new_lock_timeout(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::LockTimeout)
        }

        /// Return information on how long to wait for locked files.
        pub fn try_into_lock_timeout(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<gix_lock::acquire::Fail, config::lock_timeout::Error> {
            let value = value.map_err(|err| config::lock_timeout::Error::from(self).with_source(err))?;
            Ok(match value {
                val if val < 0 => Fail::AfterDurationWithBackoff(Duration::from_secs(u64::MAX)),
                0 => Fail::Immediately,
                val => Fail::AfterDurationWithBackoff(Duration::from_millis(
                    val.try_into().expect("i64 to u64 always works if positive"),
                )),
            })
        }
    }
}

mod refspecs {
    use crate::config::tree::{
        keys::{validate, FetchRefSpec, PushRefSpec},
        Section,
    };

    impl PushRefSpec {
        /// Create a new instance.
        pub const fn new_push_refspec(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::PushRefSpec)
        }
    }

    impl FetchRefSpec {
        /// Create a new instance.
        pub const fn new_fetch_refspec(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::FetchRefSpec)
        }
    }
}

mod url {
    use std::borrow::Cow;

    use crate::{
        bstr::BStr,
        config,
        config::tree::{
            keys::{validate, Url},
            Section,
        },
    };

    impl Url {
        /// Create a new instance.
        pub const fn new_url(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::Url)
        }

        /// Try to parse `value` as URL.
        pub fn try_into_url(&'static self, value: Cow<'_, BStr>) -> Result<gix_url::Url, config::url::Error> {
            gix_url::parse(value.as_ref())
                .map_err(|err| config::url::Error::from_value(self, value.into_owned()).with_source(err))
        }
    }
}

impl String {
    /// Create a new instance.
    pub const fn new_string(name: &'static str, section: &'static dyn Section) -> Self {
        Self::new_with_validate(name, section, validate::String)
    }
}

impl Program {
    /// Create a new instance.
    pub const fn new_program(name: &'static str, section: &'static dyn Section) -> Self {
        Self::new_with_validate(name, section, validate::Program)
    }
}

impl Executable {
    /// Create a new instance.
    pub const fn new_executable(name: &'static str, section: &'static dyn Section) -> Self {
        Self::new_with_validate(name, section, validate::Executable)
    }
}

impl Path {
    /// Create a new instance.
    pub const fn new_path(name: &'static str, section: &'static dyn Section) -> Self {
        Self::new_with_validate(name, section, validate::Path)
    }
}

mod workers {
    use crate::config::tree::{keys::UnsignedInteger, Section};

    impl UnsignedInteger {
        /// Create a new instance.
        pub const fn new_unsigned_integer(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::UnsignedInteger)
        }

        /// Convert `value` into a `usize` or wrap it into a specialized error.
        pub fn try_into_usize(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<usize, crate::config::unsigned_integer::Error> {
            value
                .map_err(|err| crate::config::unsigned_integer::Error::from(self).with_source(err))
                .and_then(|value| {
                    value
                        .try_into()
                        .map_err(|_| crate::config::unsigned_integer::Error::from(self))
                })
        }

        /// Convert `value` into a `u64` or wrap it into a specialized error.
        pub fn try_into_u64(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<u64, crate::config::unsigned_integer::Error> {
            value
                .map_err(|err| crate::config::unsigned_integer::Error::from(self).with_source(err))
                .and_then(|value| {
                    value
                        .try_into()
                        .map_err(|_| crate::config::unsigned_integer::Error::from(self))
                })
        }

        /// Convert `value` into a `u32` or wrap it into a specialized error.
        pub fn try_into_u32(
            &'static self,
            value: Result<i64, gix_config::value::Error>,
        ) -> Result<u32, crate::config::unsigned_integer::Error> {
            value
                .map_err(|err| crate::config::unsigned_integer::Error::from(self).with_source(err))
                .and_then(|value| {
                    value
                        .try_into()
                        .map_err(|_| crate::config::unsigned_integer::Error::from(self))
                })
        }
    }
}

mod time {
    use crate::{
        bstr::{BStr, ByteSlice},
        config::tree::{
            keys::{validate, Time},
            Section,
        },
    };
    use gix_error::Exn;
    use std::borrow::Cow;

    impl Time {
        /// Create a new instance.
        pub const fn new_time(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::Time)
        }

        /// Convert the `value` into a date if possible, with `now` as reference time for relative dates.
        pub fn try_into_time(
            &self,
            value: Cow<'_, BStr>,
            now: Option<std::time::SystemTime>,
        ) -> Result<gix_date::Time, Exn<gix_date::Error>> {
            gix_date::parse(
                value.as_ref().to_str().map_err(|_| {
                    gix_date::Error::new_with_input("UTF8 conversion failed", value.clone().into_owned())
                })?,
                now,
            )
        }
    }
}

mod boolean {
    use crate::{
        config,
        config::tree::{
            keys::{validate, Boolean},
            Section,
        },
    };

    impl Boolean {
        /// Create a new instance.
        pub const fn new_boolean(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::Boolean)
        }

        /// Process the `value` into a result with an improved error message.
        ///
        /// `value` is expected to be provided by [`gix_config::File::boolean()`].
        pub fn enrich_error(
            &'static self,
            value: Result<bool, gix_config::value::Error>,
        ) -> Result<bool, config::boolean::Error> {
            value.map_err(|err| config::boolean::Error::from(self).with_source(err))
        }
    }
}

mod remote_name {
    use std::borrow::Cow;

    use crate::{
        bstr::{BStr, BString},
        config,
        config::tree::{keys::RemoteName, Section},
    };

    impl RemoteName {
        /// Create a new instance.
        pub const fn new_remote_name(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::RemoteName)
        }

        /// Try to validate `name` as symbolic remote name and return it.
        #[allow(clippy::result_large_err)]
        pub fn try_into_symbolic_name(
            &'static self,
            name: Cow<'_, BStr>,
        ) -> Result<BString, config::remote::symbolic_name::Error> {
            crate::remote::name::validated(name.into_owned())
                .map_err(|err| config::remote::symbolic_name::Error::from(self).with_source(err))
        }
    }
}

/// Provide a way to validate a value, or decode a value from `git-config`.
pub trait Validate {
    /// Validate `value` or return an error.
    fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>>;
}

/// various implementations of the `Validate` trait.
pub mod validate {
    use std::{borrow::Cow, error::Error};

    use crate::{
        bstr::{BStr, ByteSlice},
        config::tree::keys::Validate,
        remote,
    };

    /// Everything is valid.
    #[derive(Default)]
    pub struct All;

    impl Validate for All {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            Ok(())
        }
    }

    /// Assure that values that parse as git dates are valid.
    #[derive(Default)]
    pub struct Time;

    impl Validate for Time {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            gix_date::parse(value.to_str()?, std::time::SystemTime::now().into())
                .map_err(gix_error::Exn::into_inner)?;
            Ok(())
        }
    }

    /// Assure that values that parse as unsigned integers are valid.
    #[derive(Default)]
    pub struct UnsignedInteger;

    impl Validate for UnsignedInteger {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            usize::try_from(
                gix_config::Integer::try_from(value)?
                    .to_decimal()
                    .ok_or_else(|| format!("integer {value} cannot be represented as `usize`"))?,
            )
            .map_err(|_| "cannot use sign for unsigned integer")?;
            Ok(())
        }
    }

    /// Assure that values that parse as git booleans are valid.
    #[derive(Default)]
    pub struct Boolean;

    impl Validate for Boolean {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
            gix_config::Boolean::try_from(value)?;
            Ok(())
        }
    }

    /// Values that are git remotes, symbolic or urls
    #[derive(Default)]
    pub struct RemoteName;
    impl Validate for RemoteName {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            remote::Name::try_from(Cow::Borrowed(value))
                .map_err(|_| format!("Illformed UTF-8 in remote name: \"{}\"", value.to_str_lossy()))?;
            Ok(())
        }
    }

    /// Values that are programs - everything is allowed.
    #[derive(Default)]
    pub struct Program;
    impl Validate for Program {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(())
        }
    }

    /// Values that are programs executables, everything is allowed.
    #[derive(Default)]
    pub struct Executable;
    impl Validate for Executable {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(())
        }
    }

    /// Values that parse as URLs.
    #[derive(Default)]
    pub struct Url;
    impl Validate for Url {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            gix_url::parse(value)?;
            Ok(())
        }
    }

    /// Values that parse as ref-specs for pushing.
    #[derive(Default)]
    pub struct PushRefSpec;
    impl Validate for PushRefSpec {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            gix_refspec::parse(value, gix_refspec::parse::Operation::Push)?;
            Ok(())
        }
    }

    /// Values that parse as ref-specs for pushing.
    #[derive(Default)]
    pub struct FetchRefSpec;
    impl Validate for FetchRefSpec {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            gix_refspec::parse(value, gix_refspec::parse::Operation::Fetch)?;
            Ok(())
        }
    }

    /// Timeouts used for file locks.
    pub struct LockTimeout;
    impl Validate for LockTimeout {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let value = gix_config::Integer::try_from(value)?
                .to_decimal()
                .ok_or_else(|| format!("integer {value} cannot be represented as integer"));
            super::super::Core::FILES_REF_LOCK_TIMEOUT.try_into_lock_timeout(Ok(value?))?;
            Ok(())
        }
    }

    /// Durations in milliseconds.
    pub struct DurationInMilliseconds;
    impl Validate for DurationInMilliseconds {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let value = gix_config::Integer::try_from(value)?
                .to_decimal()
                .ok_or_else(|| format!("integer {value} cannot be represented as integer"));
            super::super::gitoxide::Http::CONNECT_TIMEOUT.try_into_duration(Ok(value?))?;
            Ok(())
        }
    }

    /// A UTF-8 string.
    pub struct String;
    impl Validate for String {
        fn validate(&self, value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            value.to_str()?;
            Ok(())
        }
    }

    /// Any path - everything is allowed.
    pub struct Path;
    impl Validate for Path {
        fn validate(&self, _value: &BStr) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            Ok(())
        }
    }
}
