#[gix_macros::unstable]
pub fn missing() {}

#[gix_macros::unstable()]
pub fn empty() {}

#[gix_macros::unstable(reason = "Only a reason")]
pub fn reason_only() {}

#[gix_macros::unstable(since = "2026-1-01")]
pub fn malformed() {}

#[gix_macros::unstable(since = "2026-02-29")]
pub fn non_leap() {}

#[gix_macros::unstable(since = "1900-02-29")]
pub fn non_leap_century() {}

#[gix_macros::unstable(since = "2026-04-31")]
pub fn impossible_day() {}

#[gix_macros::unstable(since = "0000-01-01")]
pub fn zero_year() {}

#[gix_macros::unstable(since = "2026-13-01")]
pub fn impossible_month() {}

#[gix_macros::unstable(since = "2026-10-00")]
pub fn zero_day() {}

#[gix_macros::unstable(since = "2026-10-13", since = "2026-10-14")]
pub fn duplicate_since() {}

#[gix_macros::unstable(since = "2026-10-13", reason = "One", reason = "Two")]
pub fn duplicate_reason() {}

#[gix_macros::unstable_fields]
pub struct MissingField {
    #[unstable]
    pub field: usize,
}

#[gix_macros::unstable_fields]
pub struct EmptyField {
    #[unstable()]
    pub field: usize,
}

#[gix_macros::unstable_fields]
pub struct InvalidField {
    #[unstable(since = "2026-02-30")]
    pub field: usize,
}

fn main() {}
