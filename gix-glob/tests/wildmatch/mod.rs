use std::{
    fmt::{Debug, Display, Formatter},
    panic::catch_unwind,
};

use bstr::ByteSlice;
use gix_glob::{Pattern, pattern::Case, wildmatch};

#[test]
fn corpus() {
    // based on git/t/t3070.sh
    let tests = [
        (1u8, 1u8, 1u8, 1u8, "foo", "foo"),
        (0, 0, 0, 0, "foo", "bar"),
        (1, 1, 1, 1, "foo", "???"),
        (0, 0, 0, 0, "foo", "??"),
        (1, 1, 1, 1, "foo", "*"),
        (1, 1, 1, 1, "foo", "f*"),
        (0, 0, 0, 0, "foo", "*f"),
        (1, 1, 1, 1, "foo", "*foo*"),
        (1, 1, 1, 1, "foobar", "*ob*a*r*"),
        (1, 1, 1, 1, "aaaaaaabababab", "*ab"),
        (1, 1, 1, 1, "foo*", r"foo\*"),
        (0, 0, 0, 0, "foobar", r"foo\*bar"),
        (1, 1, 1, 1, r"f\oo", r"f\\oo"),
        (1, 1, 1, 1, "ball", "*[al]?"),
        (0, 0, 0, 0, "ten", "[ten]"),
        (1, 1, 1, 1, "ten", "**[!te]"),
        (0, 0, 0, 0, "ten", "**[!ten]"),
        (1, 1, 1, 1, "ten", "t[a-g]n"),
        (0, 0, 0, 0, "ten", "t[!a-g]n"),
        (1, 1, 1, 1, "ton", "t[!a-g]n"),
        (1, 1, 1, 1, "ton", "t[^a-g]n"),
        (1, 1, 1, 1, "a]b", "a[]]b"),
        (1, 1, 1, 1, "a-b", "a[]-]b"),
        (1, 1, 1, 1, "a]b", "a[]-]b"),
        (0, 0, 0, 0, "aab", "a[]-]b"),
        (1, 1, 1, 1, "aab", "a[]a-]b"),
        (1, 1, 1, 1, "]", "]"),
        // Extended slash-matching features
        (0, 0, 1, 1, "foo/baz/bar", "foo*bar"),
        (0, 0, 1, 1, "foo/baz/bar", "foo**bar"),
        (1, 1, 1, 1, "foobazbar", "foo**bar"),
        (1, 1, 1, 1, "foo/baz/bar", "foo/**/bar"),
        (1, 1, 0, 0, "foo/baz/bar", "foo/**/**/bar"),
        (1, 1, 1, 1, "foo/b/a/z/bar", "foo/**/bar"),
        (1, 1, 1, 1, "foo/b/a/z/bar", "foo/**/**/bar"),
        (1, 1, 0, 0, "foo/bar", "foo/**/bar"),
        (1, 1, 0, 0, "foo/bar", "foo/**/**/bar"),
        (0, 0, 1, 1, "foo/bar", "foo?bar"),
        (0, 0, 1, 1, "foo/bar", "foo[/]bar"),
        (0, 0, 1, 1, "foo/bar", "foo[^a-z]bar"),
        (0, 0, 1, 1, "foo/bar", "f[^eiu][^eiu][^eiu][^eiu][^eiu]r"),
        (1, 1, 1, 1, "foo-bar", "f[^eiu][^eiu][^eiu][^eiu][^eiu]r"),
        (1, 1, 0, 0, "foo", "**/foo"),
        (1, 1, 1, 1, "XXX/foo", "**/foo"),
        (1, 1, 1, 1, "bar/baz/foo", "**/foo"),
        (0, 0, 1, 1, "bar/baz/foo", "*/foo"),
        (0, 0, 1, 1, "foo/bar/baz", "**/bar*"),
        (1, 1, 1, 1, "deep/foo/bar/baz", "**/bar/*"),
        (0, 0, 1, 1, "deep/foo/bar/baz/", "**/bar/*"),
        (1, 1, 1, 1, "deep/foo/bar/baz/", "**/bar/**"),
        (0, 0, 0, 0, "deep/foo/bar", "**/bar/*"),
        (1, 1, 1, 1, "deep/foo/bar/", "**/bar/**"),
        (0, 0, 1, 1, "foo/bar/baz", "**/bar**"),
        (1, 1, 1, 1, "foo/bar/baz/x", "*/bar/**"),
        (0, 0, 1, 1, "deep/foo/bar/baz/x", "*/bar/**"),
        (1, 1, 1, 1, "deep/foo/bar/baz/x", "**/bar/*/*"),
        // Various additional tests
        (0, 0, 0, 0, "acrt", "a[c-c]st"),
        (1, 1, 1, 1, "acrt", "a[c-c]rt"),
        (0, 0, 0, 0, "]", "[!]-]"),
        (1, 1, 1, 1, "a", "[!]-]"),
        (0, 0, 0, 0, "", r"\"),
        (0, 0, 0, 0, r"XXX/\", r"*/\"),
        (1, 1, 1, 1, r"XXX/\", r"*/\\"),
        (1, 1, 1, 1, "foo", "foo"),
        (1, 1, 1, 1, "@foo", "@foo"),
        (0, 0, 0, 0, "foo", "@foo"),
        (1, 1, 1, 1, "[ab]", r"\[ab]"),
        (1, 1, 1, 1, "[ab]", "[[]ab]"),
        (1, 1, 1, 1, "[ab]", "[[:]ab]"),
        (0, 0, 0, 0, "[ab]", "[[::]ab]"),
        (1, 1, 1, 1, "[ab]", "[[:digit]ab]"),
        (1, 1, 1, 1, "[ab]", r"[\[:]ab]"),
        (1, 1, 1, 1, "?a?b", r"\??\?b"),
        (1, 1, 1, 1, "abc", r"\a\b\c"),
        (1, 1, 1, 1, "foo/bar/baz/to", "**/t[o]"),
        // Character class tests
        (1, 1, 1, 1, "a1B", "[[:alpha:]][[:digit:]][[:upper:]]"),
        (0, 1, 0, 1, "a", "[[:digit:][:upper:][:space:]]"),
        (1, 1, 1, 1, "A", "[[:digit:][:upper:][:space:]]"),
        (1, 1, 1, 1, "1", "[[:digit:][:upper:][:space:]]"),
        (0, 0, 0, 0, "1", "[[:digit:][:upper:][:spaci:]]"),
        (1, 1, 1, 1, " ", "[[:digit:][:upper:][:space:]]"),
        (0, 0, 0, 0, ".", "[[:digit:][:upper:][:space:]]"),
        (1, 1, 1, 1, ".", "[[:digit:][:punct:][:space:]]"),
        (1, 1, 1, 1, "5", "[[:xdigit:]]"),
        (1, 1, 1, 1, "f", "[[:xdigit:]]"),
        (1, 1, 1, 1, "D", "[[:xdigit:]]"),
        (
            1,
            1,
            1,
            1,
            "_",
            "[[:alnum:][:alpha:][:blank:][:cntrl:][:digit:][:graph:][:lower:][:print:][:punct:][:space:][:upper:][:xdigit:]]",
        ),
        (
            1,
            1,
            1,
            1,
            ".",
            "[^[:alnum:][:alpha:][:blank:][:cntrl:][:digit:][:lower:][:space:][:upper:][:xdigit:]]",
        ),
        (1, 1, 1, 1, "5", "[a-c[:digit:]x-z]"),
        (1, 1, 1, 1, "b", "[a-c[:digit:]x-z]"),
        (1, 1, 1, 1, "y", "[a-c[:digit:]x-z]"),
        (0, 0, 0, 0, "q", "[a-c[:digit:]x-z]"),
        // Additional tests, including some malformed wild(patterns
        (1, 1, 1, 1, "]", r"[\\-^]"),
        (0, 0, 0, 0, "[", r"[\\-^]"),
        (1, 1, 1, 1, "-", r"[\-_]"),
        (1, 1, 1, 1, "]", r"[\]]"),
        (0, 0, 0, 0, r"\]", r"[\]]"),
        (0, 0, 0, 0, r"\", r"[\]]"),
        (0, 0, 0, 0, "ab", "a[]b"),
        (0, 0, 0, 0, "ab", "[!"),
        (0, 0, 0, 0, "ab", "[-"),
        (1, 1, 1, 1, "-", "[-]"),
        (0, 0, 0, 0, "-", "[a-"),
        (0, 0, 0, 0, "-", "[!a-"),
        (1, 1, 1, 1, "-", "[--A]"),
        (1, 1, 1, 1, "5", "[--A]"),
        (1, 1, 1, 1, " ", "[ --]"),
        (1, 1, 1, 1, "$", "[ --]"),
        (1, 1, 1, 1, "-", "[ --]"),
        (0, 0, 0, 0, "0", "[ --]"),
        (1, 1, 1, 1, "-", "[---]"),
        (1, 1, 1, 1, "-", "[------]"),
        (0, 0, 0, 0, "j", "[a-e-n]"),
        (1, 1, 1, 1, "-", "[a-e-n]"),
        (1, 1, 1, 1, "a", "[!------]"),
        (0, 0, 0, 0, "[", "[]-a]"),
        (1, 1, 1, 1, "^", "[]-a]"),
        (0, 0, 0, 0, "^", "[!]-a]"),
        (1, 1, 1, 1, "[", "[!]-a]"),
        (1, 1, 1, 1, "^", "[a^bc]"),
        (1, 1, 1, 1, "-b]", "[a-]b]"),
        (0, 0, 0, 0, r"\", r"[\]"),
        (1, 1, 1, 1, r"\", r"[\\]"),
        (0, 0, 0, 0, r"\", r"[!\\]"),
        (1, 1, 1, 1, "G", r"[A-\\]"),
        (0, 0, 0, 0, "aaabbb", "b*a"),
        (0, 0, 0, 0, "aabcaa", "*ba*"),
        (1, 1, 1, 1, ",", "[,]"),
        (1, 1, 1, 1, ",", r"[\\,]"),
        (1, 1, 1, 1, r"\", r"[\\,]"),
        (1, 1, 1, 1, "-", "[,-.]"),
        (0, 0, 0, 0, "+", "[,-.]"),
        (0, 0, 0, 0, "-.]", "[,-.]"),
        (1, 1, 1, 1, "2", r"[\1-\3]"),
        (1, 1, 1, 1, "3", r"[\1-\3]"),
        (0, 0, 0, 0, "4", r"[\1-\3]"),
        (1, 1, 1, 1, r"\", r"[[-\]]"),
        (1, 1, 1, 1, "[", r"[[-\]]"),
        (1, 1, 1, 1, "]", r"[[-\]]"),
        (0, 0, 0, 0, "-", r"[[-\]]"),
        // Test recursion
        (
            1,
            1,
            1,
            1,
            "-adobe-courier-bold-o-normal--12-120-75-75-m-70-iso8859-1",
            "-*-*-*-*-*-*-12-*-*-*-m-*-*-*",
        ),
        (
            0,
            0,
            0,
            0,
            "-adobe-courier-bold-o-normal--12-120-75-75-X-70-iso8859-1",
            "-*-*-*-*-*-*-12-*-*-*-m-*-*-*",
        ),
        (
            0,
            0,
            0,
            0,
            "-adobe-courier-bold-o-normal--12-120-75-75-/-70-iso8859-1",
            "-*-*-*-*-*-*-12-*-*-*-m-*-*-*",
        ),
        (
            1,
            1,
            1,
            1,
            "XXX/adobe/courier/bold/o/normal//12/120/75/75/m/70/iso8859/1",
            "XXX/*/*/*/*/*/*/12/*/*/*/m/*/*/*",
        ),
        (
            0,
            0,
            0,
            0,
            "XXX/adobe/courier/bold/o/normal//12/120/75/75/X/70/iso8859/1",
            "XXX/*/*/*/*/*/*/12/*/*/*/m/*/*/*",
        ),
        (
            1,
            1,
            1,
            1,
            "abcd/abcdefg/abcdefghijk/abcdefghijklmnop.txt",
            "**/*a*b*g*n*t",
        ),
        (
            0,
            0,
            0,
            0,
            "abcd/abcdefg/abcdefghijk/abcdefghijklmnop.txtz",
            "**/*a*b*g*n*t",
        ),
        (0, 0, 0, 0, "foo", "*/*/*"),
        (0, 0, 0, 0, "foo/bar", "*/*/*"),
        (1, 1, 1, 1, "foo/bba/arr", "*/*/*"),
        (0, 0, 1, 1, "foo/bb/aa/rr", "*/*/*"),
        (1, 1, 1, 1, "foo/bb/aa/rr", "**/**/**"),
        (1, 1, 1, 1, "abcXdefXghi", "*X*i"),
        (0, 0, 1, 1, "ab/cXd/efXg/hi", "*X*i"),
        (1, 1, 1, 1, "ab/cXd/efXg/hi", "*/*X*/*/*i"),
        (1, 1, 1, 1, "ab/cXd/efXg/hi", "**/*X*/**/*i"),
        // Extra path(tests
        (0, 0, 0, 0, "foo", "fo"),
        (1, 1, 1, 1, "foo/bar", "foo/bar"),
        (1, 1, 1, 1, "foo/bar", "foo/*"),
        (0, 0, 1, 1, "foo/bba/arr", "foo/*"),
        (1, 1, 1, 1, "foo/bba/arr", "foo/**"),
        (0, 0, 1, 1, "foo/bba/arr", "foo*"),
        (0, 0, 1, 1, "foo/bba/arr", "foo/*arr"),
        (0, 0, 1, 1, "foo/bba/arr", "foo/**arr"),
        (0, 0, 0, 0, "foo/bba/arr", "foo/*z"),
        (0, 0, 0, 0, "foo/bba/arr", "foo/**z"),
        (0, 0, 1, 1, "foo/bar", "foo?bar"),
        (0, 0, 1, 1, "foo/bar", "foo[/]bar"),
        (0, 0, 1, 1, "foo/bar", "foo[^a-z]bar"),
        (0, 0, 1, 1, "ab/cXd/efXg/hi", "*Xg*i"),
        // Extra case-sensitivity tests
        (0, 1, 0, 1, "a", "[A-Z]"),
        (1, 1, 1, 1, "A", "[A-Z]"),
        (0, 1, 0, 1, "A", "[a-z]"),
        (1, 1, 1, 1, "a", "[a-z]"),
        (0, 1, 0, 1, "a", "[[:upper:]]"),
        (1, 1, 1, 1, "A", "[[:upper:]]"),
        (0, 1, 0, 1, "A", "[[:lower:]]"),
        (1, 1, 1, 1, "a", "[[:lower:]]"),
        (0, 1, 0, 1, "A", "[B-Za]"),
        (1, 1, 1, 1, "a", "[B-Za]"),
        (0, 1, 0, 1, "A", "[B-a]"),
        (1, 1, 1, 1, "a", "[B-a]"),
        (0, 1, 0, 1, "z", "[Z-y]"),
        (1, 1, 1, 1, "Z", "[Z-y]"),
    ];

    let mut failures = Vec::new();
    let mut at_least_one_panic = 0;
    for (path_match, path_imatch, glob_match, glob_imatch, text, pattern_text) in tests {
        let (pattern, actual) = multi_match(pattern_text, text);
        let expected = expect_multi(path_match, path_imatch, glob_match, glob_imatch);

        if actual.all_panicked() {
            at_least_one_panic += 1;
        } else if actual != expected {
            failures.push((pattern, pattern_text, text, actual, expected));
        } else {
            at_least_one_panic += i32::from(actual.any_panicked());
        }
    }

    dbg!(&failures);
    assert_eq!(failures.len(), 0);
    assert_eq!(at_least_one_panic, 0, "not a single panic in any invocation");

    // TODO: reproduce these
    // (0 0 0 0 \
    // 1 1 1 1 '\' '\'
    // (0 0 0 0 \
    // E E E E 'foo' ''
    // (0 0 0 0 \
    // 1 1 1 1 'a[]b' 'a[]b'
    //     (0 0 0 0 \
    //      1 1 1 1 'ab[' 'ab['
    // (0 0 1 1 \
    // 1 1 1 1 foo/bba/arr 'foo**'
}

#[test]
fn brackets() {
    let (_pattern, actual) = multi_match(r"[B-a]", "A");
    assert!(!actual.any_panicked());
    assert_eq!(actual, expect_multi(0, 1, 0, 1));
}

fn multi_match(pattern_text: &str, text: &str) -> (Pattern, MultiMatch) {
    let pattern = gix_glob::Pattern::from_bytes(pattern_text.as_bytes()).expect("valid (enough) pattern");
    let actual_path_match: MatchResult = catch_unwind(|| match_file_path(&pattern, text, Case::Sensitive)).into();
    let actual_path_imatch: MatchResult = catch_unwind(|| match_file_path(&pattern, text, Case::Fold)).into();
    let actual_glob_match: MatchResult =
        catch_unwind(|| gix_glob::wildmatch(pattern.text.as_bstr(), text.into(), wildmatch::Mode::empty())).into();
    let actual_glob_imatch: MatchResult =
        catch_unwind(|| gix_glob::wildmatch(pattern.text.as_bstr(), text.into(), wildmatch::Mode::IGNORE_CASE)).into();
    let actual = MultiMatch {
        path_match: actual_path_match,
        path_imatch: actual_path_imatch,
        glob_match: actual_glob_match,
        glob_imatch: actual_glob_imatch,
    };
    (pattern, actual)
}

fn expect_multi(path_match: u8, path_imatch: u8, glob_match: u8, glob_imatch: u8) -> MultiMatch {
    (path_match, path_imatch, glob_match, glob_imatch).into()
}

#[derive(Eq, PartialEq)]
struct MultiMatch {
    path_match: MatchResult,
    path_imatch: MatchResult,
    glob_match: MatchResult,
    glob_imatch: MatchResult,
}

impl MultiMatch {
    fn all_panicked(&self) -> bool {
        use MatchResult::Panic;
        matches!(self.path_match, Panic)
            && matches!(self.path_imatch, Panic)
            && matches!(self.glob_match, Panic)
            && matches!(self.glob_imatch, Panic)
    }
    fn any_panicked(&self) -> bool {
        use MatchResult::Panic;
        matches!(self.path_match, Panic)
            || matches!(self.path_imatch, Panic)
            || matches!(self.glob_match, Panic)
            || matches!(self.glob_imatch, Panic)
    }
}

impl From<(u8, u8, u8, u8)> for MultiMatch {
    fn from(t: (u8, u8, u8, u8)) -> Self {
        MultiMatch {
            path_match: t.0.into(),
            path_imatch: t.1.into(),
            glob_match: t.2.into(),
            glob_imatch: t.3.into(),
        }
    }
}

impl Debug for MultiMatch {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({} {} {} {})",
            self.path_match, self.path_imatch, self.glob_match, self.glob_imatch
        )
    }
}

enum MatchResult {
    Match,
    NoMatch,
    Panic,
}

impl PartialEq<Self> for MatchResult {
    fn eq(&self, other: &Self) -> bool {
        use MatchResult::*;
        match (self, other) {
            (Panic, _) | (_, Panic) => true,
            (Match, NoMatch) | (NoMatch, Match) => false,
            (Match, Match) | (NoMatch, NoMatch) => true,
        }
    }
}

impl std::cmp::Eq for MatchResult {}

impl From<std::thread::Result<bool>> for MatchResult {
    fn from(v: std::thread::Result<bool>) -> Self {
        use MatchResult::*;
        match v {
            Ok(v) if v => Match,
            Ok(_) => NoMatch,
            Err(_) => Panic,
        }
    }
}

impl From<u8> for MatchResult {
    fn from(v: u8) -> Self {
        use MatchResult::*;
        match v {
            1 => Match,
            0 => NoMatch,
            _ => unreachable!("BUG: only use 0 or 1 for expected values"),
        }
    }
}

impl Display for MatchResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use MatchResult::*;
        f.write_str(match self {
            Match => "✔️",
            NoMatch => "⨯",
            Panic => "E",
        })
    }
}

fn match_file_path(pattern: &gix_glob::Pattern, path: &str, case: Case) -> bool {
    pattern.matches_repo_relative_path(
        path.into(),
        basename_of(path),
        false.into(), /* is_dir */
        case,
        gix_glob::wildmatch::Mode::NO_MATCH_SLASH_LITERAL,
    )
}
fn basename_of(path: &str) -> Option<usize> {
    path.rfind('/').map(|pos| pos + 1)
}
