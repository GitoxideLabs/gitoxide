use gix_testtools::TestResult;
use std::fs::Metadata;

use bstr::{BStr, ByteSlice};
use gix_fs::stack::ToNormalPathComponents;
use gix_index::entry::Mode;
use gix_worktree::{Stack, stack::state::ignore::Source};

use crate::{hex_to_id, stack::probe_case};

#[test]
fn index_backed_ignore_paths_and_patterns_match_git_case_rules() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_casefold_index_ignore.sh")?;
    let worktree = fixture.join("repo");
    let git_dir = worktree.join(".git");
    let index = gix_index::File::at(
        git_dir.join("index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let objects = gix_odb::at(git_dir.join("objects"), gix_testtools::object_hash())?;
    for ignore_case in [false, true] {
        let state = gix_worktree::stack::State::IgnoreStack(gix_worktree::stack::state::Ignore::new(
            Default::default(),
            Default::default(),
            None,
            Source::WorktreeThenIdMappingIfNotSkipped,
            Default::default(),
        ));
        let mut stack = Stack::from_state_and_ignore_case(&worktree, ignore_case, state, &index, index.path_backing());
        let baseline = std::fs::read_to_string(fixture.join(format!("baseline-{ignore_case}")))?;
        for line in baseline.lines() {
            let (status, path) = line.split_once(' ').expect("the fixture records status and path");
            assert_eq!(
                stack.at_entry(path, Some(Mode::FILE), &objects)?.is_excluded(),
                status == "0",
                "index-backed ignores match Git for {path} with ignoreCase={ignore_case}"
            );
        }
    }
    Ok(())
}

#[test]
#[cfg(any(unix, windows))]
fn symlinked_ignore_files_are_skipped_with_index_fallback() -> TestResult {
    for use_index in [false, true] {
        let dir = gix_testtools::scripted_fixture_read_only_with_args(
            "make_symlinked_ignore_repo.sh",
            [if use_index { "indexed" } else { "symlink" }],
        )?;
        if !gix_testtools::fixture_has_symlinks(&dir)? {
            return Ok(());
        }
        let worktree = dir.join("repo");
        let ignore = worktree.join(".gitignore");
        assert!(
            ignore.symlink_metadata()?.file_type().is_symlink(),
            "the worktree .gitignore must be a symlink, not a copied regular file"
        );
        let mut buf = Vec::new();
        assert!(
            gix_glob::search::pattern::List::from_file(
                &ignore,
                None,
                true,
                &mut buf,
                gix_ignore::search::Ignore::default(),
            )?
            .is_some(),
            "explicitly configured pattern files may still follow symlinks"
        );
        assert_eq!(buf, b"external\n", "following loads the target's contents");

        let git_dir = worktree.join(".git");
        let index = gix_index::File::at(
            git_dir.join("index"),
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )?;
        let objects = gix_odb::at(git_dir.join("objects"), gix_testtools::object_hash())?;
        let state = gix_worktree::stack::State::IgnoreStack(gix_worktree::stack::state::Ignore::new(
            Default::default(),
            Default::default(),
            None,
            Source::WorktreeThenIdMappingIfNotSkipped,
            Default::default(),
        ));
        let mut stack = Stack::from_state_and_ignore_case(&worktree, false, state, &index, index.path_backing());
        for (path, excluded) in [("external", false), ("indexed", use_index)] {
            let baseline_status: i32 = std::fs::read_to_string(dir.join(format!("{path}.git-check-ignore.status")))?
                .trim()
                .parse()?;
            assert_eq!(baseline_status, i32::from(!excluded), "Git skips the symlink");
            assert_eq!(
                std::fs::read(dir.join(format!("{path}.git-check-ignore.out")))?,
                if excluded { b"indexed\n".as_slice() } else { b"" },
                "Git reports only the path excluded by the skip-worktree blob"
            );
            assert_eq!(
                stack.at_entry(path, Some(Mode::FILE), &objects)?.is_excluded(),
                excluded,
                "external patterns are skipped and skip-worktree blobs remain available"
            );
        }
    }
    Ok(())
}

struct IgnoreExpectations<'a> {
    lines: bstr::Lines<'a>,
}

