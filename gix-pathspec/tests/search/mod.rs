use std::path::Path;

use bstr::BStr;
use gix_pathspec::search::MatchKind::*;

#[cfg(feature = "parallel")]
#[test]
fn is_send_with_parallel_enabled() {
    fn assert_send<T: Send>() {}
    assert_send::<gix_pathspec::Search>();
}

#[test]
fn directories() -> crate::Result {
    baseline::run("directory", true, baseline::directories)
}

#[test]
fn directory_matches_prefix() -> crate::Result {
    for spec in ["dir", "dir/", "di*", "dir/*", "dir/*.o"] {
        for specs in [&[spec] as &[_], &[spec, "other"]] {
            let search = gix_pathspec::Search::from_specs(pathspecs(specs), None, Path::new(""))?;
            assert!(
                search.directory_matches_prefix("dir".into(), false),
                "{spec}: must match"
            );
            assert!(
                !search.directory_matches_prefix("d".into(), false),
                "{spec}: must not match"
            );
        }
    }

    for spec in ["dir/d", "dir/d/", "dir/*/*", "dir/d/*.o"] {
        for specs in [&[spec] as &[_], &[spec, "other"]] {
            let search = gix_pathspec::Search::from_specs(pathspecs(specs), None, Path::new(""))?;
            assert!(
                search.directory_matches_prefix("dir/d".into(), false),
                "{spec}: must match"
            );
            assert!(
                search.directory_matches_prefix("dir/d".into(), true),
                "{spec}: must match"
            );
            for leading in [false, true] {
                assert!(
                    !search.directory_matches_prefix("d".into(), leading),
                    "{spec}: must not match"
                );
                assert!(
                    !search.directory_matches_prefix("di".into(), leading),
                    "{spec}: must not match"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn directory_matches_prefix_starting_wildcards_always_match() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["*ir"]), None, Path::new(""))?;
    assert!(search.directory_matches_prefix("dir".into(), false));
    assert!(search.directory_matches_prefix("d".into(), false));
    Ok(())
}

#[test]
fn empty_dir_always_matches() -> crate::Result {
    for specs in [
        &["*ir"] as &[_],
        &[],
        &["included", ":!excluded"],
        &[":!all", ":!excluded"],
    ] {
        let mut search = gix_pathspec::Search::from_specs(pathspecs(specs), None, Path::new(""))?;
        assert_eq!(
            search
                .pattern_matching_relative_path("".into(), None, &mut no_attrs)
                .map(|m| m.kind),
            Some(Always),
            "{specs:?}"
        );
        assert!(search.directory_matches_prefix("".into(), false));
        assert!(search.directory_matches_prefix("".into(), false));
        for is_dir in [Some(true), Some(false), None] {
            assert!(search.can_match_relative_path("".into(), is_dir));
        }
    }
    Ok(())
}

#[test]
fn directory_matches_prefix_leading() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["d/d/generated/b"]), None, Path::new(""))?;
    assert!(!search.directory_matches_prefix("di".into(), false));
    assert!(!search.directory_matches_prefix("di".into(), true));
    assert!(search.directory_matches_prefix("d".into(), true));
    assert!(!search.directory_matches_prefix("d".into(), false));
    assert!(search.directory_matches_prefix("d/d".into(), true));
    assert!(!search.directory_matches_prefix("d/d".into(), false));
    assert!(search.directory_matches_prefix("d/d/generated".into(), true));
    assert!(!search.directory_matches_prefix("d/d/generated".into(), false));
    assert!(!search.directory_matches_prefix("d/d/generatedfoo".into(), false));
    assert!(!search.directory_matches_prefix("d/d/generatedfoo".into(), true));

    let search = gix_pathspec::Search::from_specs(pathspecs(&[":(icase)d/d/GENERATED/b"]), None, Path::new(""))?;
    assert!(
        search.directory_matches_prefix("d/d/generated".into(), true),
        "icase is respected as well"
    );
    assert!(!search.directory_matches_prefix("d/d/generated".into(), false));
    Ok(())
}

