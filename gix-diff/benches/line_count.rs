use criterion::{criterion_group, criterion_main, Criterion};

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

impl imara_diff::intern::TokenSource for BenchmarkTokenSource {
    type Token = String;

    type Tokenizer = BenchmarkTokenizer;

    fn tokenize(&self) -> Self::Tokenizer {
        BenchmarkTokenizer::new(self.number_of_lines, self.skip_every)
    }

    fn estimate_tokens(&self) -> u32 {
        self.number_of_lines
    }
}

impl imara_diff_v2::TokenSource for BenchmarkTokenSource {
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
    let input = imara_diff::intern::InternedInput::new(
        BenchmarkTokenSource::new(10_000, 5),
        BenchmarkTokenSource::new(10_000, 6),
    );

    let input_v2 = imara_diff_v2::InternedInput::new(
        BenchmarkTokenSource::new(10_000, 5),
        BenchmarkTokenSource::new(10_000, 6),
    );

    c.bench_function("imara-diff 0.1", |b| {
        b.iter(|| {
            let counters = gix_diff::blob::diff(
                gix_diff::blob::Algorithm::Histogram,
                &input,
                gix_diff::blob::sink::Counter::default(),
            );

            assert_eq!(counters.insertions, 1666);
            assert_eq!(counters.removals, 1333);
        });
    });
    c.bench_function("imara-diff 0.2", |b| {
        b.iter(|| {
            let diff = imara_diff_v2::Diff::compute(imara_diff_v2::Algorithm::Histogram, &input_v2);

            let additions = diff.count_additions();
            let removals = diff.count_removals();

            assert_eq!(additions, 1666);
            assert_eq!(removals, 1333);
        });
    });
}

criterion_group!(benches, count_lines);
criterion_main!(benches);
