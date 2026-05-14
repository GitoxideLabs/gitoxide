#![no_main]

use bstr::ByteSlice;
use gix_glob::pattern::Case;
use gix_ignore::{
    Search,
    search::{Ignore, pattern_idx_matching_relative_path},
};
use libfuzzer_sys::fuzz_target;
use std::{borrow::Cow, hint::black_box};

// Keep fuzz-generated match paths small enough that pathological glob patterns don't dominate fuzzing time.
// We don't mitigate this in gix-glob as memoization made typical matches slower, and we want to stay on par with Git.
const MAX_FUZZ_PATH_LEN: usize = 256;
const MAX_FUZZ_IGNORE_BYTES: usize = 1024;
const MAX_FUZZ_PATTERN_BYTES: usize = 64;
const MAX_FUZZ_PATTERNS: usize = 16;
const MAX_FUZZ_OVERRIDE_PATTERNS: usize = 8;
const MAX_FUZZ_PATTERN_WILDCARDS: usize = 8;

fn relative_path(input: &[u8]) -> Option<&bstr::BStr> {
    let path = &input[input.iter().position(|b| *b != b'/').unwrap_or(input.len())..];
    let path = path.as_bstr();
    if path.len() > MAX_FUZZ_PATH_LEN {
        None
    } else if path.is_empty() {
        Some("fuzz".into())
    } else {
        Some(path)
    }
}

/// Bound ignore-pattern input from the fuzzer so pathological patterns can't dominate execution time.
///
/// Sanitization only removes bytes:
///
/// - Total ignore-pattern input is truncated to `MAX_FUZZ_IGNORE_BYTES`.
/// - Pattern lines after `MAX_FUZZ_PATTERNS` are dropped.
/// - Bytes after `MAX_FUZZ_PATTERN_BYTES` in each pattern line are dropped.
/// - `*` wildcards after `MAX_FUZZ_PATTERN_WILDCARDS` in each pattern line are dropped.
///
/// Unchanged input is borrowed; sanitized input is returned as an owned copy.
fn sanitized_ignore_input(input: &[u8]) -> Cow<'_, [u8]> {
    let original_len = input.len();
    let input = if input.len() > MAX_FUZZ_IGNORE_BYTES {
        &input[..MAX_FUZZ_IGNORE_BYTES]
    } else {
        input
    };
    let mut out = Vec::with_capacity(input.len());

    for (idx, line) in input.split_inclusive(|b| *b == b'\n').enumerate() {
        if idx == MAX_FUZZ_PATTERNS {
            break;
        }

        let (line, newline) = line.strip_suffix(b"\n").map_or((line, false), |line| (line, true));
        let mut wildcards = 0;

        for (idx, byte) in line.iter().copied().enumerate() {
            if idx == MAX_FUZZ_PATTERN_BYTES {
                break;
            }
            if byte == b'*' {
                if wildcards == MAX_FUZZ_PATTERN_WILDCARDS {
                    continue;
                }
                wildcards += 1;
            }
            out.push(byte);
        }

        if newline && out.len() != MAX_FUZZ_IGNORE_BYTES {
            out.push(b'\n');
        }
    }

    if out.len() == original_len {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(out)
    }
}

fn fuzz(input: &[u8]) {
    let support_precious = input.first().is_some_and(|b| b & 1 != 0);
    let ignore = Ignore { support_precious };
    let sane_input = sanitized_ignore_input(input);

    for (pattern, line_no, kind) in gix_ignore::parse(&sane_input, support_precious) {
        _ = black_box(pattern.to_string());
        _ = black_box(line_no);
        _ = black_box(kind);
    }

    let mut search = Search::default();
    search.add_patterns_buffer(&sane_input, "fuzz.gitignore", None, ignore);

    let overrides: Vec<String> = sane_input
        .split(|b| *b == 0 || *b == b'\n')
        .filter(|segment| !segment.is_empty())
        .take(MAX_FUZZ_OVERRIDE_PATTERNS)
        .map(|segment| String::from_utf8_lossy(segment).into_owned())
        .collect();
    let overrides_search = Search::from_overrides(overrides.iter().map(|s| s.as_str()), ignore);

    for path in [
        b"target".as_slice(),
        b"target/keep.me".as_slice(),
        b"dir/file.txt".as_slice(),
        input,
    ] {
        let Some(path) = relative_path(path) else {
            continue;
        };
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