#[test]
fn directory_matches_prefix_negative_wildcard() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&[":!*generated*"]), None, Path::new(""))?;
    assert!(
        search.directory_matches_prefix("di".into(), false),
        "it's always considered matching, we can't really tell anyway"
    );
    assert!(search.directory_matches_prefix("di".into(), true));
    assert!(search.directory_matches_prefix("d".into(), true));
    assert!(search.directory_matches_prefix("d".into(), false));
    assert!(search.directory_matches_prefix("d/d".into(), true));
    assert!(search.directory_matches_prefix("d/d".into(), false));
    assert!(search.directory_matches_prefix("d/d/generated".into(), true));
    assert!(search.directory_matches_prefix("d/d/generated".into(), false));
    assert!(search.directory_matches_prefix("d/d/generatedfoo".into(), false));
    assert!(search.directory_matches_prefix("d/d/generatedfoo".into(), true));

    let search = gix_pathspec::Search::from_specs(pathspecs(&[":(exclude,icase)*GENERATED*"]), None, Path::new(""))?;
    assert!(search.directory_matches_prefix("d/d/generated".into(), true));
    assert!(search.directory_matches_prefix("d/d/generated".into(), false));
    Ok(())
}

#[test]
fn directory_matches_prefix_all_excluded() -> crate::Result {
    for spec in ["!dir", "!dir/", "!d*", "!di*", "!dir/*", "!dir/*.o", "!*ir"] {
        for specs in [&[spec] as &[_], &[spec, "other"]] {
            let search = gix_pathspec::Search::from_specs(pathspecs(specs), None, Path::new(""))?;
            assert!(
                !search.directory_matches_prefix("dir".into(), false),
                "{spec}: must not match, it's excluded"
            );
        }
    }
    Ok(())
}

#[test]
fn no_pathspecs_match_everything() -> crate::Result {
    let mut search = gix_pathspec::Search::from_specs([], None, Path::new(""))?;
    assert_eq!(search.patterns().count(), 0, "nothing artificial is added");
    let m = search
        .pattern_matching_relative_path("hello".into(), None, &mut no_attrs)
        .expect("matches");
    assert_eq!(m.pattern.prefix_directory(), "", "there is no prefix as none was given");
    assert_eq!(m.kind, Always, "no pathspec always matches");
    assert_eq!(
        m.sequence_number, 0,
        "this is actually a fake pattern, as we have to match even though there isn't anything"
    );
    assert!(search.can_match_relative_path("anything".into(), None));
    assert!(search.directory_matches_prefix("anything".into(), false));
    Ok(())
}

#[test]
fn included_directory_and_excluded_subdir_top_level_with_prefix() -> crate::Result {
    let mut search = gix_pathspec::Search::from_specs(pathspecs(&[":/foo", ":!/foo/target/"]), None, Path::new("foo"))?;
    let m = search
        .pattern_matching_relative_path("foo".into(), Some(true), &mut no_attrs)
        .expect("matches");
    assert_eq!(m.kind, Verbatim);

    let m = search
        .pattern_matching_relative_path("foo/bar".into(), Some(false), &mut no_attrs)
        .expect("matches");
    assert_eq!(m.kind, Prefix);

    let m = search
        .pattern_matching_relative_path("foo/target".into(), Some(false), &mut no_attrs)
        .expect("matches");
    assert_eq!(m.kind, Prefix, "files named `target` are allowed");

    let m = search
        .pattern_matching_relative_path("foo/target".into(), Some(true), &mut no_attrs)
        .expect("matches");
    assert!(m.is_excluded(), "directories named `target` are excluded");
    assert_eq!(m.kind, Verbatim);

    let m = search
        .pattern_matching_relative_path("foo/target/file".into(), Some(false), &mut no_attrs)
        .expect("matches");
    assert!(m.is_excluded(), "everything below `target/` is also excluded");
    assert_eq!(m.kind, Prefix);

    assert!(search.directory_matches_prefix("foo/bar".into(), false));
    assert!(search.directory_matches_prefix("foo/bar".into(), true));
    assert!(search.directory_matches_prefix("foo".into(), false));
    assert!(search.directory_matches_prefix("foo".into(), true));
    assert!(search.can_match_relative_path("foo".into(), Some(true)));
    assert!(search.can_match_relative_path("foo".into(), Some(false)));
    assert!(search.can_match_relative_path("foo/hi".into(), Some(true)));
    assert!(search.can_match_relative_path("foo/hi".into(), Some(false)));
    Ok(())
}

