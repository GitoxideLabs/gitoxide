use gix::worktree::IndexPersistedOrInMemory;

use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub statistics: bool,
}

pub(crate) mod function {
    use std::{borrow::Cow, io, path::Path};

    use anyhow::bail;
    use gix::bstr::BStr;

    use crate::{
        is_dir_to_mode,
        repository::{
            attributes::query::{attributes_cache, Options},
            PathsOrPatterns,
        },
        OutputFormat,
    };

    pub fn query(
        repo: gix::Repository,
        input: PathsOrPatterns,
        mut out: impl io::Write,
        mut err: impl io::Write,
        Options { format, statistics }: Options,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("JSON output isn't implemented yet");
        }

        let (mut cache, index) = attributes_cache(&repo)?;
        let mut matches = cache.attribute_matches();

        match input {
            PathsOrPatterns::Paths(paths) => {
                for path in paths {
                    let mode = gix::path::from_bstr(Cow::Borrowed(path.as_ref()))
                        .metadata()
                        .ok()
                        .map(|m| is_dir_to_mode(m.is_dir()));

                    let entry = cache.at_entry(path.as_slice(), mode)?;
                    if !entry.matching_attributes(&mut matches) {
                        continue;
                    }
                    print_match(&matches, path.as_ref(), &mut out)?;
                }
            }
            PathsOrPatterns::Patterns(patterns) => {
                let mut pathspec = repo.pathspec(
                    true,
                    patterns.iter(),
                    true,
                    &index,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                        .adjust_for_bare(repo.is_bare()),
                )?;
                let mut pathspec_matched_entry = false;
                if let Some(it) = pathspec.index_entries_with_paths(&index) {
                    for (path, entry) in it {
                        pathspec_matched_entry = true;
                        let entry = cache.at_entry(path, entry.mode.into())?;
                        if !entry.matching_attributes(&mut matches) {
                            continue;
                        }
                        print_match(&matches, path, &mut out)?;
                    }
                }

                if !pathspec_matched_entry {
                    // TODO(borrowchk): this shouldn't be necessary at all, but `pathspec` stays borrowed mutably for some reason.
                    //                  It's probably due to the strange lifetimes of `index_entries_with_paths()`.
                    let pathspec = repo.pathspec(
                        true,
                        patterns.iter(),
                        true,
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
                        if !entry.matching_attributes(&mut matches) {
                            continue;
                        }
                        print_match(&matches, path, &mut out)?;
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
        matches: &gix::attrs::search::Outcome,
        path: &BStr,
        mut out: impl std::io::Write,
    ) -> std::io::Result<()> {
        for m in matches.iter() {
            writeln!(
                out,
                "{}:{}:{}\t{}\t{}",
                m.location.source.map(Path::to_string_lossy).unwrap_or_default(),
                m.location.sequence_number,
                m.pattern,
                path,
                m.assignment
            )?;
        }
        Ok(())
    }
}

pub(crate) fn attributes_cache(
    repo: &gix::Repository,
) -> anyhow::Result<(gix::AttributeStack<'_>, IndexPersistedOrInMemory)> {
    let index = repo.index_or_load_from_head()?;
    let cache = repo.attributes(
        &index,
        if repo.is_bare() {
            gix::worktree::stack::state::attributes::Source::IdMapping
        } else {
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
        },
        gix::worktree::stack::state::ignore::Source::IdMapping,
        None,
    )?;
    Ok((cache, index))
}
