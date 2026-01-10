#![allow(clippy::result_large_err)]
use std::{collections::HashMap, path::PathBuf, str::FromStr};

use gix_object::{bstr, bstr::BStr};
use gix_ref::bstr::{BString, ByteSlice};
use gix_revision::spec::Kind;
use std::sync::LazyLock;

const FIXTURE_NAME: &str = "make_rev_spec_parse_repos.sh";
static BASELINE: LazyLock<HashMap<PathBuf, HashMap<BString, Option<gix_revision::Spec>>>> = LazyLock::new(|| {
    fn kind_of(spec: &BStr) -> gix_revision::spec::Kind {
        if spec.starts_with(b"^") {
            gix_revision::spec::Kind::IncludeReachable
        } else if spec.contains_str(b"...") {
            gix_revision::spec::Kind::ReachableToMergeBase
        } else if spec.contains_str(b"..") {
            gix_revision::spec::Kind::RangeBetween
        } else if spec.ends_with(b"^!") {
            gix_revision::spec::Kind::ExcludeReachableFromParents
        } else if spec.ends_with(b"^@") {
            unreachable!("BUG: cannot use rev^@ as it won't list the actual commit")
        } else {
            gix_revision::spec::Kind::IncludeReachable
        }
    }
    fn lines_of(kind: gix_revision::spec::Kind) -> Option<usize> {
        Some(match kind {
            Kind::ExcludeReachable | Kind::IncludeReachable => 1,
            Kind::RangeBetween => 2,
            Kind::ReachableToMergeBase => 3,
            Kind::IncludeReachableFromParents | Kind::ExcludeReachableFromParents => return None,
        })
    }
    fn object_id_of_next(lines: &mut std::iter::Peekable<bstr::Lines<'_>>) -> gix_hash::ObjectId {
        let hex_hash = lines.next().expect("valid respect yields enough lines");
        object_id_of(hex_hash).expect("git yields full objects ids")
    }
    fn object_id_of(input: &[u8]) -> Option<gix_hash::ObjectId> {
        let hex_hash = input.strip_prefix(b"^").unwrap_or(input);
        gix_hash::ObjectId::from_str(hex_hash.to_str().expect("hex is ascii")).ok()
    }
    let mut baseline_map = HashMap::new();
    let base = gix_testtools::scripted_fixture_read_only(FIXTURE_NAME).unwrap();
    for baseline_entry in walkdir::WalkDir::new(base)
        .max_depth(2)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok().and_then(|e| (e.file_name() == "baseline.git").then_some(e)))
    {
        let map = baseline_map
            .entry(baseline_entry.path().parent().expect("file in directory").into())
            .or_insert_with(HashMap::default);
        let baseline = std::fs::read(baseline_entry.path()).unwrap();
        let mut lines = baseline.lines().peekable();
        while let Some(spec) = lines.next() {
            let exit_code_or_hash = lines.next().expect("exit code or single hash").to_str().unwrap();
            let kind = kind_of(spec.as_bstr());
            let first_hash = match u8::from_str(exit_code_or_hash) {
                Ok(_exit_code) => {
                    let is_duplicate = map.insert(spec.into(), None).is_some();
                    assert!(!is_duplicate, "Duplicate spec '{}' cannot be handled", spec.as_bstr());
                    continue;
                }
                Err(_) => match gix::ObjectId::from_str(exit_code_or_hash) {
                    Ok(hash) => hash,
                    Err(_) => break, // for now bail out, we can't parse multi-line results yet
                },
            };
            let num_lines = lines_of(kind);
            let rev_spec = match num_lines {
                Some(line_count) => match line_count {
                    1 if kind == gix_revision::spec::Kind::IncludeReachable => gix_revision::Spec::Include(first_hash),
                    1 if kind == gix_revision::spec::Kind::ExcludeReachable => gix_revision::Spec::Exclude(first_hash),
                    2 | 3 => {
                        let second_hash = object_id_of_next(&mut lines);
                        if line_count == 2 {
                            gix_revision::Spec::Range {
                                from: second_hash,
                                to: first_hash,
                            }
                        } else {
                            lines.next().expect("merge-base to consume");
                            gix_revision::Spec::Merge {
                                theirs: first_hash,
                                ours: second_hash,
                            }
                        }
                    }
                    _ => unreachable!(),
                },
                None => {
                    let rev_spec = match kind {
                        gix_revision::spec::Kind::ExcludeReachableFromParents => {
                            gix_revision::Spec::ExcludeParents(first_hash)
                        }
                        _ => unreachable!(),
                    };
                    while let Some(_oid) = lines.peek().map(|hex| object_id_of(hex)) {
                        lines.next();
                    }
                    rev_spec
                }
            };
            let is_duplicate = map.insert(spec.into(), Some(rev_spec)).is_some();
            assert!(!is_duplicate, "Duplicate spec '{}' cannot be handled", spec.as_bstr());
            if num_lines.filter(|count| *count > 1).is_some() {
                // git always considers these errors for some reason, so skip it.
                lines.next();
            }
        }
    }
    baseline_map
});

