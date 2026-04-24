#![no_main]

use gix_transport::client::Capabilities;
use gix_transport_fuzz::inspect_capabilities;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fuzz_target!(|input: &[u8]| {
    match Capabilities::from_bytes(input) {
        Ok((caps, delimiter_pos)) => {
            _ = black_box(delimiter_pos);
            inspect_capabilities(&caps);
        }
        Err(err) => {
            _ = black_box(err);
        }
    }
});
