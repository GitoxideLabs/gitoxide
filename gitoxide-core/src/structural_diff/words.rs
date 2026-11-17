use gix_imara_diff::{Algorithm, Diff, InternedInput, sources::words};
use owo_colors::{OwoColorize, Style};

/// Inputs contain only sanitized structural text; output may contain generated ANSI styling.
pub(super) fn diff(before: &str, after: &str, color: bool) -> String {
    let input = InternedInput::new(words(before), words(after));
    let mut changes = Diff::compute(Algorithm::Myers, &input);
    changes.postprocess_no_heuristic(&input);
    let mut output = String::new();
    let mut previous = 0;
    for hunk in changes.hunks() {
        output.extend(
            input.before[previous..hunk.before.start as usize]
                .iter()
                .map(|token| input.interner[*token]),
        );
        for (tokens, open, close, style) in [
            (
                &input.before[hunk.before.start as usize..hunk.before.end as usize],
                "[-",
                "-]",
                Style::new().red().bold(),
            ),
            (
                &input.after[hunk.after.start as usize..hunk.after.end as usize],
                "{+",
                "+}",
                Style::new().green().bold(),
            ),
        ] {
            if !tokens.is_empty() {
                let text: String = tokens.iter().map(|token| input.interner[*token]).collect();
                let marked = format!("{open}{text}{close}");
                if color {
                    output.push_str(&marked.style(style).to_string());
                } else {
                    output.push_str(&marked);
                }
            }
        }
        previous = hunk.before.end as usize;
    }
    output.extend(input.before[previous..].iter().map(|token| input.interner[*token]));
    output
}

#[cfg(test)]
mod tests {
    use gix::error::TestResult;

    use super::diff;

    #[test]
    fn identifiers_preserve_unicode_tabs_and_punctuation() -> TestResult {
        assert_eq!(
            diff(
                "\tfn café(arg: i32) -> i32 { … }",
                "\tfn thé(arg: u32) -> i32 { … }",
                false
            ),
            "\tfn [-café-]{+thé+}(arg: [-i32-]{+u32+}) -> i32 { … }",
            "word changes retain surrounding signature punctuation and source indentation"
        );
        Ok(())
    }

    #[test]
    fn unchanged_multiline_text_is_lossless() -> TestResult {
        let signature = "\tfn café(
\t\targ: i32,
\t) -> i32 { … }";
        assert_eq!(
            diff(signature, signature, false),
            signature,
            "unchanged whitespace and Unicode remain byte-for-byte identical"
        );
        assert_eq!(
            diff(signature, &signature.replace("arg", "value"), false),
            signature.replace("arg", "[-arg-]{+value+}"),
            "a changed word does not disturb its surrounding multiline layout"
        );
        Ok(())
    }

    #[test]
    fn insertions_deletions_and_empty_inputs() -> TestResult {
        for (before, after, expected) in [
            ("", "", ""),
            ("", "fn new();", "{+fn new();+}"),
            ("fn old();", "", "[-fn old();-]"),
            ("fn f();", "pub fn f();", "{+pub +}fn f();"),
            ("pub fn f();", "fn f();", "[-pub -]fn f();"),
            ("fn f(a: i32)", "fn f(a: &i32)", "fn f(a: {+&+}i32)"),
        ] {
            assert_eq!(
                diff(before, after, false),
                expected,
                "only changed tokens receive delimiters"
            );
        }
        Ok(())
    }

    #[test]
    fn color_styles_only_changed_spans() -> TestResult {
        let output = diff("fn f(a: i32)", "fn f(a: u32)", true);
        assert!(output.starts_with("fn f(a: "), "unchanged text has no terminal styling");
        assert!(
            output.ends_with(')'),
            "trailing punctuation remains outside changed spans"
        );
        assert!(
            output.contains("[-i32-]"),
            "removal delimiters remain readable with color"
        );
        assert!(
            output.contains("{+u32+}"),
            "addition delimiters remain readable with color"
        );
        assert!(output.contains('\u{1b}'), "color mode generates ANSI styling");
        assert_eq!(
            diff("fn f()", "fn f()", true),
            "fn f()",
            "unchanged text needs no color"
        );
        Ok(())
    }
}