#[test]
fn starts_with() -> crate::Result {
    let mut search = gix_pathspec::Search::from_specs(pathspecs(&["a/*"]), None, Path::new(""))?;
    assert!(
        search
            .pattern_matching_relative_path("a".into(), Some(false), &mut no_attrs)
            .is_none(),
        "this can only match if it's a directory"
    );
    assert!(
        search
            .pattern_matching_relative_path("a".into(), Some(true), &mut no_attrs)
            .is_none(),
        "can't match as the '*' part is missing in value"
    );
    assert!(
        search.can_match_relative_path("a".into(), Some(true)),
        "prefix-matches work though"
    );
    assert!(
        search.can_match_relative_path("a".into(), Some(false)),
        "but not if it's a file"
    );
    assert!(
        search.can_match_relative_path("a".into(), None),
        "if unspecified, we match for good measure"
    );
    assert!(search.directory_matches_prefix("a".into(), false));
    assert!(!search.directory_matches_prefix("ab".into(), false));
    assert_eq!(
        search
            .pattern_matching_relative_path("a/file".into(), None, &mut no_attrs)
            .expect("matches")
            .kind,
        WildcardMatch,
        "a wildmatch is always performed here, even though it looks like a prefix"
    );
    Ok(())
}

#[test]
fn simplified_search_respects_must_be_dir() -> crate::Result {
    let mut search = gix_pathspec::Search::from_specs(pathspecs(&["a/be/"]), None, Path::new(""))?;
    assert_eq!(
        search
            .pattern_matching_relative_path("a/be/file".into(), Some(false), &mut no_attrs)
            .expect("matches as this is a prefix match")
            .kind,
        Prefix,
        "a verbatim part of the spec matches"
    );
    assert!(
        !search.can_match_relative_path("any".into(), Some(false)),
        "not our directory: a, and must be dir"
    );
    assert!(
        !search.can_match_relative_path("any".into(), Some(true)),
        "not our directory: a"
    );
    assert!(
        !search.can_match_relative_path("any".into(), None),
        "not our directory: a, and must be dir, still completely out of scope"
    );
    assert!(
        !search.can_match_relative_path("a/bei".into(), None),
        "not our directory: a/be"
    );
    assert!(!search.can_match_relative_path("a".into(), Some(false)), "must be dir");
    assert!(search.can_match_relative_path("a".into(), Some(true)));
    assert!(
        search.can_match_relative_path("a".into(), None),
        "now dir or not doesn't matter"
    );
    assert!(search.can_match_relative_path("a/be".into(), Some(true)));
    assert!(
        search.can_match_relative_path("a/be".into(), None),
        "dir doesn't matter anymore"
    );
    assert!(
        !search.can_match_relative_path("a/be".into(), Some(false)),
        "files can't match as prefix"
    );
    assert!(
        search.can_match_relative_path("a/be/file".into(), Some(false)),
        "files can match if they are part of the suffix"
    );

    assert!(
        !search.can_match_relative_path("a/b".into(), Some(false)),
        "can't match a/be"
    );
    assert!(
        !search.can_match_relative_path("a/b".into(), None),
        "still can't match a/be"
    );
    assert!(
        search
            .pattern_matching_relative_path("a/b".into(), None, &mut no_attrs)
            .is_none(),
        "no match if it's not the whole pattern that matches"
    );
    assert!(
        !search.can_match_relative_path("a/b".into(), Some(true)),
        "can't match a/be, which must be directory"
    );

    Ok(())
}

