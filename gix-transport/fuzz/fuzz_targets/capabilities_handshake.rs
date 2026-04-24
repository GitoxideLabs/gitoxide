#![no_main]

use gix_transport_fuzz::inspect_handshake;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    inspect_handshake(input);
});
