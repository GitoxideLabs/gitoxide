use crate::Result;
use bstr::{BStr, ByteSlice};
use gix_glob::pattern::Case;
use gix_ignore::search::Match;

struct Expectations<'a> {
    lines: bstr::Lines<'a>,
}

impl<'a> Iterator for Expectations<'a> {
    type Item = (&'a BStr, Option<(&'a BStr, usize, &'a BStr)>);

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.lines.next()?;
        let (left, value) = line.split_at(line.find_byte(b'\t').unwrap());
        let value = value[1..].as_bstr();

        let source_and_line = if left == b"::" {
            None
        } else {
            let mut tokens = left.split(|b| *b == b':');
            let source = tokens.next().unwrap().as_bstr();
            let line_number: usize = tokens.next().unwrap().to_str_lossy().parse().ok().unwrap();
            let pattern = tokens.next().unwrap().as_bstr();
            Some((source, line_number, pattern))
        };
        Some((value, source_and_line))
    }
}

#[test]
fn baseline_from_git_dir() -> Result {
    for repo_name in [
        "repo",
        "slash-and-excludes",
        "star-and-excludes-in-subdir",
        "slash-and-excludes-in-subdir",
    ] {
        let case = if gix_fs::Capabilities::probe("../.git".as_ref()).ignore_case {
            Case::Fold
        } else {
            Case::Sensitive
        };
        let dir = gix_testtools::scripted_fixture_read_only("make_global_and_external_and_dir_ignores.sh")?;
        let repo_dir = dir.join(repo_name);
        let git_dir = repo_dir.join(".git");
        let baseline = std::fs::read(git_dir.parent().unwrap().join("git-check-ignore.baseline"))?;
        let mut buf = Vec::new();
        let user_exclude = dir.join("user.exclude");
        let mut group = gix_ignore::Search::from_git_dir(
            &git_dir,
            user_exclude.is_file().then_some(user_exclude),
            &mut buf,
            Default::default(),
        )?;

        assert!(
            !gix_glob::search::add_patterns_file(
                &mut group.patterns,
                "not-a-file".into(),
                false,
                None,
                &mut buf,
                Default::default()
            )?,
            "missing files are no problem and cause a negative response"
        );
        let mut ignore_file = repo_dir.join(".gitignore");
        if !ignore_file.is_file() {
            ignore_file.pop();
            ignore_file.push("sub/.gitignore");
        }
        assert!(
            gix_glob::search::add_patterns_file(
                &mut group.patterns,
                ignore_file,
                true,
                repo_dir.as_path().into(),
                &mut buf,
                Default::default()
            )?,
            "existing files return true"
        );

        let ignore_file = repo_dir.join("dir-with-ignore").join(".gitignore");
        if ignore_file.is_file() {
            let buf = std::fs::read(&ignore_file)?;
            group.add_patterns_buffer(&buf, ignore_file, repo_dir.as_path().into(), Default::default());
        }

        for (path, source_and_line) in (Expectations {
            lines: baseline.lines(),
        }) {
            let actual = group.pattern_matching_relative_path(
                path,
                repo_dir
                    .join(path.to_str_lossy().as_ref())
                    .metadata()
                    .ok()
                    .map(|m| m.is_dir()),
                case,
            );
            match (actual, source_and_line) {
                (
                    Some(Match {
                        sequence_number,
                        pattern: _,
                        source,
                        kind: gix_ignore::Kind::Expendable,
                    }),
                    Some((expected_source, line, _expected_pattern)),
                ) => {
                    assert_eq!(sequence_number, line, "our counting should match the one used in git");
                    assert_eq!(
                        source.map(|p| p.canonicalize().unwrap()),
                        Some(repo_dir.join(expected_source.to_str_lossy().as_ref()).canonicalize()?)
                    );
                }
                (None, None) => {}
                (actual, expected) => {
                    panic!("{repo_name}: {case:?}: actual {actual:?} should match {expected:?} with path '{path}'")
                }
            }
        }
    }
    Ok(())
}

#[test]
fn from_overrides_with_precious() {
    let input = ["$s?mple", "pattern/"];
    let group = gix_ignore::Search::from_overrides(input.iter(), gix_ignore::search::Ignore { support_precious: true });

    assert_eq!(
        group.pattern_matching_relative_path("Simple".into(), None, gix_glob::pattern::Case::Fold),
        Some(pattern_to_match(
            &gix_glob::parse("s?mple").unwrap(),
            1,
            gix_ignore::Kind::Precious
        )),
        ""
    );
}

#[test]
fn from_overrides_with_excludes() {
    let group = gix_ignore::Search::from_overrides(["$simple", "!simple", "pattern/"], Default::default());
    assert_eq!(
        group.pattern_matching_relative_path("Simple".into(), None, gix_glob::pattern::Case::Fold),
        Some(pattern_to_match(
            &gix_glob::parse("!simple").unwrap(),
            2,
            gix_ignore::Kind::Expendable
        )),
        "Now the negative pattern matches - the sequence numbers are 1-based"
    );
}

#[test]
fn from_overrides() {
    let group = gix_ignore::Search::from_overrides(["simple", "pattern/"], Default::default());
    assert_eq!(
        group.pattern_matching_relative_path("Simple".into(), None, gix_glob::pattern::Case::Fold),
        Some(pattern_to_match(
            &gix_glob::parse("simple").unwrap(),
            1,
            gix_ignore::Kind::Expendable
        ))
    );
    assert_eq!(
        group.pattern_matching_relative_path("pattern".into(), Some(true), gix_glob::pattern::Case::Sensitive),
        Some(pattern_to_match(
            &gix_glob::parse("pattern/").unwrap(),
            2,
            gix_ignore::Kind::Expendable
        ))
    );
    assert_eq!(group.patterns.len(), 1);
    assert_eq!(
        gix_ignore::Search::from_overrides(["simple", "pattern/"], Default::default()).patterns[0],
        group.patterns.into_iter().next().unwrap()
    );
}

fn pattern_to_match(pattern: &gix_glob::Pattern, sequence_number: usize, kind: gix_ignore::Kind) -> Match<'_> {
    Match {
        pattern,
        source: None,
        sequence_number,
        kind,
    }
}
