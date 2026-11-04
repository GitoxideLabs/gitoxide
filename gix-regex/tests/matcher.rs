// Derived from posix-regex 0.1.4, upstream revision
// 063882aa6051b5075062d20d00b41ddc0fb8cd89.
// Copyright (c) 2018 jD91mZM2. Licensed under the MIT license; see LICENSE-MIT.

extern crate alloc;

use gix_regex::PosixRegexBuilder;
use gix_regex::matcher::*;

// FIXME: Workaround to coerce a Box<[T; N]> into a Box<[T]>. Use type
// ascription when stabilized.
fn boxed_slice<T>(slice: Box<[T]>) -> Box<[T]> {
    slice
}

macro_rules! abox {
    ($($item:expr),*) => {
        boxed_slice(Box::new([$($item),*]))
    }
}

fn compile(regex: &str) -> PosixRegex<'static> {
    PosixRegexBuilder::new(regex.as_bytes())
        .with_default_classes()
        .compile()
        .expect("error compiling regex")
}
type Captures = Box<[Option<(usize, usize)>]>;

fn matches(regex: &str, input: &str) -> Vec<Captures> {
    compile(regex)
        .matches(input.as_bytes(), None)
        .expect("matching stays within the default resource limits")
}
fn matches_exact(regex: &str, input: &str) -> Option<Captures> {
    compile(regex)
        .matches_exact(input.as_bytes())
        .expect("matching stays within the default resource limits")
}

