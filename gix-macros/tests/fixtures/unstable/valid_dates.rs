#![allow(unexpected_cfgs)]

#[gix_macros::unstable(since = "2000-02-29")]
pub fn leap_century() {}

#[gix_macros::unstable(since = "2024-02-29",)]
pub fn leap_year() {}

#[gix_macros::unstable(since = "0001-01-01")]
pub fn first_year() {}

#[gix_macros::unstable(since = "9999-12-31")]
pub fn last_year() {}

fn main() {}
