use gix::diff::blob::{Algorithm, Diff, InternedInput, sources::words};

use super::parse::Item;

pub(super) fn candidates(before: &[&Item], after: &[&Item]) -> Vec<(usize, usize)> {
    if before.is_empty() || after.is_empty() {
        return Vec::new();
    }
    let before_tokens: Vec<_> = before.iter().map(|item| normalized(item)).collect();
    let after_tokens: Vec<_> = after.iter().map(|item| normalized(item)).collect();
    let mut before_best = vec![(None, 0.0); before.len()];
    let mut after_best = vec![(None, 0.0); after.len()];
    for (left, old) in before.iter().enumerate() {
        for (right, new) in after.iter().enumerate() {
            if old.kind != new.kind || old.name == new.name {
                continue;
            }
            let mut input = InternedInput::default();
            input.update_before(before_tokens[left].iter().copied());
            input.update_after(after_tokens[right].iter().copied());
            let diff = Diff::compute(Algorithm::Myers, &input);
            let common = input.before.len() - diff.count_removals() as usize;
            let total = input.before.len() as f64 + input.after.len() as f64;
            if total == 0.0 {
                continue;
            }
            let score = 2.0 * common as f64 / total;
            if score < 0.6 {
                continue;
            }
            for (best, index) in [(&mut before_best[left], right), (&mut after_best[right], left)] {
                match score.total_cmp(&best.1) {
                    std::cmp::Ordering::Greater => *best = (Some(index), score),
                    std::cmp::Ordering::Equal => best.0 = None,
                    std::cmp::Ordering::Less => {}
                }
            }
        }
    }
    before_best
        .into_iter()
        .enumerate()
        .filter_map(|(left, (right, _))| {
            right
                .filter(|right| after_best[*right].0 == Some(left))
                .map(|right| (left, right))
        })
        .collect()
}

fn normalized(item: &Item) -> Vec<Option<&str>> {
    let tokens: Vec<_> = words(&item.content)
        .filter(|token| !token.chars().all(char::is_whitespace))
        .collect();
    let name: Vec<_> = words(&item.name)
        .filter(|token| !token.chars().all(char::is_whitespace))
        .collect();
    let mut out = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        if !name.is_empty() && tokens[index..].starts_with(&name) {
            // Recursive references should not make a function rename look like a body rewrite.
            out.push(None);
            index += name.len();
        } else {
            out.push(Some(tokens[index]));
            index += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gix_testtools::TestResult;

    fn item(kind: &'static str, name: &str, content: &str) -> Item {
        Item {
            kind,
            name: name.into(),
            header: String::new(),
            footer: String::new(),
            content: content.into(),
            children: Vec::new(),
            test_only: false,
            contains_tests: false,
            parameters: None,
        }
    }

    #[test]
    fn exact_renames_ignore_whitespace_and_recursive_name_references() -> TestResult {
        let before = item("function", "old", "fn old() { old(); }");
        let after = item(
            "function",
            "new",
            "fn new() {
\tnew();
}",
        );
        assert_eq!(
            candidates(&[&before], &[&after]),
            [(0, 0)],
            "renaming the declaration and recursive calls preserves the function's identity"
        );
        Ok(())
    }

    #[test]
    fn renames_can_include_body_edits() -> TestResult {
        let before = item("function", "old", "fn old(value: u32) -> u32 { value + 1 }");
        let after = item("function", "new", "fn new(value: u32) -> u32 { value + 2 }");
        assert_eq!(
            candidates(&[&before], &[&after]),
            [(0, 0)],
            "a small body edit does not hide an otherwise similar rename"
        );
        Ok(())
    }

    #[test]
    fn equal_candidates_on_either_side_remain_unmatched() -> TestResult {
        let before = item("function", "old", "fn old() {}");
        let after = item("function", "new", "fn new() {}");
        let other = item("function", "other", "fn other() {}");
        assert!(
            candidates(&[&before], &[&after, &other]).is_empty(),
            "one old declaration cannot choose between equally similar replacements"
        );
        assert!(
            candidates(&[&before, &other], &[&after]).is_empty(),
            "a replacement cannot choose between equally similar old declarations"
        );
        Ok(())
    }

    #[test]
    fn different_kinds_and_unchanged_names_are_not_renames() -> TestResult {
        let before = item("function", "old", "old");
        let after = item("struct", "new", "new");
        assert!(
            candidates(&[&before], &[&after]).is_empty(),
            "matching token sequences cannot rename a function into a struct"
        );
        assert!(
            candidates(&[&before], &[&before]).is_empty(),
            "ordinary matching handles unchanged names"
        );
        Ok(())
    }

    #[test]
    fn unicode_and_multiword_names_are_normalized_without_substring_replacement() -> TestResult {
        let before = item("function", "café", "fn café() { café(); } ");
        let after = item("function", "茶", "fn 茶() { 茶(); }");
        assert_eq!(
            candidates(&[&before], &[&after]),
            [(0, 0)],
            "Unicode names are complete word tokens"
        );
        let heading = item(
            "heading",
            "Old title",
            "# Old title
An Old title appears here.",
        );
        assert_eq!(
            normalized(&heading).iter().filter(|token| token.is_none()).count(),
            2,
            "multiword declaration names are normalized as complete token sequences"
        );
        assert_eq!(
            normalized(&item("function", "old", "oldish old")),
            [Some("oldish"), None],
            "name normalization never rewrites part of another identifier"
        );
        Ok(())
    }
}