#[test]
fn basic() {
    assert!(matches_exact("abc", "abc").is_some());
    assert!(matches_exact("abc", "bbc").is_none());
    assert!(matches_exact("abc", "acc").is_none());
    assert!(matches_exact("abc", "abd").is_none());
}
#[test]
fn repetitions() {
    assert!(matches_exact("abc*", "ab").is_some());
    assert!(matches_exact("abc*", "abc").is_some());
    assert!(matches_exact("abc*", "abccc").is_some());

    assert!(matches_exact(r"a\{1,2\}b", "b").is_none());
    assert!(matches_exact(r"a\{1,2\}b", "ab").is_some());
    assert!(matches_exact(r"a\{1,2\}b", "aab").is_some());
    assert!(matches_exact(r"a\{1,2\}b", "aaab").is_none());

    assert!(matches_exact(r"[abc]\{3\}", "abcTRAILING").is_some());
    assert!(matches_exact(r"[abc]\{3\}", "abTRAILING").is_none());
}
#[test]
fn any() {
    assert!(matches_exact(".*", "").is_some());
    assert!(matches_exact(".*b", "b").is_some());
    assert!(matches_exact(".*b", "ab").is_some());
    assert!(matches_exact(".*b", "aaaaab").is_some());
    assert!(matches_exact(".*b", "HELLO WORLD").is_none());
    assert!(matches_exact(".*b", "HELLO WORLDb").is_some());
    assert!(matches_exact("H.*O WORLD", "HELLO WORLD").is_some());
    assert!(matches_exact("H.*ORLD", "HELLO WORLD").is_some());
}
#[test]
fn brackets() {
    assert!(matches_exact("[abc]*d", "abcd").is_some());
    assert!(matches_exact("[0-9]*d", "1234d").is_some());
    assert!(matches_exact("[[:digit:]]*d", "1234d").is_some());
    assert!(matches_exact("[[:digit:]]*d", "abcd").is_none());
}
#[test]
fn alternations() {
    assert!(matches_exact(r"abc\|bcd", "abc").is_some());
    assert!(matches_exact(r"abc\|bcd", "bcd").is_some());
    assert!(matches_exact(r"abc\|bcd", "cde").is_none());
    assert!(matches_exact(r"[A-Z]\+\|yee", "").is_none());
    assert!(matches_exact(r"[A-Z]\+\|yee", "HELLO").is_some());
    assert!(matches_exact(r"[A-Z]\+\|yee", "yee").is_some());
    assert!(matches_exact(r"[A-Z]\+\|yee", "hello").is_none());
}
#[test]
fn offsets() {
    assert_eq!(matches_exact("abc", "abcd"), Some(abox![Some((0, 3))]));
    assert_eq!(matches_exact(r"[[:alpha:]]\+", "abcde12345"), Some(abox![Some((0, 5))]));
    assert_eq!(
        matches_exact(r"a\(bc\)\+d", "abcbcd"),
        Some(abox![Some((0, 6)), Some((3, 5))])
    );
    assert_eq!(
        matches_exact(r"hello\( \(world\|universe\) :D\)\?!", "hello world :D!"),
        Some(abox![Some((0, 15)), Some((5, 14)), Some((6, 11))])
    );
    assert_eq!(
        matches_exact(r"hello\( \(world\|universe\) :D\)\?", "hello world :D"),
        Some(abox![Some((0, 14)), Some((5, 14)), Some((6, 11))])
    );
    assert_eq!(
        matches_exact(r"\(\<hello\>\) world", "hello world"),
        Some(abox![Some((0, 11)), Some((0, 5))])
    );
    assert_eq!(matches_exact(r".*d", "hid howd ared youd"), Some(abox![Some((0, 18))]));
    assert_eq!(
        matches_exact(r".*\(a\)", "bbbbba"),
        Some(abox![Some((0, 6)), Some((5, 6))])
    );
    assert_eq!(
        matches_exact(r"\(a \(b\) \(c\)\) \(d\)", "a b c d"),
        Some(abox![
            Some((0, 7)),
            Some((0, 5)),
            Some((2, 3)),
            Some((4, 5)),
            Some((6, 7))
        ])
    );
    assert_eq!(
        matches_exact(r"\(.\)*", "hello"),
        Some(abox![Some((0, 5)), Some((4, 5))])
    );
    assert_eq!(
        matches(r"h\(i\)", "hello hi lol"),
        vec![abox![Some((6, 8)), Some((7, 8))]]
    );
    assert_eq!(
        matches_exact(r"\(\([[:alpha:]]\)*\)", "abcdefg"),
        Some(abox![Some((0, 7)), Some((0, 7)), Some((6, 7))])
    );
    assert_eq!(
        matches_exact(r"\(\.\([[:alpha:]]\)\)*", ".a.b.c.d.e.f.g"),
        Some(abox![Some((0, 14)), Some((12, 14)), Some((13, 14))])
    );
    assert_eq!(
        matches_exact(r"\(a\|\(b\)\)*\(c\)", "bababac"),
        Some(abox![Some((0, 7)), Some((5, 6)), Some((4, 5)), Some((6, 7))])
    );
    assert_eq!(
        matches_exact(r"\(a\|\(b\)\)*\(c\)", "aaac"),
        Some(abox![Some((0, 4)), Some((2, 3)), None, Some((3, 4))])
    );
    assert_eq!(
        matches_exact(r"a\(\)bc", "abc"),
        Some(abox![Some((0, 3)), Some((1, 1))])
    );
}
#[test]
fn matches_is_lazy() {
    assert_eq!(
        matches(r"\(hi\)\+", "hello hihi kek"),
        vec![abox![Some((6, 10)), Some((8, 10))]]
    );
    assert_eq!(
        matches(r"o\+", "helloooooooo woooorld, hooow are you?"),
        vec![
            abox![Some((4, 12))],
            abox![Some((14, 18))],
            abox![Some((24, 27))],
            abox![Some((34, 35))]
        ]
    );
    assert_eq!(
        matches(r"z*", "abc"),
        vec![
            abox![Some((0, 0))],
            abox![Some((1, 1))],
            abox![Some((2, 2))],
            abox![Some((3, 3))]
        ]
    );
}
#[test]
fn start_and_end() {
    assert!(matches_exact("^abc$", "abc").is_some());
    assert!(matches_exact("^bcd", "bcde").is_some());
    assert!(matches_exact("^bcd", "abcd").is_none());
    assert!(matches_exact("abc$", "abc").is_some());
    assert!(matches_exact("abc$", "abcd").is_none());

    assert!(matches_exact(r".*\(^\|a\)c", "c").is_some());
    assert!(matches_exact(r".*\(^\|a\)c", "ac").is_some());
    assert!(matches_exact(r".*\(^\|a\)c", "bc").is_none());

    // In basic syntax, only the first character of a branch can be a start anchor.
    assert!(matches_exact(".*^^a", "helloabc").is_none());
    assert!(matches_exact(".*^^a", "abc").is_none());
    assert!(matches_exact(".*^^a", "^^abc").is_some());
}
#[test]
fn word_boundaries() {
    assert!(matches_exact(r"hello\>.world", "hello world").is_some());
    assert!(matches_exact(r"hello\>.world", "hello!world").is_some());
    assert!(matches_exact(r"hello\>.world", "hellooworld").is_none());

    assert!(matches_exact(r"hello.\<world", "hello world").is_some());
    assert!(matches_exact(r"hello.\<world", "hello!world").is_some());
    assert!(matches_exact(r"hello.\<world", "hellooworld").is_none());

    assert!(matches_exact(r".*\<hello\>", "hihello").is_none());
    assert!(matches_exact(r".*\<hello\>", "hi_hello").is_none());
    assert!(matches_exact(r".*\<hello\>", "hi hello").is_some());
}
#[test]
fn groups() {
    assert!(matches_exact(r"\(a*\)*", "aaaaa").is_some());
    assert!(matches_exact(r"\(hello\) world", "hello world").is_some());
    assert!(matches_exact(r"\(a*\|b\|c\)d", "d").is_some());
    assert!(matches_exact(r"\(a*\|b\|c\)d", "aaaad").is_some());
    assert!(matches_exact(r"\(a*\|b\|c\)d", "bd").is_some());
    assert!(matches_exact(r"\(a*\|b\|c\)d", "bbbbbd").is_none());
}
#[test]
fn repeating_groups() {
    assert!(matches_exact(r"\(a\|b\|c\)*d", "d").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)*d", "aaaad").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)*d", "bbbbd").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)*d", "aabbd").is_some());

    assert!(matches_exact(r"\(a\|b\|c\)\{1,2\}d", "d").is_none());
    assert!(matches_exact(r"\(a\|b\|c\)\{1,2\}d", "ad").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)\{1,2\}d", "abd").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)\{1,2\}d", "abcd").is_none());
    assert!(matches_exact(r"\(\(a\|b\|c\)\)\{1,2\}d", "abd").is_some());
    assert!(matches_exact(r"\(\(a\|b\|c\)\)\{1,2\}d", "abcd").is_none());
    assert!(matches_exact(r"\(\(a\|b\|c\)\{1,2\}\)\{1,2\}d", "abad").is_some());
    assert!(matches_exact(r"\(\(a\|b\|c\)\{1,2\}\)\{1,2\}d", "ababd").is_some());
    assert!(matches_exact(r"\(\(a\|b\|c\)\{1,2\}\)\{1,2\}d", "ababad").is_none());

    assert!(matches_exact(r"\(a\|b\|c\)\{4\}d", "ababad").is_none());
    assert!(matches_exact(r"\(a\|b\|c\)\{4\}d", "ababd").is_some());
    assert!(matches_exact(r"\(a\|b\|c\)\{4\}d", "abad").is_none());

    assert!(matches_exact(r"\(\([abc]\)\)\{3\}", "abcTRAILING").is_some());
    assert!(matches_exact(r"\(\([abc]\)\)\{3\}", "abTRAILING").is_none());
}
#[test]
fn backref() {
    assert!(matches_exact(r"\([abc]\)\1d", "aad").is_some());
    assert!(matches_exact(r"\([abc]\)\1d", "abd").is_none());
    assert!(matches_exact(r"\([abc]\{2,3\}\)\1d", "abcabcd").is_some());
    assert!(matches_exact(r"\([abc]\{2,3\}\)\1d", "abcbcd").is_none());
    assert!(matches_exact(r"\([abc]\{2,3\}\)\1d", "ababd").is_some());
    assert!(matches_exact(r"\([abc]\{2,3\}\)\1d", "abacd").is_none());

    assert!(matches_exact(r"\([[:alpha:]]\).*\1d", "hellohd").is_some());
    assert!(matches_exact(r"\([[:alpha:]]\).*\1d", "hellod").is_none());
    assert!(matches_exact(r"\([[:alpha:]]\).*\1", "hello").is_none());
    assert!(matches_exact(r"\([[:alpha:]]\).*\1", "helloh").is_some());

    assert!(matches_exact(r"\(\)-\?\1d", "d").is_some());
    assert!(matches_exact(r"\(\)-\?\1", "").is_some());

    assert!(
        PosixRegexBuilder::new(br"\(\1\)").compile().is_err(),
        "a backreference to an open group is rejected during compilation"
    );

    assert!(matches_exact(r"\(h.\)\1\+!", "hihihi!").is_some());
    assert!(matches_exact(r"\(h.\)\1\+!", "hehehe!").is_some());
    assert!(matches_exact(r"\(h.\)\1\+!", "hahehe!").is_none());

    assert!(matches_exact(r"\(hello \(\<.*\>\) \)*how are you \2", "hello world how are you world").is_some());
    assert!(
        matches_exact(
            r"\(hello \(\<.*\>\) \)*how are you \2",
            "hello universe hello world how are you world"
        )
        .is_some()
    );
    assert!(
        matches_exact(
            r"\(hello \(\<.*\>\) \)*how are you \2",
            "hello world hello universe how are you world"
        )
        .is_none()
    );
}
#[test]
fn case_insensitive() {
    assert!(
        compile(r"abc[de]")
            .case_insensitive(true)
            .matches_exact(b"ABCD")
            .expect("matching stays within the default resource limits")
            .is_some()
    );
    assert!(
        compile(r"abc[de]")
            .case_insensitive(true)
            .matches_exact(b"ABCF")
            .expect("matching stays within the default resource limits")
            .is_none()
    );
}
#[test]
fn newline() {
    assert_eq!(
        compile(r"^hello$")
            .newline(true)
            .matches(b"hi\nhello\ngreetings", None)
            .expect("matching stays within the default resource limits")
            .len(),
        1
    );
    assert!(
        compile(r"^hello$")
            .newline(true)
            .matches(b"hi\ngood day\ngreetings", None)
            .expect("matching stays within the default resource limits")
            .is_empty()
    );
}
#[test]
fn no_start_end() {
    assert!(
        compile(r"^hello")
            .no_start(true)
            .matches_exact(b"hello")
            .expect("matching stays within the default resource limits")
            .is_none()
    );
    assert!(
        compile(r"hello$")
            .no_end(true)
            .matches_exact(b"hello")
            .expect("matching stays within the default resource limits")
            .is_none()
    );
}
