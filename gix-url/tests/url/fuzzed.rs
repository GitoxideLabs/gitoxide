use std::{path::Path, time::Duration};

use bstr::ByteSlice;

#[test]
fn fuzzed() {
    for name in [
        "illegal-utf8",
        "short-panic",
        "very-long-abort2",
        "very-long-abort",
        "very-long6",
        "very-long5",
        "very-long4",
        "very-long3",
        "very-long2",
        "very-long",
    ] {
        let base = Path::new("tests").join("fixtures").join("fuzzed");
        let location = base.join(Path::new(name).with_extension("url"));
        let url = std::fs::read(&location).unwrap();
        let start = std::time::Instant::now();
        gix_url::parse(url.as_bstr()).ok();
        assert!(
            start.elapsed() < Duration::from_millis(250),
            "URL at {location:?} parsed too slowly, took {:.00}s",
            start.elapsed().as_secs_f32()
        );
    }
}
