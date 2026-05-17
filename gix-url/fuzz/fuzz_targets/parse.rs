#![no_main]
use anyhow::Result;
use bstr::ByteSlice;
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;
use std::path::Path;

fn fuzz(data: &[u8]) -> Result<()> {
    let url = gix_url::parse(data.as_bstr())?;
    _ = black_box(url.user());
    _ = black_box(url.password());
    _ = black_box(url.password());
    if let Some(safe_host) = black_box(url.host_argument_safe()) {
        // Ensure malicious host paths can't be returned see;
        // https://secure.phabricator.com/T12961
        assert!(!safe_host.starts_with("ssh://-"));
    }
    _ = black_box(url.path_argument_safe());
    _ = black_box(url.path_is_root());
    _ = black_box(url.port_or_default());
    _ = black_box(url.canonicalized(Path::new("/cwd")));
    _ = black_box(url.to_bstring());

    _ = black_box(gix_url::expand_path::parse(data.as_bstr()));
    Ok(())
}

fuzz_target!(|data: &[u8]| {
    _ = black_box(fuzz(data));
});