#[test]
fn simplified_search_respects_ignore_case() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&[":(icase)foo/**/bar"]), None, Path::new(""))?;
    assert!(search.can_match_relative_path("Foo".into(), None));
    assert!(search.can_match_relative_path("foo".into(), Some(true)));
    assert!(search.can_match_relative_path("FOO/".into(), Some(true)));

    Ok(())
}

#[test]
fn simplified_search_respects_all_excluded() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(
        pathspecs(&[":(exclude)a/file", ":(exclude)b/file"]),
        None,
        Path::new(""),
    )?;
    assert!(
        search.can_match_relative_path("b".into(), None),
        "non-trivial excludes are ignored in favor of false-positives"
    );
    assert!(
        search.can_match_relative_path("a".into(), None),
        "non-trivial excludes are ignored in favor of false-positives"
    );
    assert!(search.can_match_relative_path("c".into(), None));
    assert!(search.can_match_relative_path("c/".into(), None));

    Ok(())
}

#[test]
fn simplified_search_wildcards() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["**/a*"]), None, Path::new(""))?;
    assert!(
        search.can_match_relative_path("a".into(), None),
        "it can't determine it, so assume match"
    );
    assert!(search.can_match_relative_path("a/a".into(), Some(false)));
    assert!(search.can_match_relative_path("a/a.o".into(), Some(false)));
    assert!(
        search.can_match_relative_path("b-unrelated".into(), None),
        "this is also assumed to be a match, prefer false-positives over false-negatives"
    );
    Ok(())
}

#[test]
fn simplified_search_wildcards_simple() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["dir/*"]), None, Path::new(""))?;
    for is_dir in [None, Some(false), Some(true)] {
        assert!(
            !search.can_match_relative_path("a".into(), is_dir),
            "definitely out of bound"
        );
        assert!(
            !search.can_match_relative_path("di".into(), is_dir),
            "prefix is not enough"
        );
        assert!(
            search.can_match_relative_path("dir".into(), is_dir),
            "directories can match"
        );
        assert!(
            search.can_match_relative_path("dir/file".into(), is_dir),
            "substrings can also match"
        );
    }

    Ok(())
}

#[test]
fn simplified_search_handles_nil() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&[":"]), None, Path::new(""))?;
    assert!(search.can_match_relative_path("a".into(), None), "everything matches");
    assert!(search.can_match_relative_path("a".into(), Some(false)));
    assert!(search.can_match_relative_path("a".into(), Some(true)));
    assert!(search.can_match_relative_path("a/b".into(), Some(true)));

    let search = gix_pathspec::Search::from_specs(pathspecs(&[":(exclude)"]), None, Path::new(""))?;
    assert!(
        !search.can_match_relative_path("a".into(), None),
        "everything does not match"
    );
    assert!(!search.can_match_relative_path("a".into(), Some(false)));
    assert!(!search.can_match_relative_path("a".into(), Some(true)));
    assert!(!search.can_match_relative_path("a/b".into(), Some(true)));

    Ok(())
}

#[test]
fn longest_common_directory_no_prefix() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["tests/a/", "tests/b/", ":!*.sh"]), None, Path::new(""))?;
    assert_eq!(search.common_prefix(), "tests/");
    assert_eq!(search.prefix_directory(), Path::new(""));
    assert_eq!(
        search.longest_common_directory().expect("present").to_string_lossy(),
        "tests/",
        "trailing slashes are not stripped"
    );
    Ok(())
}

#[test]
fn longest_common_directory_with_prefix() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(
        pathspecs(&["tests/a/", "tests/b/", ":!*.sh"]),
        Some(Path::new("a/b")),
        Path::new(""),
    )?;
    assert_eq!(search.common_prefix(), "a/b/tests/");
    assert_eq!(
        search.prefix_directory().to_string_lossy(),
        "a/b",
        "trailing slashes are not contained"
    );
    assert_eq!(
        search.longest_common_directory().expect("present").to_string_lossy(),
        "a/b/tests/",
        "trailing slashes are present, they don't matter"
    );
    Ok(())
}

