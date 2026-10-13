#![allow(unexpected_cfgs, unused_macros)]

macro_rules! methods {
    () => {
        pub fn generated() {}
    };
}

pub struct Handle;

#[gix_macros::unstable(since = "2026-10-13")]
impl Handle {
    methods!();
}

fn main() {}