impl<'a> Iterator for IgnoreExpectations<'a> {
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
fn exclude_by_dir_is_handled_just_like_git() -> TestResult {
    let dir = crate::scripted_fixture_read_only("make_special_exclude_case.sh").unwrap();
    let git_dir = dir.join(".git");

    let mut buf = Vec::new();
    let case = gix_glob::pattern::Case::Sensitive;
    let state = gix_worktree::stack::State::for_add(
        Default::default(),
        gix_worktree::stack::state::Ignore::new(
            Default::default(),
            gix_ignore::Search::from_git_dir(&git_dir, None, &mut buf, Default::default()).unwrap(),
            None,
            Source::WorktreeThenIdMappingIfNotSkipped,
            Default::default(),
        ),
    );
    let mut cache = Stack::new(&dir, state, case, buf, Default::default());
    let baseline = std::fs::read(git_dir.parent().unwrap().join("git-check-ignore.baseline")).unwrap();
    let expectations = IgnoreExpectations {
        lines: baseline.lines(),
    };
    struct FindError;
    impl gix_object::Find for FindError {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            gix_object::FindExt::find(&gix_object::find::Never, id, buffer).map(Some)
        }
    }
    for (relative_entry, source_and_line) in expectations {
        let (source, line, expected_pattern) = source_and_line.expect("every value is matched");
        let relative_path = gix_path::from_byte_slice(relative_entry)?;
        let is_dir = dir.join(relative_path).metadata().ok().map(metadata_to_mode);

        let platform = cache.at_entry(relative_entry, is_dir, &FindError).unwrap();
        let match_ = platform.matching_exclude_pattern().expect("match all values");
        let _is_excluded = platform.is_excluded();
        assert_eq!(
            match_.pattern.to_string(),
            expected_pattern,
            "we perfectly agree with git"
        );
        assert_eq!(
            expected_pattern, "tld/",
            "each entry matches on the main directory exclude, ignoring negations entirely"
        );
        // TODO: adjust baseline to also include precious files.
        assert_eq!(
            match_.kind,
            gix_ignore::Kind::Expendable,
            "for now all patterns are expendable until precious files are supported by git"
        );
        assert_eq!(line, 2);
        assert_eq!(source, ".gitignore");
    }
    Ok(())
}

fn metadata_to_mode(meta: Metadata) -> Mode {
    if meta.is_dir() {
        gix_index::entry::Mode::DIR
    } else {
        gix_index::entry::Mode::FILE
    }
}

#[test]
fn check_against_baseline() -> TestResult {
    let dir = crate::scripted_fixture_read_only("make_ignore_and_attributes_setup.sh")?;
    let worktree_dir = dir.join("repo");
    let git_dir = worktree_dir.join(".git");
    let mut buf = Vec::new();
    let user_exclude_path = dir.join("user.exclude");
    assert!(user_exclude_path.is_file());

    // Due to the way our setup differs from gits dynamic stack (which involves trying to read files from disk
    // by path) we can only test one case baseline, so we require multiple platforms (or filesystems) to run this.
    let case = probe_case()?;
    let mut index = gix_index::File::at(
        git_dir.join("index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let odb = gix_odb::at(git_dir.join("objects"), gix_testtools::object_hash())?;
    let parse_ignore = gix_ignore::search::Ignore::default();
    let state = gix_worktree::stack::State::for_add(
        Default::default(),
        gix_worktree::stack::state::Ignore::new(
            gix_ignore::Search::from_overrides(["!force-include"], parse_ignore),
            gix_ignore::Search::from_git_dir(&git_dir, Some(user_exclude_path), &mut buf, parse_ignore)?,
            None,
            Source::WorktreeThenIdMappingIfNotSkipped,
            parse_ignore,
        ),
    );
    let paths_storage = index.take_path_backing();
    let attribute_files_in_index = state.id_mappings_from_index(&index, &paths_storage, case);
    assert_eq!(
        attribute_files_in_index,
        vec![(
            "other-dir-with-ignore/.gitignore".into(),
            hex_to_id("5c7e0ed672d3d31d83a3df61f13cc8f7b22d5bfd")
        )]
    );
    let mut cache = Stack::new(&worktree_dir, state, case, buf, attribute_files_in_index);

    let baseline = std::fs::read(git_dir.parent().unwrap().join("git-check-ignore.baseline"))?;
    let expectations = IgnoreExpectations {
        lines: baseline.lines(),
    };
    for (relative_entry, source_and_line) in expectations {
        let relative_path = gix_path::from_byte_slice(relative_entry)?;
        let is_dir = worktree_dir.join(relative_path).metadata().ok().map(metadata_to_mode);

        let platform = cache.at_entry(relative_entry, is_dir, &odb)?;

        let match_ = platform.matching_exclude_pattern();
        let is_excluded = platform.is_excluded();
        match (match_, source_and_line) {
            (None, None) => {
                assert!(!is_excluded);
            }
            (Some(m), Some((source_file, line, pattern))) => {
                assert_eq!(m.pattern.to_string(), pattern);
                assert_eq!(m.sequence_number, line);
                // TODO: adjust baseline to also include precious files.
                if !m.pattern.is_negative() {
                    assert_eq!(
                        m.kind,
                        platform.excluded_kind().expect("it matches"),
                        "both values agree, no matter which method is used"
                    );
                }
                // Paths read from the index are relative to the repo, and they don't exist locally due tot skip-worktree
                if m.source.is_some_and(std::path::Path::exists) {
                    assert_eq!(
                        m.source.map(|p| p.canonicalize().unwrap()),
                        Some(worktree_dir.join(source_file.to_str_lossy().as_ref()).canonicalize()?)
                    );
                }
            }
            (Some(actual), None) if actual.pattern.is_negative() => {
                // OK: we provide negative patterns that matched on paths if there was no other match, while git doesn't.
            }
            (actual, expected) => {
                panic!(
                    "actual {actual:?} didn't match {expected:?} at \"{relative_entry}\": {components:?}",
                    components = relative_entry.to_normal_path_components().collect::<Vec<_>>()
                );
            }
        }
    }
    Ok(())
}
