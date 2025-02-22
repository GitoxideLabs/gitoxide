use std::collections::BTreeSet;

use bstr::{BStr, ByteSlice};
use gix_glob::{pattern, pattern::Case};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone)]
pub struct GitMatch<'a> {
    pattern: &'a BStr,
    value: &'a BStr,
    /// True if git could match `value` with `pattern`
    is_match: bool,
}

pub struct Baseline<'a> {
    inner: bstr::Lines<'a>,
}

impl<'a> Iterator for Baseline<'a> {
    type Item = GitMatch<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut tokens = self.inner.next()?.splitn(2, |b| *b == b' ');
        let pattern = tokens.next().expect("pattern").as_bstr();
        let value = tokens.next().expect("value").as_bstr().trim_start().as_bstr();

        let git_match = self.inner.next()?;
        let is_match = !git_match.starts_with(b"::\t");
        Some(GitMatch {
            pattern,
            value,
            is_match,
        })
    }
}

impl<'a> Baseline<'a> {
    fn new(input: &'a [u8]) -> Self {
        Baseline {
            inner: input.as_bstr().lines(),
        }
    }
}

#[test]
fn compare_baseline_with_ours() {
    let dir = gix_testtools::scripted_fixture_read_only("make_baseline.sh").unwrap();
    let (mut total_matches, mut total_correct, mut panics) = (0, 0, 0);
    let mut mismatches = Vec::new();
    for (input_file, expected_matches, case) in &[
        ("git-baseline.match", true, pattern::Case::Sensitive),
        ("git-baseline.nmatch", false, pattern::Case::Sensitive),
        ("git-baseline.match-icase", true, pattern::Case::Fold),
    ] {
        let input = std::fs::read(dir.join(*input_file)).unwrap();
        let mut seen = BTreeSet::default();

        for m @ GitMatch {
            pattern,
            value,
            is_match,
        } in Baseline::new(&input)
        {
            total_matches += 1;
            assert!(seen.insert(m), "duplicate match entry: {m:?}");
            assert_eq!(
                is_match, *expected_matches,
                "baseline for matches must be {expected_matches} - check baseline and git version: {m:?}"
            );
            let actual = std::panic::catch_unwind(|| {
                let pattern = pat(pattern);
                pattern.matches_repo_relative_path(
                    value,
                    basename_start_pos(value),
                    None,
                    *case,
                    gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
                )
            });
            match actual {
                Ok(actual_match) => {
                    if actual_match == is_match {
                        total_correct += 1;
                    } else {
                        mismatches.push((pattern.to_owned(), value.to_owned(), is_match, expected_matches));
                    }
                }
                Err(_) => {
                    panics += 1;
                    continue;
                }
            }
        }
    }

    dbg!(mismatches);
    assert_eq!(
        total_correct,
        total_matches - panics,
        "We perfectly agree with git here"
    );
    assert_eq!(panics, 0);
}

#[test]
fn non_dirs_for_must_be_dir_patterns_are_ignored() {
    let pattern = pat("hello/");

    assert!(pattern.mode.contains(pattern::Mode::MUST_BE_DIR));
    assert_eq!(
        pattern.text, "hello",
        "a dir pattern doesn't actually end with the trailing slash"
    );
    let path = "hello";
    assert!(
        !pattern.matches_repo_relative_path(
            path.into(),
            None,
            false.into(), /* is-dir */
            Case::Sensitive,
            gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL
        ),
        "non-dirs never match a dir pattern"
    );
    assert!(
        pattern.matches_repo_relative_path(
            path.into(),
            None,
            true.into(), /* is-dir */
            Case::Sensitive,
            gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL
        ),
        "dirs can match a dir pattern with the normal rules"
    );
}

#[test]
fn matches_of_absolute_paths_work() {
    let pattern = "/hello/git";
    assert!(
        gix_glob::wildmatch(pattern.into(), pattern.into(), gix_glob::wildmatch::Mode::empty()),
        "patterns always match themselves"
    );
    assert!(
        gix_glob::wildmatch(
            pattern.into(),
            pattern.into(),
            gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL
        ),
        "patterns always match themselves, path mode doesn't change that"
    );
}

#[test]
fn basename_matches_from_end() {
    let pat = &pat("foo");
    assert!(match_file(pat, "FoO", Case::Fold));
    assert!(!match_file(pat, "FoOo", Case::Fold));
    assert!(!match_file(pat, "Foo", Case::Sensitive));
    assert!(match_file(pat, "foo", Case::Sensitive));
    assert!(!match_file(pat, "Foo", Case::Sensitive));
    assert!(!match_file(pat, "barfoo", Case::Sensitive));
}

