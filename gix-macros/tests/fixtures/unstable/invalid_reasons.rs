#[gix_macros::unstable(since = "2026-10-13", reason = "  ")]
pub fn empty() {}

#[gix_macros::unstable(why = "not the supported key")]
pub fn wrong_key() {}

#[gix_macros::unstable_fields]
pub struct EmptyField {
    #[unstable(since = "2026-10-13", reason = "")]
    pub field: usize,
}

#[gix_macros::unstable(since = "2026-10-13", reason = "")]
pub mod missing;

#[gix_macros::unstable_fields]
pub struct DuplicateField {
    #[unstable(since = "2026-10-13")]
    #[unstable(since = "2026-10-13", reason = "Another reason")]
    pub field: usize,
}

fn main() {}