#[test]
fn init_with_exclude() -> crate::Result {
    let search = gix_pathspec::Search::from_specs(pathspecs(&["tests/", ":!*.sh"]), None, Path::new(""))?;
    assert_eq!(search.patterns().count(), 2, "nothing artificial is added");
    assert!(
        search.patterns().next().expect("first of two").is_excluded(),
        "re-ordered so that excluded are first"
    );
    assert_eq!(search.common_prefix(), "tests");
    assert_eq!(
        search.prefix_directory(),
        Path::new(""),
        "there was no prefix during initialization"
    );
    assert_eq!(
        search.longest_common_directory(),
        Some(Path::new("tests").into()),
        "but this works here, and it should be tested"
    );
    assert!(
        search.can_match_relative_path("tests".into(), Some(true)),
        "prefix matches"
    );
    assert!(
        !search.can_match_relative_path("test".into(), Some(true)),
        "prefix can not be shorter"
    );
    assert!(!search.can_match_relative_path("outside-of-tests".into(), None));
    Ok(())
}

#[test]
fn no_pathspecs_respect_prefix() -> crate::Result {
    let mut search = gix_pathspec::Search::from_specs([], Some(Path::new("a")), Path::new(""))?;
    assert_eq!(
        search.patterns().count(),
        1,
        "we get an artificial pattern to get the prefix"
    );
    assert!(
        search
            .pattern_matching_relative_path("hello".into(), None, &mut no_attrs)
            .is_none(),
        "not the right prefix"
    );
    assert!(!search.can_match_relative_path("hello".into(), None));
    let m = search
        .pattern_matching_relative_path("a/b".into(), None, &mut no_attrs)
        .expect("match");
    assert_eq!(
        m.pattern.prefix_directory(),
        "a",
        "the prefix directory matched verbatim"
    );
    assert_eq!(m.kind, Prefix, "the common path also works like a prefix");
    assert!(search.can_match_relative_path("a/".into(), Some(true)));
    assert!(search.can_match_relative_path("a".into(), Some(true)));
    assert!(!search.can_match_relative_path("a".into(), Some(false)));
    assert!(search.can_match_relative_path("a".into(), None), "simple prefix search");

    Ok(())
}

#[test]
fn prefixes_are_always_case_sensitive() -> crate::Result {
    let path = gix_testtools::scripted_fixture_read_only("match_baseline_files.sh")?.join("paths");
    let items = baseline::parse_paths(path)?;

    for (spec, prefix, common_prefix, expected, expected_common_dir) in [
        (
            ":(icase)bar",
            "FOO",
            "FOO",
            &["FOO/BAR", "FOO/bAr", "FOO/bar"] as &[_],
            "FOO",
        ),
        (":(icase)bar", "F", "F", &[], "F"),
        (":(icase)bar", "FO", "FO", &[], "FO"),
        (":(icase)../bar", "fOo", "", &["BAR", "bAr", "bar"], ""),
        ("../bar", "fOo", "bar", &["bar"], ""),
        ("    ", "", "    ", &["    "], ""),    // whitespace can match verbatim
        ("  hi*", "", "  hi", &["  hi  "], ""), // whitespace can match with globs as well
        (":(icase)../bar", "fO", "", &["BAR", "bAr", "bar"], ""), // prefixes are virtual, and don't have to exist at all.
        (
            ":(icase)../foo/bar",
            "FOO",
            "",
            &[
                "FOO/BAR", "FOO/bAr", "FOO/bar", "fOo/BAR", "fOo/bAr", "fOo/bar", "foo/BAR", "foo/bAr", "foo/bar",
            ],
            "",
        ),
        ("../foo/bar", "FOO", "foo/bar", &["foo/bar"], ""),
        (
            ":(icase)../foo/../fOo/bar",
            "FOO",
            "",
            &[
                "FOO/BAR", "FOO/bAr", "FOO/bar", "fOo/BAR", "fOo/bAr", "fOo/bar", "foo/BAR", "foo/bAr", "foo/bar",
            ],
            "",
        ),
        ("../foo/../fOo/BAR", "FOO", "fOo/BAR", &["fOo/BAR"], ""),
    ] {
        let mut search = gix_pathspec::Search::from_specs(
            gix_pathspec::parse(spec.as_bytes(), Default::default()),
            Some(Path::new(prefix)),
            Path::new(""),
        )?;
        assert_eq!(search.common_prefix(), common_prefix, "{spec} {prefix}");
        assert_eq!(search.prefix_directory(), Path::new(expected_common_dir));
        let actual: Vec<_> = items
            .iter()
            .filter(|relative_path| {
                search
                    .pattern_matching_relative_path(relative_path.as_str().into(), Some(false), &mut no_attrs)
                    .is_some()
            })
            .collect();
        assert_eq!(actual, expected, "{spec} {prefix}");
    }

    let search = gix_pathspec::Search::from_specs(
        gix_pathspec::parse(":(icase)bar".as_bytes(), Default::default()),
        Some(Path::new("FOO")),
        Path::new(""),
    )?;
    assert!(
        !search.can_match_relative_path("foo".into(), Some(true)),
        "icase does not apply to the prefix"
    );
    assert!(search.can_match_relative_path("FOO".into(), Some(true)));
    assert!(
        !search.can_match_relative_path("FOO/ba".into(), Some(true)),
        "a full match is needed"
    );
    assert!(search.can_match_relative_path("FOO/bar".into(), Some(true)));
    Ok(())
}

