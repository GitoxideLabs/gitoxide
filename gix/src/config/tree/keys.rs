use std::fmt::{Debug, Formatter};

use gix_config::KeyRef;
use gix_error::ResultExt;

use crate::{
    ExnResult, Result,
    bstr::{BStr, ByteSlice},
    config,
    config::tree::{Key, Link, Note, Section, SubSectionRequirement},
};

/// Implements a value without any constraints, i.e. a any value.
#[derive(Copy, Clone)]
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
    /// The value to use if this key is unset.
    pub default_value: Option<&'static [u8]>,
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
            default_value: None,
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

    /// Record another key as fallback if this key is not set.
    ///
    /// This is descriptive metadata; consumers must apply the fallback during value resolution.
    pub const fn with_fallback(mut self, key: &'static dyn Key) -> Self {
        self.link = Some(Link::FallbackKey(key));
        self
    }

    /// Set the value to use if this key is unset.
    pub const fn with_default(mut self, value: &'static [u8]) -> Self {
        self.default_value = Some(value);
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
        value: impl gix_utils::AsBStr,
        op: gix_refspec::parse::Operation,
    ) -> Result<gix_refspec::RefSpec> {
        let value = value.as_bstr();
        Ok(gix_refspec::parse(value.as_bstr(), op)
            .map(|spec| spec.to_owned())
            .or_raise(|| config::key::error_with_value(self, "Could not parse refspec", value))?)
    }

    /// Try to interpret `value` as UTF-8 encoded string.
    pub fn try_into_string(&'static self, value: impl gix_utils::AsBStr) -> Result<std::string::String> {
        let value = value.as_bstr();
        Ok(value
            .to_str()
            .or_raise(|| config::key::error_with_value(self, "Could not decode UTF-8 string", value))?
            .to_owned())
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

    fn validate(&self, value: &BStr) -> Result<()> {
        self.validate
            .validate(value)
            .or_raise(|| config::key::error_with_value(self, "Invalid configuration value", value))?;
        Ok(())
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

    fn default_value(&self) -> Option<&BStr> {
        self.default_value.map(BStr::new)
    }
}

impl<T: Validate + Copy + Clone> gix_config::AsKey for Any<T> {
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

/// The `core.compression`, `core.looseCompression` and `pack.compression` keys to validate compression values.
pub type Compression = Any<validate::Compression>;

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

    use gix_error::ResultExt;

    use crate::{
        ExnMessageResult, Result, config,
        config::tree::{Section, keys::DurationInMilliseconds},
    };

    impl DurationInMilliseconds {
        /// Create a new instance.
        pub const fn new_duration(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::DurationInMilliseconds)
        }

        /// Return a valid duration as parsed from an integer that is interpreted as milliseconds.
        pub fn try_into_duration(
            &'static self,
            value: ExnMessageResult<Option<i64>>,
        ) -> Result<Option<std::time::Duration>> {
            let Some(value) = value.or_raise(|| config::key::error(self, "Invalid duration in milliseconds"))? else {
                return Ok(None);
            };
            Ok(Some(match value {
                val if val < 0 => Duration::from_secs(u64::MAX),
                val => Duration::from_millis(val.try_into().expect("i64 to u64 always works if positive")),
            }))
        }
    }
}

mod lock_timeout {
    use std::time::Duration;

    use gix_error::ResultExt;
    use gix_lock::acquire::Fail;

    use crate::{
        ExnMessageResult, Result, config,
        config::tree::{Section, keys::LockTimeout},
    };

    impl LockTimeout {
        /// Create a new instance.
        pub const fn new_lock_timeout(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::LockTimeout)
        }

        /// Return information on how long to wait for locked files.
        pub fn try_into_lock_timeout(
            &'static self,
            value: ExnMessageResult<Option<i64>>,
        ) -> Result<Option<gix_lock::acquire::Fail>> {
            let Some(value) = value.or_raise(|| config::key::error(self, "Invalid lock timeout"))? else {
                return Ok(None);
            };
            Ok(Some(match value {
                val if val < 0 => Fail::AfterDurationWithBackoff(Duration::from_secs(u64::MAX)),
                0 => Fail::Immediately,
                val => Fail::AfterDurationWithBackoff(Duration::from_millis(
                    val.try_into().expect("i64 to u64 always works if positive"),
                )),
            }))
        }
    }
}

