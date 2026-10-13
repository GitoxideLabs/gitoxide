#[gix_macros::unstable]
pub fn missing() {}

#[gix_macros::unstable(reason = "  ")]
pub fn empty() {}

#[gix_macros::unstable(why = "not the supported key")]
pub fn wrong_key() {}

#[gix_macros::unstable_fields]
pub struct EmptyField {
    #[unstable(reason = "")]
    pub field: usize,
}

gix_macros::unstable_module! {
    reason = "";
    pub mod missing;
}

fn main() {}
