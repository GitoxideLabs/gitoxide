//! Histogram computation on pre-interned lines and postprocessing of shifted groups.
//! Run `cargo bench -p gix-diff --bench histogram` or add `-- --test` for a correctness check.
//! To compare revisions, apply the same benchmark code to both and set `CRITERION_HOME` to the same
//! absolute directory. Keep separate Cargo target directories for each revision.
//! Run with `-- --save-baseline main` on the baseline and `-- --baseline main` on the comparison.

use std::{fmt::Write, hint::black_box, time::Duration};

use criterion::{
    BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main,
    measurement::WallTime,
};
use gix_diff::blob::{Algorithm, Diff, InternedInput};

fn histogram(c: &mut Criterion) {
    let mut group = c.benchmark_group("histogram");
    group
        .warm_up_time(Duration::from_secs(1))
        .measurement_time(Duration::from_secs(2))
        .sample_size(30);

    benchmark(
        &mut group,
        BenchmarkId::new("source-like", "2048-functions"),
        source_like(2_048),
        (128, 192),
    );
    benchmark(
        &mut group,
        BenchmarkId::new("sparse-edits-no-shared-tail", 8_192),
        separated_replacements(8_192, 16),
        (16, 16),
    );
    benchmark(
        &mut group,
        BenchmarkId::new("dense-edits-no-shared-tail", 4_000),
        separated_replacements(4_000, 2_000),
        (2_000, 2_000),
    );

    // Each suffix token can seed a match, but the separated edits offer rarer anchors.
    for (case, edits, pairs, old_end, new_end) in [
        ("paired-tail", 250, 62_500, "", ""),
        ("paired-tail", 500, 125_000, "", ""),
        (
            "paired-tail-changed-end",
            128,
            25_000,
            "old final line\n",
            "new final line\n",
        ),
        ("paired-tail-appended-line", 128, 25_000, "", "new final line\n"),
    ] {
        let (mut before, mut after) = separated_replacements(edits * 2, edits);
        let mut tail = String::new();
        for pair in 0..pairs {
            writeln!(tail, "pair {pair}\npair {pair}").expect("writing to a string cannot fail");
        }
        before.push_str(&tail);
        after.push_str(&tail);
        before.push_str(old_end);
        after.push_str(new_end);
        benchmark(
            &mut group,
            BenchmarkId::new(case, pairs * 2),
            (before, after),
            (
                edits + u32::from(!old_end.is_empty()),
                edits + u32::from(!new_end.is_empty()),
            ),
        );
    }

    // The alternating lines exceed the seed cutoff. The blanks in the mixed tail can seed a match.
    for (case, tail) in [
        ("alternating-tail", "x\ny\n".repeat(125_000)),
        ("mixed-tail", "x\n".repeat(249_998) + "\n\n"),
    ] {
        let (mut before, mut after) = separated_replacements(1_000, 500);
        before.push_str(&tail);
        after.push_str(&tail);
        benchmark(&mut group, BenchmarkId::new(case, 250_000), (before, after), (500, 500));
    }

    let (mut before, mut after) = separated_replacements(128, 64);
    let tail_lines = 25_000;
    // The suffix appears unique when considered alone. Each token also occurs in the old prefix.
    for line in (0..tail_lines).rev() {
        writeln!(before, "tail {line}").expect("writing to a string cannot fail");
    }
    before.push_str("old fence\n");
    after.push_str("new fence\n");
    let start = after.len();
    for line in 0..tail_lines {
        writeln!(after, "tail {line}").expect("writing to a string cannot fail");
    }
    before.push_str(&after[start..]);
    benchmark(
        &mut group,
        BenchmarkId::new("overlapping-tail", tail_lines),
        (before, after),
        (tail_lines + 65, 65),
    );
    postprocess_shifted_groups(&mut group);
    group.finish();
}

