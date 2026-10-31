use std::{borrow::Cow, io};

use gix::{
    Result,
    bstr::{BStr, ByteSlice},
    error::{ResultExt, bail},
};

use crate::{OutputFormat, is_dir_to_mode, repository::PathsOrPatterns};

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
) -> Result<()> {
    if format != OutputFormat::Human {
        bail!(gix::error::unsupported("JSON output isn't implemented yet"));
    }

    let index = repo.index()?;
    let mut cache = repo.excludes(
        &index,
        Some(gix::ignore::Search::from_overrides(
            overrides,
            repo.ignore_pattern_parser()?,
        )),
        Default::default(),
    )?;

    let paths: Box<dyn Iterator<Item = gix::bstr::BString>> = match input {
        PathsOrPatterns::Paths(paths) => paths,
        PathsOrPatterns::Patterns(paths) => Box::new(paths.into_iter()),
    };
    for path in paths {
        let mode = gix::path::from_bstr(Cow::Borrowed(path.as_ref()))?
            .metadata()
            .ok()
            .map(|m| is_dir_to_mode(m.is_dir()))
            .or_else(|| is_directory_query(path.as_bstr()).then_some(gix::index::entry::Mode::DIR));
        let query_path = repo.normalize_path(&path)?;
        let entry = cache.at_entry(query_path.as_bstr(), mode)?;
        let match_ = entry
            .matching_exclude_pattern()
            .filter(|m| show_ignore_patterns || !m.pattern.is_negative());
        print_match_unless_tracked(match_, &index, query_path.as_bstr(), path.as_ref(), &mut out).or_error()?;
    }

    if let Some(stats) = statistics.then(|| cache.take_statistics()) {
        out.flush().or_error()?;
        writeln!(err, "{stats:#?}").ok();
    }
    Ok(())
}

fn is_directory_query(path: &BStr) -> bool {
    path.ends_with(b"/") || path.ends_with(b"/.")
}

fn print_match_unless_tracked(
    match_: Option<gix::ignore::search::Match<'_>>,
    index: &gix::index::State,
    query_path: &BStr,
    display_path: &BStr,
    out: impl std::io::Write,
) -> std::io::Result<()> {
    print_match(
        match_.filter(|_| !is_tracked(index, query_path, is_directory_query(display_path))),
        display_path,
        out,
    )
}

fn is_tracked(index: &gix::index::State, path: &BStr, is_directory_query: bool) -> bool {
    let path = path.trim_end_with(|b| b == '/').as_bstr();
    (!is_directory_query && index.entry_index_by_path(path).is_ok()) || index.path_is_directory(path)
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
