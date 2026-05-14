use std::{hint::black_box, time::Duration};

use bstr::{BStr, ByteSlice};
use criterion::{Criterion, criterion_group, criterion_main};
use gix_glob::{Pattern, pattern::Case, wildmatch::Mode};

fn basename_start_pos(value: &BStr) -> Option<usize> {
    value.rfind_byte(b'/').map(|pos| pos + 1)
}

fn matches(pattern: &Pattern, path: &BStr) -> bool {
    pattern.matches_repo_relative_path(
        path,
        basename_start_pos(path),
        Some(false),
        Case::Sensitive,
        Mode::NO_MATCH_SLASH_LITERAL,
    )
}

fn slash_rich_non_match(num_components: usize) -> Vec<u8> {
    let mut path = Vec::with_capacity(num_components * 4 + 5);
    for _ in 0..num_components {
        path.extend_from_slice(b"dir/");
    }
    path.extend_from_slice(b"onfug");
    path
}

fn wildmatch(c: &mut Criterion) {
    let mut group = c.benchmark_group("wildmatch");

    let literal = Pattern::from_bytes(b"target").expect("valid pattern");
    group.bench_function("literal shortcut", |b| {
        b.iter(|| assert!(black_box(matches(&literal, b"target".as_bstr()))));
    });

    let wildcard = Pattern::from_bytes(b"foo*bar").expect("valid pattern");
    group.bench_function("single star", |b| {
        b.iter(|| assert!(black_box(matches(&wildcard, b"foo123bar".as_bstr()))));
    });

    let globstar = Pattern::from_bytes(b"src/**/*.rs").expect("valid pattern");
    group.bench_function("globstar match", |b| {
        b.iter(|| assert!(black_box(matches(&globstar, b"src/a/b/c/lib.rs".as_bstr()))));
    });

    group.sample_size(10).measurement_time(Duration::from_secs(2));
    let pathological = Pattern::from_bytes(b"***/**/****/***/onfig").expect("valid pattern");
    let path = slash_rich_non_match(48);
    group.bench_function("globstar pathological non-match", |b| {
        b.iter(|| assert!(black_box(!matches(&pathological, path.as_bstr()))));
    });

    group.finish();
}

criterion_group!(benches, wildmatch);
criterion_main!(benches);