#[test]
fn absolute_basename_matches_only_from_beginning() {
    let pat = &pat("/foo");
    assert!(match_file(pat, "FoO", Case::Fold));
    assert!(!match_file(pat, "bar/Foo", Case::Fold));
    assert!(match_file(pat, "foo", Case::Sensitive));
    assert!(!match_file(pat, "Foo", Case::Sensitive));
    assert!(!match_file(pat, "bar/foo", Case::Sensitive));
}

#[test]
fn absolute_path_matches_only_from_beginning() {
    let pat = &pat("/bar/foo");
    assert!(!match_file(pat, "FoO", Case::Fold));
    assert!(match_file(pat, "bar/Foo", Case::Fold));
    assert!(!match_file(pat, "foo", Case::Sensitive));
    assert!(match_file(pat, "bar/foo", Case::Sensitive));
    assert!(!match_file(pat, "bar/Foo", Case::Sensitive));
}

#[test]
fn absolute_path_with_recursive_glob_detects_mismatches_quickly() {
    let pat = &pat("/bar/foo/**");
    assert!(!match_file(pat, "FoO", Case::Fold));
    assert!(!match_file(pat, "bar/Fooo", Case::Fold));
    assert!(!match_file(pat, "baz/bar/Foo", Case::Fold));
}

#[test]
fn absolute_path_with_recursive_glob_can_do_case_insensitive_prefix_search() {
    let pat = &pat("/bar/foo/**");
    assert!(!match_file(pat, "bar/Foo/match", Case::Sensitive));
    assert!(match_file(pat, "bar/Foo/match", Case::Fold));
}

#[test]
fn relative_path_does_not_match_from_end() {
    for pattern in &["bar/foo", "/bar/foo"] {
        let pattern = &pat(*pattern);
        assert!(!match_file(pattern, "FoO", Case::Fold));
        assert!(match_file(pattern, "bar/Foo", Case::Fold));
        assert!(!match_file(pattern, "baz/bar/Foo", Case::Fold));
        assert!(!match_file(pattern, "foo", Case::Sensitive));
        assert!(match_file(pattern, "bar/foo", Case::Sensitive));
        assert!(!match_file(pattern, "baz/bar/foo", Case::Sensitive));
        assert!(!match_file(pattern, "Baz/bar/Foo", Case::Sensitive));
    }
}

#[test]
fn basename_glob_and_literal_is_ends_with() {
    let pattern = &pat("*foo");
    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "BarFoO", Case::Fold));
    assert!(!match_file(pattern, "BarFoOo", Case::Fold));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "BarFoo", Case::Sensitive));
    assert!(match_file(pattern, "barfoo", Case::Sensitive));
    assert!(!match_file(pattern, "barfooo", Case::Sensitive));

    assert!(match_file(pattern, "bar/foo", Case::Sensitive));
    assert!(match_file(pattern, "bar/bazfoo", Case::Sensitive));
}

#[test]
fn special_cases_from_corpus() {
    let pattern = &pat("foo*bar");
    assert!(
        !match_file(pattern, "foo/baz/bar", Case::Sensitive),
        "asterisk does not match path separators"
    );
    let pattern = &pat("*some/path/to/hello.txt");
    assert!(
        !match_file(pattern, "a/bigger/some/path/to/hello.txt", Case::Sensitive),
        "asterisk doesn't match path separators"
    );

    let pattern = &pat("/*foo.txt");
    assert!(match_file(pattern, "hello-foo.txt", Case::Sensitive));
    assert!(
        !match_file(pattern, "hello/foo.txt", Case::Sensitive),
        "absolute single asterisk doesn't match paths"
    );
}

#[test]
fn absolute_basename_glob_and_literal_is_ends_with_in_basenames() {
    let pattern = &pat("/*foo");

    assert!(match_file(pattern, "FoO", Case::Fold));
    assert!(match_file(pattern, "BarFoO", Case::Fold));
    assert!(!match_file(pattern, "BarFoOo", Case::Fold));
    assert!(!match_file(pattern, "Foo", Case::Sensitive));
    assert!(!match_file(pattern, "BarFoo", Case::Sensitive));
    assert!(match_file(pattern, "barfoo", Case::Sensitive));
    assert!(!match_file(pattern, "barfooo", Case::Sensitive));
}

#[test]
fn absolute_basename_glob_and_literal_is_glob_in_paths() {
    let pattern = &pat("/*foo");

    assert!(!match_file(pattern, "bar/foo", Case::Sensitive), "* does not match /");
    assert!(!match_file(pattern, "bar/bazfoo", Case::Sensitive));
}

