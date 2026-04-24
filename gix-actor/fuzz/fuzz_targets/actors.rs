#![no_main]

use gix_actor::{IdentityRef, Signature, SignatureRef};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn inspect_identity(identity: IdentityRef<'_>) {
    _ = black_box(identity.trim());
    _ = black_box(identity.to_owned());
    let mut out = Vec::new();
    _ = black_box(identity.write_to(&mut out));
    _ = black_box(out);
}

fn inspect_signature(signature: SignatureRef<'_>) {
    _ = black_box(signature.trim());
    _ = black_box(signature.actor());
    _ = black_box(signature.seconds());
    _ = black_box(signature.time());
    _ = black_box(signature.to_owned());
    let mut out = Vec::new();
    _ = black_box(signature.write_to(&mut out));
    _ = black_box(out);
}

fuzz_target!(|input: &[u8]| {
    if let Ok(identity) = IdentityRef::from_bytes::<()>(input) {
        inspect_identity(identity);
    }
    if let Ok(signature) = SignatureRef::from_bytes::<()>(input) {
        inspect_signature(signature);
    }
});
