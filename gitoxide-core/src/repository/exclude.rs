use std::{borrow::Cow, io};

use anyhow::bail;
use gix::bstr::{BStr, ByteSlice};

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
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }

    let index = repo.index()?;
    let mut cache = repo.excludes(
        &index,
        Some(gix::ignore::Search::from_overrides(
            overrides.clone(),
            repo.ignore_pattern_parser()?,
        )),
        Default::default(),
    )?;
    let mut probe_cache = repo.excludes(
        &index,
        Some(gix::ignore::Search::from_overrides(
            overrides,
            repo.ignore_pattern_parser()?,
        )),
        Default::default(),
    )?;

    let current_dir = repo.current_dir().to_owned();
    let prefix = repo.prefix()?.map(|prefix| gix::path::into_bstr(prefix).into_owned());
    let workdir = repo.workdir().map(ToOwned::to_owned);
    match input {
        PathsOrPatterns::Paths(paths) => {
            for path in paths {
                let query_path = prefixed_path(prefix.as_ref().map(AsRef::as_ref), path.as_ref());
                print_exclude_match(
                    query_path.as_ref(),
                    path.as_ref(),
                    mode_for_path(Some(&current_dir), path.as_ref(), false),
                    &index,
                    &mut cache,
                    show_ignore_patterns,
                    &mut out,
                )?;
            }
        }
        PathsOrPatterns::Patterns(patterns) => {
            enum Action<'a> {
                Direct {
                    query_path: gix::bstr::BString,
                    display_path: gix::bstr::BString,
                    mode: Option<gix::index::entry::Mode>,
                },
                Expand {
                    idx: usize,
                    path: &'a gix::bstr::BString,
                },
            }

            let mut inclusion_pathspec = repo.pathspec(
                true,
                patterns.iter(),
                repo.workdir().is_some(),
                &index,
                gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping.adjust_for_bare(repo.is_bare()),
            )?;
            let pattern_metadata = inclusion_pathspec
                .search()
                .patterns()
                .map(|pattern| {
                    (
                        pattern.path().to_owned(),
                        pattern.signature.contains(gix::pathspec::MagicSignature::MUST_BE_DIR),
                        pattern_uses_glob_matching(pattern),
                        pattern.is_excluded(),
                    )
                })
                .collect::<Vec<_>>();
            let has_excluded_patterns = pattern_metadata.iter().any(|(_, _, _, is_excluded)| *is_excluded);
            let mut actions = Vec::new();
            let mut expansion_patterns = Vec::new();
            let mut has_positive_expansion_pattern = false;
            for (path, (query_path, must_be_dir, is_glob, is_excluded)) in patterns.iter().zip(pattern_metadata) {
                let mode = mode_for_path(workdir.as_deref(), query_path.as_ref(), must_be_dir);
                let query_path_ref: &BStr = query_path.as_ref();
                if !has_excluded_patterns
                    && !is_glob
                    && !is_excluded
                    && inclusion_pathspec.is_included(query_path_ref, mode.map(mode_to_is_dir))
                    && has_exclude_match(query_path_ref, mode, &mut probe_cache, show_ignore_patterns)?
                {
                    let display_path = display_path_for_pathspec(path.as_ref(), query_path_ref).to_owned();
                    actions.push(Action::Direct {
                        query_path,
                        display_path,
                        mode,
                    });
                } else {
                    has_positive_expansion_pattern |= !is_excluded;
                    let idx = expansion_patterns.len();
                    expansion_patterns.push(path);
                    actions.push(Action::Expand { idx, path });
                }
            }

            let mut expanded_entries = Vec::new();
            if !expansion_patterns.is_empty() && has_positive_expansion_pattern {
                let mut pathspec = repo.pathspec(
                    true,
                    expansion_patterns.iter().copied(),
                    repo.workdir().is_some(),
                    &index,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                        .adjust_for_bare(repo.is_bare()),
                )?;

                {
                    let entries = pathspec.index_entries_with_paths(&index);
                    if let Some(entries) = entries {
                        for (entry_path, entry) in entries {
                            expanded_entries.push((entry_path.to_owned(), entry.mode.into()));
                        }
                    }
                }
            }

            if expanded_entries.is_empty() {
                let fallback_patterns = if !expansion_patterns.is_empty() && has_positive_expansion_pattern {
                    let mut pathspec = repo.pathspec(
                        true,
                        expansion_patterns.iter().copied(),
                        repo.workdir().is_some(),
                        &index,
                        gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                            .adjust_for_bare(repo.is_bare()),
                    )?;
                    let fallback_candidates = expansion_patterns
                        .iter()
                        .zip(pathspec.search().patterns())
                        .map(|(_path, pattern)| {
                            (!pattern.is_excluded()).then(|| {
                                let query_path = pattern.path().to_owned();
                                let mode = mode_for_path(
                                    workdir.as_deref(),
                                    query_path.as_ref(),
                                    pattern.signature.contains(gix::pathspec::MagicSignature::MUST_BE_DIR),
                                );
                                (query_path, mode)
                            })
                        })
                        .collect::<Vec<_>>();
                    fallback_candidates
                        .into_iter()
                        .map(|candidate| {
                            candidate.and_then(|(query_path, mode)| {
                                let is_included = {
                                    let query_path: &BStr = query_path.as_ref();
                                    pathspec.is_included(query_path, mode.map(mode_to_is_dir))
                                };
                                is_included.then_some((query_path, mode, true))
                            })
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };

                for action in actions {
                    match action {
                        Action::Direct {
                            query_path,
                            display_path,
                            mode,
                        } => print_exclude_match(
                            query_path.as_ref(),
                            display_path.as_ref(),
                            mode,
                            &index,
                            &mut cache,
                            show_ignore_patterns,
                            &mut out,
                        )?,
                        Action::Expand { idx, path } => {
                            let Some(Some((query_path, mode, true))) = fallback_patterns.get(idx) else {
                                continue;
                            };
                            let query_path: &BStr = query_path.as_ref();
                            let display_path = display_path_for_pathspec(path.as_ref(), query_path);
                            print_exclude_match(
                                query_path,
                                display_path,
                                *mode,
                                &index,
                                &mut cache,
                                show_ignore_patterns,
                                &mut out,
                            )?;
                        }
                    }
                }
            } else {
                for action in actions {
                    if let Action::Direct {
                        query_path,
                        display_path,
                        mode,
                    } = action
                    {
                        print_exclude_match(
                            query_path.as_ref(),
                            display_path.as_ref(),
                            mode,
                            &index,
                            &mut cache,
                            show_ignore_patterns,
                            &mut out,
                        )?;
                    }
                }
                for (entry_path, mode) in expanded_entries {
                    print_exclude_match(
                        entry_path.as_ref(),
                        entry_path.as_ref(),
                        mode,
                        &index,
                        &mut cache,
                        show_ignore_patterns,
                        &mut out,
                    )?;
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

fn pattern_uses_glob_matching(pattern: &gix::pathspec::Pattern) -> bool {
    pattern.search_mode != gix::pathspec::SearchMode::Literal
        && pattern.path().iter().any(|b| matches!(b, b'*' | b'?' | b'['))
}

fn prefixed_path<'a>(prefix: Option<&BStr>, path: &'a BStr) -> Cow<'a, BStr> {
    match prefix {
        Some(prefix) if !prefix.is_empty() => {
            let mut prefixed = prefix.to_owned();
            if !path.is_empty() {
                prefixed.push(b'/');
                prefixed.extend_from_slice(path.as_bytes());
            }
            Cow::Owned(prefixed)
        }
        _ => Cow::Borrowed(path),
    }
}

fn display_path_for_pathspec<'a>(pathspec: &'a BStr, query_path: &'a BStr) -> &'a BStr {
    if pathspec.starts_with_str(":") {
        query_path
    } else {
        pathspec
    }
}

fn mode_for_path(workdir: Option<&std::path::Path>, path: &BStr, must_be_dir: bool) -> Option<gix::index::entry::Mode> {
    let rela_path = gix::path::from_bstr(Cow::Borrowed(path));
    let is_dir = match workdir {
        Some(workdir) => workdir.join(&rela_path).metadata(),
        None => rela_path.metadata(),
    }
    .ok()
    .map_or(must_be_dir || path.ends_with_str("/"), |m| m.is_dir());
    Some(is_dir_to_mode(is_dir))
}

fn mode_to_is_dir(mode: gix::index::entry::Mode) -> bool {
    mode.is_sparse() || mode.is_submodule()
}

fn print_exclude_match(
    query_path: &BStr,
    display_path: &BStr,
    mode: Option<gix::index::entry::Mode>,
    index: &gix::index::State,
    cache: &mut gix::AttributeStack<'_>,
    show_ignore_patterns: bool,
    out: impl std::io::Write,
) -> anyhow::Result<()> {
    let entry = cache.at_entry(query_path, mode)?;
    let match_ = entry
        .matching_exclude_pattern()
        .filter(|m| show_ignore_patterns || !m.pattern.is_negative())
        .filter(|_| !index_suppresses_exclude_match(index, query_path, mode));
    Ok(print_match(match_, display_path, out)?)
}

fn has_exclude_match(
    query_path: &BStr,
    mode: Option<gix::index::entry::Mode>,
    cache: &mut gix::AttributeStack<'_>,
    show_ignore_patterns: bool,
) -> anyhow::Result<bool> {
    let entry = cache.at_entry(query_path, mode)?;
    Ok(entry
        .matching_exclude_pattern()
        .is_some_and(|m| show_ignore_patterns || !m.pattern.is_negative()))
}

fn index_suppresses_exclude_match(
    index: &gix::index::State,
    path: &BStr,
    mode: Option<gix::index::entry::Mode>,
) -> bool {
    let path = path_without_trailing_slashes(path);
    if path.is_empty() {
        return false;
    }
    if mode.is_some_and(|mode| mode.is_sparse() || mode.is_submodule()) {
        index.path_is_directory(path)
    } else {
        index.entry_by_path(path).is_some()
    }
}

fn path_without_trailing_slashes(path: &BStr) -> &BStr {
    let bytes = path.as_bytes();
    let end = bytes.iter().rposition(|b| *b != b'/').map_or(0, |idx| idx + 1);
    bytes[..end].as_bstr()
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

#[cfg(test)]
mod tests {
    use std::io::Write;

    use gix::bstr::{BString, ByteSlice};

    use super::*;

    #[test]
    fn query_matches_git_check_ignore_with_tracked_paths() -> anyhow::Result<()> {
        let repo_dir = repo_with_ignored_directories_containing_tracked_files()?;
        let _lock = current_dir_lock().lock().expect("current dir lock isn't poisoned");
        let _cwd = CurrentDir::set(repo_dir.path())?;
        let repo = gix::discover(".")?;
        let paths = paths();
        let expected = git_check_ignore_baseline(repo_dir.path(), &paths)?;

        assert_query_paths_match_git(&repo, &paths, &expected)?;

        Ok(())
    }

    #[test]
    fn paths_from_stdin_are_relative_to_current_dir() -> anyhow::Result<()> {
        let repo_dir = tempfile::TempDir::new()?;
        run_git(repo_dir.path(), ["init", "-q"])?;
        std::fs::create_dir_all(repo_dir.path().join("sub/ignored-dir"))?;
        std::fs::write(repo_dir.path().join("sub/.gitignore"), "ignored-dir/\n")?;
        run_git(repo_dir.path(), ["add", "sub/.gitignore"])?;

        let subdir = repo_dir.path().join("sub");
        let _lock = current_dir_lock().lock().expect("current dir lock isn't poisoned");
        let _cwd = CurrentDir::set(&subdir)?;
        let repo = gix::discover(".")?;
        let paths = vec!["ignored-dir".into()];
        let expected = git_check_ignore_baseline(&subdir, &paths)?;

        assert_eq!(query_stdin_paths(&repo, &paths)?, expected);

        Ok(())
    }

    #[test]
    fn pathspec_expansion_is_preserved() -> anyhow::Result<()> {
        let repo_dir = tempfile::TempDir::new()?;
        run_git(repo_dir.path(), ["init", "-q"])?;
        std::fs::create_dir_all(repo_dir.path().join("src"))?;
        std::fs::create_dir_all(repo_dir.path().join("ignored"))?;
        std::fs::write(repo_dir.path().join(".gitignore"), "*.rs\nignored/\n")?;
        std::fs::write(repo_dir.path().join("src/a.rs"), "fn main() {}\n")?;
        std::fs::write(repo_dir.path().join("src/b.rs"), "fn main() {}\n")?;
        run_git(repo_dir.path(), ["add", "-f", ".gitignore", "src/a.rs", "src/b.rs"])?;

        let _lock = current_dir_lock().lock().expect("current dir lock isn't poisoned");
        let _cwd = CurrentDir::set(repo_dir.path())?;
        let repo = gix::discover(".")?;

        assert_query_patterns(&repo, ["src"], ["::\tsrc/a.rs", "::\tsrc/b.rs"])?;
        assert_query_patterns(&repo, ["*.rs", "src/*"], ["::\tsrc/a.rs", "::\tsrc/b.rs"])?;
        assert_query_patterns(&repo, ["src/*", ":(exclude)src/a.rs"], ["::\tsrc/b.rs"])?;
        assert_query_patterns(&repo, [":(exclude)src/a.rs"], [])?;
        assert_query_patterns(&repo, [":(attr:foo)ignored", "src"], ["::\tsrc/a.rs", "::\tsrc/b.rs"])?;
        assert_query_patterns(&repo, [":(top)ignored"], [".gitignore:2:ignored/\tignored"])?;

        Ok(())
    }

    fn repo_with_ignored_directories_containing_tracked_files() -> anyhow::Result<tempfile::TempDir> {
        let dir = tempfile::TempDir::new()?;
        run_git(dir.path(), ["init", "-q"])?;

        std::fs::create_dir_all(dir.path().join("src/bin"))?;
        std::fs::create_dir_all(dir.path().join("other/bin"))?;
        std::fs::write(dir.path().join(".gitignore"), "bin/\n")?;
        std::fs::write(dir.path().join("src/bin/stub_gen.rs"), "fn main() {}\n")?;
        run_git(dir.path(), ["add", "-f", ".gitignore", "src/bin/stub_gen.rs"])?;

        std::fs::write(dir.path().join("src/bin/extra.txt"), "extra\n")?;
        std::fs::write(dir.path().join("other/bin/file.txt"), "other\n")?;

        Ok(dir)
    }

    fn paths() -> Vec<BString> {
        [
            "src/bin",
            "src/bin/",
            "src/bin/stub_gen.rs",
            "src/bin/extra.txt",
            "other/bin",
            "other/bin/file.txt",
        ]
        .into_iter()
        .map(Into::into)
        .collect()
    }

    fn git_check_ignore_baseline(repo_dir: &std::path::Path, paths: &[BString]) -> anyhow::Result<Vec<BString>> {
        let mut child = std::process::Command::new("git")
            .args(["check-ignore", "-vn", "--stdin"])
            .current_dir(repo_dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        {
            let mut stdin = child.stdin.take().expect("configured");
            for path in paths {
                stdin.write_all(path)?;
                stdin.write_all(b"\n")?;
            }
        }

        let output = child.wait_with_output()?;
        assert!(
            output.status.success(),
            "git check-ignore failed: {}",
            output.stderr.as_bstr()
        );
        Ok(normalize_output(&output.stdout))
    }

    fn query_output(repo: gix::Repository, input: PathsOrPatterns) -> anyhow::Result<Vec<BString>> {
        let mut out = Vec::new();
        query(repo, input, &mut out, Vec::new(), query_options())?;
        Ok(normalize_output(&out))
    }

    fn assert_query_paths_match_git(
        repo: &gix::Repository,
        paths: &[BString],
        expected: &[BString],
    ) -> anyhow::Result<()> {
        assert_eq!(query_pathspecs(repo, paths)?, expected);
        assert_eq!(query_stdin_paths(repo, paths)?, expected);
        Ok(())
    }

    fn query_pathspecs(repo: &gix::Repository, paths: &[BString]) -> anyhow::Result<Vec<BString>> {
        query_output(repo.clone(), PathsOrPatterns::Patterns(paths.to_vec()))
    }

    fn query_stdin_paths(repo: &gix::Repository, paths: &[BString]) -> anyhow::Result<Vec<BString>> {
        let paths = paths.to_vec();
        query_output(repo.clone(), PathsOrPatterns::Paths(Box::new(paths.into_iter())))
    }

    fn assert_query_patterns<const PATTERNS: usize, const EXPECTED: usize>(
        repo: &gix::Repository,
        patterns: [&str; PATTERNS],
        expected: [&str; EXPECTED],
    ) -> anyhow::Result<()> {
        assert_eq!(query_patterns(repo, patterns)?, bstrings(expected));
        Ok(())
    }

    fn query_patterns<const N: usize>(repo: &gix::Repository, patterns: [&str; N]) -> anyhow::Result<Vec<BString>> {
        query_output(
            repo.clone(),
            PathsOrPatterns::Patterns(patterns.into_iter().map(Into::into).collect()),
        )
    }

    fn bstrings<const N: usize>(lines: [&str; N]) -> Vec<BString> {
        lines.into_iter().map(Into::into).collect()
    }

    fn query_options() -> query::Options {
        query::Options {
            format: OutputFormat::Human,
            overrides: Vec::new(),
            show_ignore_patterns: false,
            statistics: false,
        }
    }

    struct CurrentDir(std::path::PathBuf);

    fn current_dir_lock() -> &'static std::sync::Mutex<()> {
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        &LOCK
    }

    impl CurrentDir {
        fn set(path: &std::path::Path) -> std::io::Result<Self> {
            let previous = std::env::current_dir()?;
            std::env::set_current_dir(path)?;
            Ok(CurrentDir(previous))
        }
    }

    impl Drop for CurrentDir {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.0).expect("restore current dir");
        }
    }

    fn normalize_output(output: &[u8]) -> Vec<BString> {
        output
            .lines()
            .map(|line| {
                if let Some(rest) = line.strip_prefix(b"./") {
                    rest.as_bstr().to_owned()
                } else if let Some(gitignore_pos) = line.find(".gitignore:") {
                    line[gitignore_pos..].as_bstr().to_owned()
                } else {
                    line.as_bstr().to_owned()
                }
            })
            .collect()
    }

    fn run_git<const N: usize>(repo_dir: &std::path::Path, args: [&str; N]) -> anyhow::Result<()> {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo_dir)
            .output()?;
        assert!(output.status.success(), "git failed: {}", output.stderr.as_bstr());
        Ok(())
    }
}