#[test]
fn common_prefix() -> crate::Result {
    for (specs, prefix, expected_common_prefix, expected_common_dir) in [
        (&["foo/bar", ":(icase)foo/bar"] as &[_], None, "", ""),
        (&["foo/bar", "foo"], None, "foo", ""),
        (&["foo/bar/baz", "foo/bar/"], None, "foo/bar", ""), // directory trailing slashes are ignored, but that prefix shouldn't care anyway
        (&[":(icase)bar", ":(icase)bart"], Some("foo"), "foo", "foo"), // only case-sensitive portions count
        (&["bar", "bart"], Some("foo"), "foo/bar", "foo"),   // otherwise everything that matches counts
        (&["bar", "bart", "ba"], Some("foo"), "foo/ba", "foo"),
    ] {
        let search = gix_pathspec::Search::from_specs(
            specs
                .iter()
                .map(|s| gix_pathspec::parse(s.as_bytes(), Default::default()).expect("valid")),
            prefix.map(Path::new),
            Path::new(""),
        )?;
        assert_eq!(search.common_prefix(), expected_common_prefix, "{specs:?} {prefix:?}");
        assert_eq!(
            search.prefix_directory(),
            Path::new(expected_common_dir),
            "{specs:?} {prefix:?}"
        );
    }
    Ok(())
}

#[test]
fn files() -> crate::Result {
    baseline::run("file", false, baseline::files)
}

fn pathspecs(input: &[&str]) -> Vec<gix_pathspec::Pattern> {
    input
        .iter()
        .map(|pattern| gix_pathspec::parse(pattern.as_bytes(), Default::default()).expect("known to be valid"))
        .collect()
}

mod baseline {
    use std::path::{Path, PathBuf};

    use bstr::{BString, ByteSlice};

    pub fn run(
        name: &str,
        items_are_dirs: bool,
        init: impl FnOnce() -> crate::Result<(PathBuf, Vec<String>, Vec<Expected>)>,
    ) -> crate::Result {
        let (root, items, expected) = init()?;
        let mut collection = Default::default();
        let attrs =
            gix_attributes::Search::new_globals(Some(root.join(".gitattributes")), &mut Vec::new(), &mut collection)?;
        let tests = expected.len();
        for expected in expected {
            let mut search = gix_pathspec::Search::from_specs(expected.pathspecs, None, Path::new(""))?;
            let actual: Vec<_> = items
                .iter()
                .filter(|path| {
                    search
                        .pattern_matching_relative_path(
                            path.as_str().into(),
                            Some(items_are_dirs),
                            &mut |rela_path, case, is_dir, out| {
                                out.initialize(&collection);
                                attrs.pattern_matching_relative_path(rela_path, case, Some(is_dir), out)
                            },
                        )
                        .is_some_and(|m| !m.is_excluded())
                })
                .cloned()
                .collect();
            let matches_expectation = actual == expected.matches;
            assert_eq!(
                matches_expectation,
                expected.is_consistent,
                "{} - {actual:?} == {:?}",
                search.patterns().map(|p| format!("{p}")).collect::<Vec<_>>().join(", "),
                expected.matches
            );
        }
        eprintln!("{tests} {name} matches OK");
        Ok(())
    }

