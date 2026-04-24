#![no_main]

use gix_actor::SignatureRef;
use gix_mailmap::{Entry, Snapshot};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn inspect_entry(entry: Entry<'_>) {
    _ = black_box(entry.new_name());
    _ = black_box(entry.new_email());
    _ = black_box(entry.old_name());
    _ = black_box(entry.old_email());
}

fn inspect_snapshot(snapshot: &Snapshot) {
    for entry in snapshot.iter().take(64) {
        inspect_entry(entry);
    }
    _ = black_box(snapshot.entries());
}

fn resolve(snapshot: &Snapshot, input: &[u8]) {
    for candidate in [
        input,
        b"Joe <bugs@example.com> 1 +0000".as_slice(),
        b"Jane <bugs@example.com> 1 +0000".as_slice(),
        b"Jane <jane@desktop.(none)> 1 +0000".as_slice(),
        b"Unknown <unknown@example.com> 1 +0000".as_slice(),
    ] {
        let Ok(signature) = SignatureRef::from_bytes::<()>(candidate) else {
            continue;
        };
        _ = black_box(snapshot.try_resolve_ref(signature));
        _ = black_box(snapshot.try_resolve(signature));
        _ = black_box(snapshot.resolve(signature));
        _ = black_box(snapshot.resolve_cow(signature));
    }
}

fn fuzz(input: &[u8]) {
    let mut parsed = Vec::new();
    for result in gix_mailmap::parse(input).take(64) {
        match result {
            Ok(entry) => {
                inspect_entry(entry);
                parsed.push(entry);
            }
            Err(err) => {
                _ = black_box(err);
            }
        }
    }

    let parsed_snapshot = Snapshot::new(parsed.iter().copied());
    inspect_snapshot(&parsed_snapshot);
    resolve(&parsed_snapshot, input);

    let bytes_snapshot = Snapshot::from_bytes(input);
    inspect_snapshot(&bytes_snapshot);
    resolve(&bytes_snapshot, input);
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