fn postprocess_shifted_groups(group: &mut BenchmarkGroup<'_, WallTime>) {
    const LARGE_GROUP_LINES: u32 = 100_000;
    const SMALL_GROUPS: u32 = 1_000;
    let (mut before, mut after) = (String::new(), String::new());
    for block in 0..=SMALL_GROUPS {
        let lines = if block == 0 { LARGE_GROUP_LINES } else { 1 };
        for line in 0..lines {
            writeln!(before, "old {block}/{line}").expect("writing to a string cannot fail");
            writeln!(after, "new {block}/{line}").expect("writing to a string cannot fail");
        }
        // The duplicate line lets the replacement slide, causing a re-diff before the guard.
        writeln!(before, "x {block}\nx {block}\nguard {block}").expect("writing to a string cannot fail");
        writeln!(after, "x {block}\ny {block}\nguard {block}").expect("writing to a string cannot fail");
    }
    let input = InternedInput::new(before.as_str(), after.as_str());
    let fresh_diff = || Diff::compute(Algorithm::Histogram, &input);
    let mut diff = fresh_diff();
    diff.postprocess_no_heuristic(&input);
    let edits = LARGE_GROUP_LINES + 1 + SMALL_GROUPS * 2;
    check_diff(&diff, &input, (edits, edits));

    group.throughput(Throughput::Elements((input.before.len() + input.after.len()) as u64));
    group.bench_function("postprocess-shifted-groups", |b| {
        b.iter_batched_ref(
            fresh_diff,
            |diff| {
                diff.postprocess_no_heuristic(black_box(&input));
                black_box(&*diff);
            },
            BatchSize::PerIteration,
        );
    });
}

fn benchmark(
    group: &mut BenchmarkGroup<'_, WallTime>,
    id: BenchmarkId,
    (before, after): (String, String),
    expected_edits: (u32, u32),
) {
    let input = InternedInput::new(before.as_str(), after.as_str());
    let num_tokens = input.interner.num_tokens();
    let mut diff = Diff::compute(Algorithm::Histogram, &input);
    check_diff(&diff, &input, expected_edits);
    group.throughput(Throughput::Elements((input.before.len() + input.after.len()) as u64));
    group.bench_function(id, |b| {
        b.iter(|| {
            diff.compute_with(
                Algorithm::Histogram,
                black_box(&input.before),
                black_box(&input.after),
                num_tokens,
            );
            black_box(&diff);
        });
    });
    check_diff(&diff, &input, expected_edits);
}

fn check_diff(diff: &Diff, input: &InternedInput<&str>, expected_edits: (u32, u32)) {
    assert_eq!(
        (diff.count_removals(), diff.count_additions()),
        expected_edits,
        "edit counts must match the generated edits"
    );
    let (mut before_end, mut after_end) = (0, 0);
    for hunk in diff.hunks() {
        assert!(
            input.before[before_end..hunk.before.start as usize] == input.after[after_end..hunk.after.start as usize],
            "the unchanged lines before each hunk must match"
        );
        before_end = hunk.before.end as usize;
        after_end = hunk.after.end as usize;
    }
    assert!(
        input.before[before_end..] == input.after[after_end..],
        "the unchanged lines after the final hunk must match"
    );
}

fn source_like(functions: usize) -> (String, String) {
    let mut before = String::new();
    let mut after = String::new();
    for id in 0..functions {
        for (file, updated) in [(&mut before, false), (&mut after, true)] {
            writeln!(file, "fn normalize_{id}(value: u64) -> u64 {{").expect("writing to a string cannot fail");
            file.push_str("    let clamped = value.max(1);\n");
            let add = if updated && id % 16 == 8 {
                "saturating_add"
            } else {
                "wrapping_add"
            };
            writeln!(file, "    let adjusted = clamped.{add}({id});").expect("writing to a string cannot fail");
            if updated && id % 32 == 8 {
                file.push_str("    let adjusted = adjusted / 2;\n");
            }
            file.push_str("    adjusted ^ value\n}\n\n");
        }
    }
    (before, after)
}

fn separated_replacements(lines: u32, edits: u32) -> (String, String) {
    let mut before = String::new();
    let mut after = String::new();
    let stride = lines / edits;
    for line in 1..=lines {
        let (old, new) = if line % stride == 0 {
            ("old", "new")
        } else {
            ("anchor", "anchor")
        };
        writeln!(before, "{old} {line}").expect("writing to a string cannot fail");
        writeln!(after, "{new} {line}").expect("writing to a string cannot fail");
    }
    (before, after)
}

criterion_group!(benches, histogram);
criterion_main!(benches);