pub fn parse_spec_no_baseline<'a>(
    spec: &str,
    repo: &'a gix::Repository,
) -> Result<gix::revision::Spec<'a>, gix_error::Error> {
    parse_spec_no_baseline_opts(spec, repo, Default::default())
}

enum BaselineExpectation {
    /// We have the same result as git
    Same,
    /// Git can't do something that we can
    GitFailsWeSucceed,
}

/// Git can't do that, but we can
pub fn parse_spec_better_than_baseline<'a>(
    spec: &str,
    repo: &'a gix::Repository,
) -> Result<gix::revision::Spec<'a>, gix_error::Error> {
    let res = gix::revision::Spec::from_bstr(spec, repo, Default::default());
    compare_with_baseline(&res, repo, spec, BaselineExpectation::GitFailsWeSucceed);
    res
}

pub fn parse_spec_no_baseline_opts<'a>(
    spec: &str,
    repo: &'a gix::Repository,
    opts: gix::revision::spec::parse::Options,
) -> Result<gix::revision::Spec<'a>, gix_error::Error> {
    gix::revision::Spec::from_bstr(spec, repo, opts)
}

pub fn parse_spec_opts<'a>(
    spec: &str,
    repo: &'a gix::Repository,
    opts: gix::revision::spec::parse::Options,
) -> Result<gix::revision::Spec<'a>, gix_error::Error> {
    let res = gix::revision::Spec::from_bstr(spec, repo, opts);
    compare_with_baseline(&res, repo, spec, BaselineExpectation::Same);
    res
}

pub fn rev_parse<'a>(spec: &str, repo: &'a gix::Repository) -> Result<gix::revision::Spec<'a>, gix_error::Error> {
    let res = repo.rev_parse(spec);
    compare_with_baseline(&res, repo, spec, BaselineExpectation::Same);
    res
}

fn compare_with_baseline(
    res: &Result<gix::revision::Spec<'_>, gix_error::Error>,
    repo: &gix::Repository,
    spec: &str,
    expectation: BaselineExpectation,
) {
    let actual = res.as_deref().ok().copied();
    let spec: BString = spec.into();
    let expected = *BASELINE
        .get(repo.workdir().unwrap_or_else(|| repo.git_dir()))
        .unwrap_or_else(|| panic!("No baseline for {repo:?}"))
        .get(&spec)
        .unwrap_or_else(|| panic!("'{spec}' revspec not found in git baseline"));
    match expectation {
        BaselineExpectation::Same => {
            assert_eq!(
                actual, expected,
                "{spec}: left (ours) should match right (git): {res:?}"
            );
        }
        BaselineExpectation::GitFailsWeSucceed => {
            assert_eq!(expected, None, "Git should fail here");
        }
    }
}

pub fn parse_spec(spec: impl AsRef<str>, repo: &gix::Repository) -> Result<gix::revision::Spec<'_>, gix_error::Error> {
    parse_spec_opts(spec.as_ref(), repo, Default::default())
}

pub fn repo(name: &str) -> crate::Result<gix::Repository> {
    let base = gix_testtools::scripted_fixture_read_only(FIXTURE_NAME)?;
    Ok(gix::open(base.join(name))?)
}
