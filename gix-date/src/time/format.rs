use crate::{
    Time,
    time::{CustomFormat, Format},
};

/// E.g. `2018-12-24`
pub const SHORT: CustomFormat = CustomFormat("%Y-%m-%d");

/// E.g. `Thu, 18 Aug 2022 12:45:06 +0800`
pub const RFC2822: CustomFormat = CustomFormat("%a, %d %b %Y %H:%M:%S %z");

/// E.g. `Thu, 8 Aug 2022 12:45:06 +0800`. This is output by `git log --pretty=%aD`.
pub const GIT_RFC2822: CustomFormat = CustomFormat("%a, %-d %b %Y %H:%M:%S %z");

/// E.g. `2022-08-17 22:04:58 +0200`
pub const ISO8601: CustomFormat = CustomFormat("%Y-%m-%d %H:%M:%S %z");

/// E.g. `2022-08-17T21:43:13+08:00`
pub const ISO8601_STRICT: CustomFormat = CustomFormat("%Y-%m-%dT%H:%M:%S%:z");

/// E.g. `123456789`
pub const UNIX: Format = Format::Unix;

/// E.g. `1660874655 +0800`
pub const RAW: Format = Format::Raw;

/// E.g. `Thu Sep 04 2022 10:45:06 -0400`, like the git `DEFAULT`, but with the year and time fields swapped.
pub const GITOXIDE: CustomFormat = CustomFormat("%a %b %d %Y %H:%M:%S %z");

/// E.g. `Thu Sep 4 10:45:06 2022 -0400`. This is output by `git log --pretty=%ad`.
pub const DEFAULT: CustomFormat = CustomFormat("%a %b %-d %H:%M:%S %Y %z");

/// Formatting
impl Time {
    /// Format this instance according to the given `format`.
    ///
    /// Use [`Format::Unix`], [`Format::Raw`] or one of the custom formats
    /// defined in the [`format`](mod@crate::time::format) submodule.
    ///
    /// Note that this can fail if the timezone isn't valid and the format requires a conversion to [`jiff::Zoned`].
    pub fn format(&self, format: impl Into<Format>) -> Result<String, jiff::Error> {
        self.format_inner(format.into())
    }

    /// Like [`Self::format()`], but on time conversion error, produce the [UNIX] format instead
    /// to make it infallible.
    pub fn format_or_unix(&self, format: impl Into<Format>) -> String {
        self.format_inner(format.into())
            .unwrap_or_else(|_| self.seconds.to_string())
    }

    fn format_inner(&self, format: Format) -> Result<String, jiff::Error> {
        Ok(match format {
            Format::Custom(CustomFormat(format)) => self.to_zoned()?.strftime(format).to_string(),
            Format::Unix => self.seconds.to_string(),
            Format::Raw => self.to_string(),
        })
    }
}

impl Time {
    /// Produce a `Zoned` time for complex time computations and limitless formatting.
    pub fn to_zoned(self) -> Result<jiff::Zoned, jiff::Error> {
        let offset = jiff::tz::Offset::from_seconds(self.offset)?;
        Ok(jiff::Timestamp::from_second(self.seconds)?.to_zoned(offset.to_time_zone()))
    }
}
