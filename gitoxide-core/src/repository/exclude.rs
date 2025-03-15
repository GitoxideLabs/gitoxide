use std::{borrow::Cow, io};

use anyhow::bail;
use gix::bstr::BStr;

use crate::{is_dir_to_mode, repository::PathsOrPatterns, OutputFormat};

pub mod query {
    use std::ffi::OsString;

    use crate::OutputFormat;

    pub struct Options {
        pub format: OutputFormat,
        pub overrides: Vec<OsString>,
        pub show_ignore_patterns: bool,
        pub statistics: bool,
    }
}

pub fn query(
    repo: gix::Repository,
    input: PathsOrPatterns,
    mut out: impl io::Write,
    mut err: impl io::Write,
    query::Options {
        overrides,
        format,
        show_ignore_patterns,
        statistics,
    }: query::Options,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }

    let index = repo.index()?;
    let mut cache = repo.excludes(
        &index,
        Some(gix::ignore::Search::from_overrides(overrides.into_iter())),
        Default::default(),
    )?;

    match input {
        PathsOrPatterns::Paths(paths) => {
            for path in paths {
                let mode = gix::path::from_bstr(Cow::Borrowed(path.as_ref()))
                    .metadata()
                    .ok()
                    .map(|m| is_dir_to_mode(m.is_dir()));
                let entry = cache.at_entry(path.as_slice(), mode)?;
                let match_ = entry
                    .matching_exclude_pattern()
                    .and_then(|m| (show_ignore_patterns || !m.pattern.is_negative()).then_some(m));
                print_match(match_, path.as_ref(), &mut out)?;
            }
        }
        PathsOrPatterns::Patterns(patterns) => {
            let mut pathspec_matched_something = false;
            let mut pathspec = repo.pathspec(
                true,
                patterns.iter(),
                repo.workdir().is_some(),
                &index,
                gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
            )?;

            if let Some(it) = pathspec.index_entries_with_paths(&index) {
                for (path, entry) in it {
                    pathspec_matched_something = true;
                    let entry = cache.at_entry(path, entry.mode.into())?;
                    let match_ = entry
                        .matching_exclude_pattern()
                        .and_then(|m| (show_ignore_patterns || !m.pattern.is_negative()).then_some(m));
                    print_match(match_, path, &mut out)?;
                }
            }

            if !pathspec_matched_something {
                // TODO(borrowchk): this shouldn't be necessary at all, but `pathspec` stays borrowed mutably for some reason.
                //                  It's probably due to the strange lifetimes of `index_entries_with_paths()`.
                let pathspec = repo.pathspec(
                    true,
                    patterns.iter(),
                    repo.workdir().is_some(),
                    &index,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                        .adjust_for_bare(repo.is_bare()),
                )?;
                let workdir = repo.workdir();
                for pattern in pathspec.search().patterns() {
                    let path = pattern.path();
                    let entry = cache.at_entry(
                        path,
                        Some(is_dir_to_mode(
                            workdir.is_some_and(|wd| wd.join(gix::path::from_bstr(path)).is_dir())
                                || pattern.signature.contains(gix::pathspec::MagicSignature::MUST_BE_DIR),
                        )),
                    )?;
                    let match_ = entry
                        .matching_exclude_pattern()
                        .and_then(|m| (show_ignore_patterns || !m.pattern.is_negative()).then_some(m));
                    print_match(match_, path, &mut out)?;
                }
            }
        }
    }

    if let Some(stats) = statistics.then(|| cache.take_statistics()) {
        out.flush()?;
        writeln!(err, "{stats:#?}").ok();
    }
    Ok(())
}

fn print_match(
    m: Option<gix::ignore::search::Match<'_>>,
    path: &BStr,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    match m {
        Some(m) => writeln!(
            out,
            "{}:{}:{}\t{}",
            m.source.map(std::path::Path::to_string_lossy).unwrap_or_default(),
            m.sequence_number,
            m.pattern,
            path
        ),
        None => writeln!(out, "::\t{path}"),
    }
}