mod compression {
    use gix_error::{ErrorExt, ResultExt};

    use crate::{
        ExnMessageResult, Result, config,
        config::tree::{Section, keys::Compression},
    };

    impl Compression {
        /// Create a new instance.
        pub const fn new_compression(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::Compression)
        }

        /// Convert `value` into a zlib compression level, where `-1` is mapped to the
        /// zlib default, just like `git` does.
        pub fn try_into_compression(
            &'static self,
            value: ExnMessageResult<Option<i64>>,
        ) -> Result<Option<gix_zlib::Compression>> {
            let Some(value) = value.or_raise(|| config::key::error(self, "Invalid compression level"))? else {
                return Ok(None);
            };
            match value {
                -1 => Ok(Some(gix_zlib::Compression::DEFAULT)),
                level => i32::try_from(level)
                    .ok()
                    .and_then(gix_zlib::Compression::new)
                    .map(Some)
                    .ok_or_else(|| {
                        config::key::error_with_value(self, "Invalid compression level", level)
                            .raise()
                            .into()
                    }),
            }
        }
    }
}

mod refspecs {
    use crate::config::tree::{
        Section,
        keys::{FetchRefSpec, PushRefSpec, validate},
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
    use gix_error::ResultExt;

    use crate::{
        Result,
        bstr::ByteSlice,
        config,
        config::tree::{
            Section,
            keys::{Url, validate},
        },
    };

    impl Url {
        /// Create a new instance.
        pub const fn new_url(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::Url)
        }

        /// Try to parse `value` as URL.
        pub fn try_into_url(&'static self, value: impl gix_utils::AsBStr) -> Result<gix_url::Url> {
            let value = value.as_bstr();
            Ok(gix_url::parse(value.as_bstr())
                .or_raise(|| config::key::error_with_value(self, "Could not parse URL", value))?)
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
    use gix_error::ResultExt;

    use crate::{
        ExnMessageResult, Result,
        config::key,
        config::tree::{Section, keys::UnsignedInteger},
    };

    impl UnsignedInteger {
        /// Create a new instance.
        pub const fn new_unsigned_integer(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::UnsignedInteger)
        }

        /// Convert `value` into a `usize`, attaching key metadata on failure.
        pub fn try_into_usize(&'static self, value: ExnMessageResult<Option<i64>>) -> Result<Option<usize>> {
            let value = value.or_raise(|| key::error(self, "Could not parse an unsigned integer"))?;
            Ok(value
                .map(|value| {
                    usize::try_from(value)
                        .or_raise(|| key::error_with_value(self, "Could not parse an unsigned integer", value))
                })
                .transpose()?)
        }

        /// Convert `value` into a `u64`, attaching key metadata on failure.
        pub fn try_into_u64(&'static self, value: ExnMessageResult<Option<i64>>) -> Result<Option<u64>> {
            let value = value.or_raise(|| key::error(self, "Could not parse an unsigned integer"))?;
            Ok(value
                .map(|value| {
                    u64::try_from(value)
                        .or_raise(|| key::error_with_value(self, "Could not parse an unsigned integer", value))
                })
                .transpose()?)
        }

        /// Convert `value` into a `u32`, attaching key metadata on failure.
        pub fn try_into_u32(&'static self, value: ExnMessageResult<Option<i64>>) -> Result<Option<u32>> {
            let value = value.or_raise(|| key::error(self, "Could not parse an unsigned integer"))?;
            Ok(value
                .map(|value| {
                    u32::try_from(value)
                        .or_raise(|| key::error_with_value(self, "Could not parse an unsigned integer", value))
                })
                .transpose()?)
        }
    }
}

mod time {
    use crate::{
        ExnMessageResult,
        bstr::ByteSlice,
        config::key,
        config::tree::{
            Section,
            keys::{Time, validate},
        },
    };
    use gix_error::ResultExt;

    impl Time {
        /// Create a new instance.
        pub const fn new_time(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, validate::Time)
        }