    #[derive(Debug)]
    pub struct Expected {
        pub pathspecs: Vec<gix_pathspec::Pattern>,
        pub matches: Vec<String>,
        /// If true, this means that the baseline is different from what we get, and that our solution is consistent with the rules.
        pub is_consistent: bool,
    }

    pub fn parse_paths(path: PathBuf) -> std::io::Result<Vec<String>> {
        let buf = std::fs::read(path)?;
        Ok(buf.lines().map(BString::from).map(|s| s.to_string()).collect())
    }

    fn parse_blocks(input: &[u8], parse_matches: impl Fn(&[u8]) -> Vec<String>) -> Vec<Expected> {
        input
            .split(|b| *b == b';')
            .filter(|b| !b.is_empty())
            .map(move |block| {
                let mut lines = block.lines();
                let mut is_inconsistent = false;
                let pathspecs = lines
                    .next()
                    .expect("pathspec")
                    .split(|b| *b == b'+')
                    .filter(|spec| {
                        is_inconsistent = spec.as_bstr() == "git-inconsistency";
                        !is_inconsistent
                    })
                    .filter(|s| !s.trim().is_empty())
                    .map(|pathspec| gix_pathspec::parse(pathspec.trim(), Default::default()).expect("valid pathspec"))
                    .collect();
                Expected {
                    pathspecs,
                    matches: parse_matches(lines.as_bytes()),
                    is_consistent: !is_inconsistent,
                }
            })
            .collect()
    }

    mod submodule {
        use bstr::ByteSlice;

        pub fn matches_from_status(input: &[u8]) -> impl Iterator<Item = (bool, String)> + '_ {
            input.lines().map(|line| {
                let matches = line[0] == b' ';
                assert_eq!(!matches, line[0] == b'-');
                let mut tokens = line[1..].split(|b| *b == b' ').skip(1);
                let path = tokens.next().expect("path").to_str().expect("valid UTF-8");
                (matches, path.to_owned())
            })
        }

        pub fn parse_expected(input: &[u8]) -> Vec<super::Expected> {
            super::parse_blocks(input, |block| {
                matches_from_status(block)
                    .filter_map(|(matches, module_path)| matches.then_some(module_path))
                    .collect()
            })
        }
    }

    mod files {
        use bstr::{BString, ByteSlice};
        pub fn parse_expected(input: &[u8]) -> Vec<super::Expected> {
            super::parse_blocks(input, |block| {
                block.lines().map(BString::from).map(|s| s.to_string()).collect()
            })
        }
    }

    pub fn directories() -> crate::Result<(PathBuf, Vec<String>, Vec<Expected>)> {
        let root = gix_testtools::scripted_fixture_read_only("match_baseline_dirs.sh")?.join("parent");
        let buf = std::fs::read(root.join("paths"))?;
        let items = submodule::matches_from_status(&buf)
            .map(|(_matches, path)| path)
            .collect();
        let expected = submodule::parse_expected(&std::fs::read(root.join("baseline.git"))?);
        Ok((root, items, expected))
    }

    pub fn files() -> crate::Result<(PathBuf, Vec<String>, Vec<Expected>)> {
        let root = gix_testtools::scripted_fixture_read_only("match_baseline_files.sh")?;
        let items = parse_paths(root.join("paths"))?;
        let expected = files::parse_expected(&std::fs::read(root.join("baseline.git"))?);
        Ok((root, items, expected))
    }
}

fn no_attrs(_: &BStr, _: gix_glob::pattern::Case, _: bool, _: &mut gix_attributes::search::Outcome) -> bool {
    unreachable!("must not be called")
}
