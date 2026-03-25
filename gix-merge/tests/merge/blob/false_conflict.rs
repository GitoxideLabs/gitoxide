use gix_merge::blob::{builtin_driver, builtin_driver::text::Conflict, Resolution};
use imara_diff::intern::InternedInput;

/// Minimal reproduction: Myers produces a false conflict where git merge-file resolves cleanly.
///
/// base:   alpha_x / (blank) / bravo_x / charlie_x / (blank)
/// ours:   (blank) / (blank) / bravo_x / charlie_x
/// theirs: alpha_x / (blank) / charlie_x / (blank)
///
/// base→ours:  alpha_x deleted (replaced by blank), trailing blank removed
/// base→theirs: bravo_x deleted
///
/// These are non-overlapping changes that git merges cleanly.
/// See https://github.com/GitoxideLabs/gitoxide/issues/2475
#[test]
fn myers_false_conflict_with_blank_line_ambiguity() {
    let base = b"alpha_x\n\nbravo_x\ncharlie_x\n\n";
    let ours = b"\n\nbravo_x\ncharlie_x\n";
    let theirs = b"alpha_x\n\ncharlie_x\n\n";

    let labels = builtin_driver::text::Labels {
        ancestor: Some("base".into()),
        current: Some("ours".into()),
        other: Some("theirs".into()),
    };

    // Histogram resolves cleanly.
    {
        let options = builtin_driver::text::Options {
            diff_algorithm: imara_diff::Algorithm::Histogram,
            conflict: Conflict::Keep {
                style: builtin_driver::text::ConflictStyle::Merge,
                marker_size: 7.try_into().unwrap(),
            },
        };
        let mut out = Vec::new();
        let mut input = InternedInput::default();
        let res = builtin_driver::text(&mut out, &mut input, labels, ours, base, theirs, options);
        assert_eq!(res, Resolution::Complete, "Histogram should resolve cleanly");
    }

    // Myers should also resolve cleanly (it used to produce a false conflict because
    // imara-diff's Myers splits the ours change into two hunks — a deletion at base[0]
    // and an empty insertion at base[2] — and the insertion collided with theirs'
    // deletion at base[2]).
    {
        let options = builtin_driver::text::Options {
            diff_algorithm: imara_diff::Algorithm::Myers,
            conflict: Conflict::Keep {
                style: builtin_driver::text::ConflictStyle::Merge,
                marker_size: 7.try_into().unwrap(),
            },
        };
        let mut out = Vec::new();
        let mut input = InternedInput::default();
        let res = builtin_driver::text(&mut out, &mut input, labels, ours, base, theirs, options);
        assert_eq!(
            res,
            Resolution::Complete,
            "Myers should resolve cleanly (git merge-file does). Output:\n{}",
            String::from_utf8_lossy(&out)
        );
    }
}
