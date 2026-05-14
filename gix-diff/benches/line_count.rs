use std::{fmt::Write, hint::black_box};

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

struct BenchmarkTokenSource {
    number_of_lines: u32,
    skip_every: u32,
}

impl BenchmarkTokenSource {
    fn new(number_of_lines: u32, skip_every: u32) -> Self {
        Self {
            number_of_lines,
            skip_every,
        }
    }
}

struct BenchmarkTokenizer {
    number_of_lines: u32,
    skip_every: u32,
    current: u32,
}

impl BenchmarkTokenizer {
    fn new(number_of_lines: u32, skip_every: u32) -> Self {
        Self {
            number_of_lines,
            skip_every,
            current: 0,
        }
    }
}

impl Iterator for BenchmarkTokenizer {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.number_of_lines {
            let item = self.current.to_string();

            self.current += 1;

            if self.current % self.skip_every == 0 {
                self.current += 1;
            }

            Some(item)
        } else {
            None
        }
    }
}

impl imara_diff::TokenSource for BenchmarkTokenSource {
    type Token = String;

    type Tokenizer = BenchmarkTokenizer;

    fn tokenize(&self) -> Self::Tokenizer {
        BenchmarkTokenizer::new(self.number_of_lines, self.skip_every)
    }

    fn estimate_tokens(&self) -> u32 {
        self.number_of_lines
    }
}

fn count_lines(c: &mut Criterion) {
    let input = imara_diff::InternedInput::new(
        BenchmarkTokenSource::new(10_000, 5),
        BenchmarkTokenSource::new(10_000, 6),
    );

    c.bench_function("imara-diff (synthetic input)", |b| {
        b.iter(|| {
            let diff = gix_diff::blob::Diff::compute(gix_diff::blob::Algorithm::Histogram, &input);

            assert_eq!(diff.count_additions(), 1666);
            assert_eq!(diff.count_removals(), 1333);
        });
    });
}

fn slider_postprocess(c: &mut Criterion) {
    let (before, after) = rust_like_fixture(2_000);
    let input = imara_diff::InternedInput::new(before.as_str(), after.as_str());

    let baseline = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
    let expected_additions = baseline.count_additions();
    let expected_removals = baseline.count_removals();

    let mut group = c.benchmark_group("slider-postprocess");
    group.bench_function("histogram-only", |b| {
        b.iter(|| {
            let diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);

            assert_eq!(diff.count_additions(), expected_additions);
            assert_eq!(diff.count_removals(), expected_removals);

            black_box(diff);
        });
    });
    group.bench_function("histogram+git-slider-postprocess", |b| {
        b.iter(|| {
            let mut diff = imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input);
            diff.postprocess_lines(&input);

            assert_eq!(diff.count_additions(), expected_additions);
            assert_eq!(diff.count_removals(), expected_removals);

            black_box(diff);
        });
    });
    group.bench_function("git-slider-postprocess-only", |b| {
        b.iter_batched(
            || imara_diff::Diff::compute(imara_diff::Algorithm::Histogram, &input),
            |mut diff| {
                diff.postprocess_lines(&input);

                assert_eq!(diff.count_additions(), expected_additions);
                assert_eq!(diff.count_removals(), expected_removals);

                black_box(diff);
            },
            BatchSize::SmallInput,
        );
    });
    group.finish();
}

fn rust_like_fixture(functions: usize) -> (String, String) {
    let mut before = String::new();
    let mut after = String::new();

    for idx in 0..functions {
        push_function(&mut before, idx, false);
        push_function(&mut after, idx, true);
    }

    (before, after)
}

fn push_function(buf: &mut String, idx: usize, with_extra_logging: bool) {
    writeln!(buf, "fn section_{idx}() {{").unwrap();
    writeln!(buf, "    let mut value = {idx};").unwrap();
    buf.push_str("    if value % 3 == 0 {\n");
    buf.push_str("        println!(\"triple: {}\", value);\n");
    if with_extra_logging && idx % 3 == 0 {
        buf.push_str("        println!(\"slider: {}\", value + 1);\n");
    }
    buf.push_str("    } else {\n");
    buf.push_str("        println!(\"plain: {}\", value);\n");
    if with_extra_logging && idx % 5 == 0 {
        buf.push_str("        println!(\"trace: {}\", value.saturating_sub(1));\n");
    }
    buf.push_str("    }\n");
    buf.push_str("}\n\n");
}

criterion_group!(benches, count_lines, slider_postprocess);
criterion_main!(benches);
