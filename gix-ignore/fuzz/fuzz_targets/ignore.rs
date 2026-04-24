#![no_main]

use bstr::ByteSlice;
use gix_glob::pattern::Case;
use gix_ignore::{
    search::{pattern_idx_matching_relative_path, Ignore},
    Search,
};
use libfuzzer_sys::fuzz_target;
use std::hint::black_box;

fn relative_path(input: &[u8]) -> &bstr::BStr {
    let path = &input[input.iter().position(|b| *b != b'/').unwrap_or(input.len())..];
    let path = path.as_bstr();
    if path.is_empty() { "fuzz".into() } else { path }
}

fn fuzz(input: &[u8]) {
    let support_precious = input.first().is_some_and(|b| b & 1 != 0);
    let ignore = Ignore { support_precious };

    for (pattern, line_no, kind) in gix_ignore::parse(input, support_precious).take(16) {
        _ = black_box(pattern.to_string());
        _ = black_box(line_no);
        _ = black_box(kind);
    }

    let mut search = Search::default();
    search.add_patterns_buffer(input, "fuzz.gitignore", None, ignore);

    let overrides: Vec<String> = input
        .split(|b| *b == 0 || *b == b'\n')
        .filter(|segment| !segment.is_empty())
        .take(8)
        .map(|segment| String::from_utf8_lossy(segment).into_owned())
        .collect();
    let overrides_search = Search::from_overrides(overrides.iter().map(|s| s.as_str()), ignore);

    for path in [
        b"target".as_slice(),
        b"target/keep.me".as_slice(),
        b"dir/file.txt".as_slice(),
        input,
    ] {
        let path = relative_path(path);
        _ = black_box(search.pattern_matching_relative_path(path, Some(false), Case::Sensitive));
        _ = black_box(search.pattern_matching_relative_path(path, Some(true), Case::Fold));
        _ = black_box(overrides_search.pattern_matching_relative_path(path, Some(false), Case::Sensitive));

        if let Some(list) = search.patterns.first() {
            let basename_pos = path.rfind_byte(b'/').map(|pos| pos + 1);
            _ = black_box(gix_ignore::search::pattern_matching_relative_path(
                list,
                path,
                basename_pos,
                Some(false),
                Case::Sensitive,
            ));
            _ = black_box(pattern_idx_matching_relative_path(
                list,
                path,
                basename_pos,
                Some(false),
                Case::Sensitive,
            ));
        }
    }
}

fuzz_target!(|input: &[u8]| {
    fuzz(input);
});
