use gix::diff::blob::{Algorithm, Diff, InternedInput, sources::words};

use super::{Change, Fragment, Line};

pub(super) fn diff(before: &str, after: &str, omit_delimiters: bool) -> Vec<Line> {
    let input = InternedInput::new(words(before), words(after));
    let mut changes = Diff::compute(Algorithm::Myers, &input);
    changes.postprocess_no_heuristic(&input);
    let mut output = vec![Line {
        change: Change::Modified,
        fragments: Vec::new(),
    }];
    let mut previous = 0;
    for hunk in changes.hunks() {
        append(
            &mut output,
            &input.before[previous..hunk.before.start as usize]
                .iter()
                .map(|token| input.interner[*token])
                .collect::<String>(),
            Change::Context,
        );
        for (tokens, open, close, change) in [
            (
                &input.before[hunk.before.start as usize..hunk.before.end as usize],
                "[-",
                "-]",
                Change::Removed,
            ),
            (
                &input.after[hunk.after.start as usize..hunk.after.end as usize],
                "{+",
                "+}",
                Change::Added,
            ),
        ] {
            if !tokens.is_empty() {
                let text: String = tokens.iter().map(|token| input.interner[*token]).collect();
                if omit_delimiters {
                    append(&mut output, &text, change);
                } else {
                    append(&mut output, &format!("{open}{text}{close}"), change);
                }
            }
        }
        previous = hunk.before.end as usize;
    }
    append(
        &mut output,
        &input.before[previous..]
            .iter()
            .map(|token| input.interner[*token])
            .collect::<String>(),
        Change::Context,
    );
    output
}

fn append(output: &mut Vec<Line>, text: &str, change: Change) {
    for (index, part) in text.split('\n').enumerate() {
        if index != 0 {
            output.push(Line {
                change: Change::Modified,
                fragments: Vec::new(),
            });
        }
        if !part.is_empty() {
            output
                .last_mut()
                .expect("word diffs always start with a row")
                .fragments
                .push(Fragment {
                    change,
                    text: part.to_owned(),
                });
        }
    }
}