        /// Convert the `value` into a date if possible, with `now` as reference time for relative dates.
        /// Failures include the [key metadata](crate::config::tree::Key::validate) and the value bytes as `input`.
        /// Date parsing failures retain the cause documented by [`gix_date::parse()`].
        pub fn try_into_time(
            &self,
            value: impl gix_utils::AsBStr,
            now: Option<gix_date::Zoned>,
        ) -> ExnMessageResult<gix_date::Time> {
            let value = value.as_bstr();
            gix_date::parse(
                value
                    .as_bstr()
                    .to_str()
                    .or_raise(|| key::error_with_value(self, "Could not decode date as UTF-8", value))?,
                now,
            )
            .or_raise(|| key::error_with_value(self, "Could not parse date", value))
        }
    }
}

mod boolean {
    use gix_error::ResultExt;

    use crate::{
        ExnMessageResult, Result, config,
        config::tree::{
            Section,
            keys::{Boolean, validate},
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
        pub fn enrich_error(&'static self, value: ExnMessageResult<Option<bool>>) -> Result<Option<bool>> {
            Ok(value.or_raise(|| config::key::error(self, "Invalid boolean"))?)
        }
    }
}

mod remote_name {
    use gix_error::ResultExt;

    use crate::{
        Result,
        bstr::BString,
        config,
        config::tree::{Section, keys::RemoteName},
    };

    impl RemoteName {
        /// Create a new instance.
        pub const fn new_remote_name(name: &'static str, section: &'static dyn Section) -> Self {
            Self::new_with_validate(name, section, super::validate::RemoteName)
        }

        /// Try to validate `name` as symbolic remote name and return it.
        pub fn try_into_symbolic_name(&'static self, name: impl gix_utils::AsBStr) -> Result<BString> {
            let name = name.as_bstr();
            Ok(crate::remote::name::validated(name.to_owned())
                .or_raise(|| config::key::error_with_value(self, "Invalid remote name", name))?)
        }
    }
}

/// Provide a way to validate a value, or decode a value from `git-config`.
pub trait Validate {
    /// Validate `value` or return an error.
    /// Errors with input context include invalid value bytes as `input` [metadata](gix_error::Exn::metadata()).
    fn validate(&self, value: &BStr) -> ExnResult;
}

/// various implementations of the `Validate` trait.
pub mod validate {
    use std::borrow::Cow;

    use gix_error::{ErrorExt, ResultExt, message};

    use crate::{
        ExnResult,
        bstr::{BStr, ByteSlice},
        config::tree::keys::Validate,
        remote,
    };

    /// Everything is valid.
    #[derive(Default, Copy, Clone)]
    pub struct All;

    impl Validate for All {
        fn validate(&self, _value: &BStr) -> ExnResult {
            Ok(())
        }
    }

    /// Assure that values that parse as git dates are valid.
    #[derive(Default, Clone, Copy)]
    pub struct Time;

    impl Validate for Time {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_date::parse(value.to_str().or_erased()?, gix_date::Zoned::now().into()).or_erased()?;
            Ok(())
        }
    }

    /// Assure that values that parse as unsigned integers are valid.
    #[derive(Default, Clone, Copy)]
    pub struct UnsignedInteger;

    impl Validate for UnsignedInteger {
        fn validate(&self, value: &BStr) -> ExnResult {
            usize::try_from(
                gix_config::Integer::try_from(value)
                    .or_erased()?
                    .to_decimal()
                    .ok_or_else(|| message!("integer {value} cannot be represented as `usize`").raise_erased())?,
            )
            .or_raise_erased(|| gix_error::validation("unsigned integer is out of range").with("input", value))?;
            Ok(())
        }
    }

    /// Assure that values that parse as git booleans are valid.
    #[derive(Default, Clone, Copy)]
    pub struct Boolean;