#[test]
fn negated_patterns_are_handled_by_caller() {
    let pattern = &pat("!foo");
    assert!(
        match_file(pattern, "foo", Case::Sensitive),
        "negative patterns match like any other"
    );
    assert!(
        pattern.is_negative(),
        "the caller checks for the negative flag and acts accordingly"
    );
}
#[test]
fn names_do_not_automatically_match_entire_directories() {
    // this feature is implemented with the directory stack.
    let pattern = &pat("foo");
    assert!(!match_file(pattern, "foobar", Case::Sensitive));
    assert!(!match_file(pattern, "foo/bar", Case::Sensitive));
    assert!(!match_file(pattern, "foo/bar/baz", Case::Sensitive));
}

#[test]
fn directory_patterns_do_not_match_files_within_a_directory_as_well_like_slash_star_star() {
    // this feature is implemented with the directory stack in `gix-ignore`, which excludes entire directories
    let pattern = &pat("dir/");
    assert!(!match_path(pattern, "dir/file", None, Case::Sensitive));
    assert!(!match_path(pattern, "base/dir/file", None, Case::Sensitive));
    assert!(!match_path(pattern, "base/ndir/file", None, Case::Sensitive));
    assert!(!match_path(pattern, "Dir/File", None, Case::Fold));
    assert!(!match_path(pattern, "Base/Dir/File", None, Case::Fold));
    assert!(!match_path(pattern, "dir2/file", None, Case::Sensitive));

    let pattern = &pat("dir/sub-dir/");
    assert!(!match_path(pattern, "dir/sub-dir/file", None, Case::Sensitive));
    assert!(!match_path(pattern, "dir/Sub-dir/File", None, Case::Fold));
    assert!(!match_path(pattern, "dir/Sub-dir2/File", None, Case::Fold));
}

#[test]
fn single_paths_match_anywhere() {
    let pattern = &pat("target");
    assert!(match_file(pattern, "dir/target", Case::Sensitive));
    assert!(!match_file(pattern, "dir/atarget", Case::Sensitive));
    assert!(!match_file(pattern, "dir/targeta", Case::Sensitive));
    assert!(match_path(pattern, "dir/target", Some(true), Case::Sensitive));

    let pattern = &pat("target/");
    assert!(!match_file(pattern, "dir/target", Case::Sensitive));
    assert!(
        !match_path(pattern, "dir/target", None, Case::Sensitive),
        "it assumes unknown to not be a directory"
    );
    assert!(match_path(pattern, "dir/target", Some(true), Case::Sensitive));
    assert!(
        !match_path(pattern, "dir/target/", Some(true), Case::Sensitive),
        "we need sanitized paths that don't have trailing slashes"
    );
}

#[test]
fn fuzzed_exponential_runaway_denial_of_service() {
    // original: "?[at(/\u{1d}\0\u{4}\u{14}\0[[[[:[\0\0\0\0\0\0\0\0Wt(/\u{1d}\0\u{4}\u{14}\0[[[[:[\0\0\0\0\0\0\0\0\0\0\0]"
    //  reduced: "[[:[:]"
    for pattern in [
        include_bytes!("../fixtures/fuzzed/many-stars.pattern"),
        "*?[wxxxxxx\0!t[:rt]\u{14}*".as_bytes(),
        "?[at(/\u{1d}\0\u{4}\u{14}\0[[[[:[\0\0\0\0\0\0\0\0\0\0/s\0\0\0*\0\0\0\0\0\0\0\0]\0\0\0\0\0\0\0\0\0".as_bytes(),
        b"[[:digit]ab]",
        b"[[:]ab]",
        b"[[:[:x]",
    ] {
        let pat = pat(pattern);
        match_file(&pat, "relative/path", Case::Sensitive);
    }
}

fn pat<'a>(pattern: impl Into<&'a BStr>) -> gix_glob::Pattern {
    gix_glob::Pattern::from_bytes(pattern.into()).expect("parsing works")
}

fn match_file<'a>(pattern: &gix_glob::Pattern, path: impl Into<&'a BStr>, case: Case) -> bool {
    match_path(pattern, path, false.into(), case)
}

fn match_path<'a>(pattern: &gix_glob::Pattern, path: impl Into<&'a BStr>, is_dir: Option<bool>, case: Case) -> bool {
    let path = path.into();
    pattern.matches_repo_relative_path(
        path,
        basename_start_pos(path),
        is_dir,
        case,
        gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
    )
}

fn basename_start_pos(value: &BStr) -> Option<usize> {
    value.rfind_byte(b'/').map(|pos| pos + 1)
}
