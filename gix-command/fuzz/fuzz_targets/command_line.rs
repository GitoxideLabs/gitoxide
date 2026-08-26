#![no_main]

use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fuzz_target!(|input: &[u8]| {
    if let Ok(parsed) = gix_command::parse::command_line(input.into()) {
        assert!(parsed.env.iter().all(|(name, _)| {
            let mut bytes = name.bytes();
            bytes.next().is_some_and(|b| b == b'_' || b.is_ascii_alphabetic())
                && bytes.all(|b| b == b'_' || b.is_ascii_alphanumeric())
        }));

        _ = black_box(parsed);
    }
});