    impl Validate for Boolean {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_config::Boolean::try_from(value).or_erased()?;
            Ok(())
        }
    }

    /// Values that are full reference names.
    #[derive(Default, Clone, Copy)]
    pub struct FullNameRef {
        allow_empty: bool,
    }

    impl FullNameRef {
        /// Create a validator that requires a full reference name.
        pub const fn new() -> Self {
            FullNameRef { allow_empty: false }
        }

        /// Create a validator that also accepts an empty value.
        pub const fn or_empty() -> Self {
            FullNameRef { allow_empty: true }
        }
    }

    impl Validate for FullNameRef {
        fn validate(&self, value: &BStr) -> ExnResult {
            if !self.allow_empty || !value.is_empty() {
                gix_ref::FullName::try_from(value.to_owned()).or_erased()?;
            }
            Ok(())
        }
    }

    /// Values that are git remotes, symbolic or urls
    #[derive(Default, Clone, Copy)]
    pub struct RemoteName;
    impl Validate for RemoteName {
        fn validate(&self, value: &BStr) -> ExnResult {
            remote::Name::try_from(Cow::Borrowed(value)).map_err(|invalid| {
                gix_error::validation("Illformed UTF-8 in remote name")
                    .with("input", invalid.into_owned())
                    .raise_erased()
            })?;
            Ok(())
        }
    }

    /// Values that are programs - everything is allowed.
    #[derive(Default, Clone, Copy)]
    pub struct Program;
    impl Validate for Program {
        fn validate(&self, _value: &BStr) -> ExnResult {
            Ok(())
        }
    }

    /// Values that are programs executables, everything is allowed.
    #[derive(Default, Clone, Copy)]
    pub struct Executable;
    impl Validate for Executable {
        fn validate(&self, _value: &BStr) -> ExnResult {
            Ok(())
        }
    }

    /// Values that parse as URLs.
    #[derive(Default, Clone, Copy)]
    pub struct Url;
    impl Validate for Url {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_url::parse(value).or_erased()?;
            Ok(())
        }
    }

    /// Values that parse as ref-specs for pushing.
    #[derive(Default, Clone, Copy)]
    pub struct PushRefSpec;
    impl Validate for PushRefSpec {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_refspec::parse(value, gix_refspec::parse::Operation::Push).or_erased()?;
            Ok(())
        }
    }

    /// Values that parse as ref-specs for pushing.
    #[derive(Default, Clone, Copy)]
    pub struct FetchRefSpec;
    impl Validate for FetchRefSpec {
        fn validate(&self, value: &BStr) -> ExnResult {
            gix_refspec::parse(value, gix_refspec::parse::Operation::Fetch).or_erased()?;
            Ok(())
        }
    }

    /// Timeouts used for file locks.
    #[derive(Clone, Copy)]
    pub struct LockTimeout;
    impl Validate for LockTimeout {
        fn validate(&self, value: &BStr) -> ExnResult {
            let value = gix_config::Integer::try_from(value)
                .or_erased()?
                .to_decimal()
                .ok_or_else(|| message!("integer {value} cannot be represented as integer").raise_erased())?;
            super::super::Core::FILES_REF_LOCK_TIMEOUT
                .try_into_lock_timeout(Ok(Some(value)))
                .or_erased()?;
            Ok(())
        }
    }

    /// A zlib compression level.
    #[derive(Clone, Copy)]
    pub struct Compression;
    impl Validate for Compression {
        fn validate(&self, value: &BStr) -> ExnResult {
            let value = gix_config::Integer::try_from(value)
                .or_erased()?
                .to_decimal()
                .ok_or_else(|| message!("integer {value} cannot be represented as integer").raise_erased())?;
            super::super::Core::COMPRESSION
                .try_into_compression(Ok(Some(value)))
                .or_erased()?;
            Ok(())
        }
    }

    /// Durations in milliseconds.
    #[derive(Clone, Copy)]
    pub struct DurationInMilliseconds;
    impl Validate for DurationInMilliseconds {
        fn validate(&self, value: &BStr) -> ExnResult {
            let value = gix_config::Integer::try_from(value)
                .or_erased()?
                .to_decimal()
                .ok_or_else(|| message!("integer {value} cannot be represented as integer").raise_erased())?;
            super::super::gitoxide::Http::CONNECT_TIMEOUT
                .try_into_duration(Ok(Some(value)))
                .or_erased()?;
            Ok(())
        }
    }

    /// A UTF-8 string.
    #[derive(Clone, Copy)]
    pub struct String;
    impl Validate for String {
        fn validate(&self, value: &BStr) -> ExnResult {
            value.to_str().or_erased()?;
            Ok(())
        }
    }

    /// Any path - everything is allowed.
    #[derive(Clone, Copy)]
    pub struct Path;
    impl Validate for Path {
        fn validate(&self, _value: &BStr) -> ExnResult {
            Ok(())
        }
    }
}
