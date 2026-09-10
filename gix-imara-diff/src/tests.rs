use crate::util::sqrt;
use crate::{Algorithm, BasicLineDiffPrinter, Diff, InternedInput, UnifiedDiffConfig};
use expect_test::expect;

#[test]
fn myers_is_even() {
    let before = "a\nb\nx\nx\ny\n";
    let after = "b\na\nx\ny\nx\n";

    cov_mark::check!(EVEN_SPLIT);
    // if the check for is_odd incorrectly always true then we take a fastpath
    // when we shouldn't, which always leads to infinite iterations/recursion
    // still we check the number of iterations here in case the search
    // is buggy in more subtle ways
    cov_mark::check_count!(SPLIT_SEARCH_ITER, 15);
    let input = InternedInput::new(before, after);
    let diff = Diff::compute(Algorithm::Myers, &input);
    expect![[r#"
        @@ -1,5 +1,5 @@
        -a
         b
        -x
        +a
         x
         y
        +x
    "#]]
    .assert_eq(
        &diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string(),
    );
}

#[test]
fn myers_is_odd() {
    let before = "a\nb\nx\ny\nx\n";
    let after = "b\na\nx\ny\n";

    cov_mark::check!(ODD_SPLIT);
    // if the check for odd doesn't work then
    // we still find the correct result but the number of search
    // iterations increases
    cov_mark::check_count!(SPLIT_SEARCH_ITER, 9);
    let input = InternedInput::new(before, after);
    let diff = Diff::compute(Algorithm::Myers, &input);
    expect![[r#"
        @@ -1,5 +1,4 @@
        -a
         b
        +a
         x
         y
        -x
    "#]]
    .assert_eq(
        &diff
            .unified_diff(
                &BasicLineDiffPrinter(&input.interner),
                UnifiedDiffConfig::default(),
                &input,
            )
            .to_string(),
    );
}

/// The frequency limit above which a line becomes a candidate for discarding comes from git's
/// `xdl_bogosqrt`, which rounds the halved bit count up. Rounding down halves the limit for every
/// odd bit length, so lines git considers ordinary are treated as too frequent to match.
#[test]
fn the_frequency_limit_rounds_the_way_git_rounds() {
    // `xdl_bogosqrt`: for (i = 1; n > 0; n >>= 2) i <<= 1;
    fn bogosqrt(mut n: usize) -> u32 {
        let mut i = 1u32;
        while n > 0 {
            n >>= 2;
            i <<= 1;
        }
        i
    }

    // Test that sqrt matches git's bogosqrt for typical values
    // and values near the top of the `usize` range
    let mut inputs = vec![0, 1, 2, 3, 4, 16, 24, 69, 277, 345, 450, 1000, 4096, 65_535];
    if usize::BITS >= 4 {
        let hi = 1usize << (usize::BITS - 3);
        inputs.extend([hi - 1, hi]);
    }
    for n in inputs {
        assert_eq!(sqrt(n), bogosqrt(n), "at {n}");
    }

    // On 64-bit platforms we have to clamp to `u32::MAX`
    // when the input is >= `1 << 62` to avoid overflow.
    let edge_inputs = [usize::MAX, 1 << (usize::BITS - 1), 1 << (usize::BITS - 2)];
    if usize::BITS >= 64 {
        for n in edge_inputs {
            assert_eq!(sqrt(n), u32::MAX, "at {n}");
        }
    }
    // On platforms with smaller word sizes these values
    // should still be at parity with bogosqrt.
    else {
        for n in edge_inputs {
            assert_eq!(sqrt(n), bogosqrt(n), "at {n}");
        }
    }
}
