use crate::OutputFormat;

pub struct Options {
    pub format: OutputFormat,
    pub statistics: bool,
    pub ignore: bool,
}

pub(crate) mod function {
    use std::{
        collections::BTreeSet,
        io,
        io::{BufRead, Write},
        iter::Peekable,
        ops::Sub,
        path::PathBuf,
        sync::atomic::Ordering,
    };

    use anyhow::{anyhow, bail};
    use gix::{attrs::Assignment, bstr::BString, Count, Progress};

    use crate::{
        repository::attributes::{query::attributes_cache, validate_baseline::Options},
        OutputFormat,
    };

    pub fn validate_baseline(
        repo: gix::Repository,
        paths: Option<impl Iterator<Item = BString> + Send + 'static>,
        mut progress: impl gix::NestedProgress + 'static,
        mut out: impl io::Write,
        mut err: impl io::Write,
        Options {
            format,
            statistics,
            mut ignore,
        }: Options,
    ) -> anyhow::Result<()> {
        if format != OutputFormat::Human {
            bail!("JSON output isn't implemented yet");
        }

        if repo.is_bare() {
            writeln!(
                err,
                "Repo {:?} is bare - disabling git-ignore baseline as `git check-ignore` needs a worktree",
                repo.path()
            )
            .ok();
            ignore = false;
        }
        let mut num_entries = None;
        let paths = paths.map_or_else(
            {
                let repo = repo.clone();
                let num_entries = &mut num_entries;
                move || -> anyhow::Result<_> {
                    let index = repo.index_or_load_from_head()?.into_owned();
                    let (entries, path_backing) = index.into_parts().0.into_entries();
                    *num_entries = Some(entries.len());
                    let iter = Box::new(entries.into_iter().map(move |e| e.path_in(&path_backing).to_owned()));
                    Ok(iter as Box<dyn Iterator<Item = BString> + Send + 'static>)
                }
            },
            |paths| anyhow::Result::Ok(Box::new(paths)),
        )?;

        let (tx_base, rx_base) = std::sync::mpsc::channel::<(String, Baseline)>();
        let feed_attrs = {
            let (tx, rx) = std::sync::mpsc::sync_channel::<BString>(100);
            std::thread::spawn({
                let path = repo.path().to_owned();
                let tx_base = tx_base.clone();
                let mut progress = progress.add_child("attributes");
                move || -> anyhow::Result<()> {
                    let mut child =
                        std::process::Command::from(gix::command::prepare(gix::path::env::exe_invocation()))
                            .args(["check-attr", "--stdin", "-a"])
                            .stdin(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::null())
                            .current_dir(path)
                            .spawn()?;

                    std::thread::spawn({
                        let mut stdin = child.stdin.take().expect("we configured it");
                        move || -> anyhow::Result<()> {
                            progress.init(num_entries, gix::progress::count("paths"));
                            let start = std::time::Instant::now();
                            for path in rx {
                                progress.inc();
                                stdin.write_all(&path)?;
                                stdin.write_all(b"\n")?;
                            }
                            progress.show_throughput(start);
                            Ok(())
                        }
                    });

                    let stdout = std::io::BufReader::new(child.stdout.take().expect("we configured it"));
                    let mut lines = stdout.lines().map_while(Result::ok).peekable();
                    while let Some(baseline) = parse_attributes(&mut lines) {
                        if tx_base.send(baseline).is_err() {
                            child.kill().ok();
                            break;
                        }
                    }

                    Ok(())
                }
            });
            tx
        };
        let work_dir = ignore
            .then(|| {
                repo.workdir()
                    .map(ToOwned::to_owned)
                    .ok_or_else(|| anyhow!("repository at {:?} must have a worktree checkout", repo.path()))
            })
            .transpose()?;
        let feed_excludes = ignore.then(|| {
            let (tx, rx) = std::sync::mpsc::sync_channel::<BString>(100);
            std::thread::spawn({
                let path = work_dir.expect("present if we are here");
                let tx_base = tx_base.clone();
                let mut progress = progress.add_child("excludes");
                move || -> anyhow::Result<()> {
                    let mut child =
                        std::process::Command::from(gix::command::prepare(gix::path::env::exe_invocation()))
                            .args(["check-ignore", "--stdin", "-nv", "--no-index"])
                            .stdin(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::null())
                            .current_dir(path)
                            .spawn()?;

                    std::thread::spawn({
                        let mut stdin = child.stdin.take().expect("we configured it");
                        move || -> anyhow::Result<()> {
                            progress.init(num_entries, gix::progress::count("paths"));
                            let start = std::time::Instant::now();
                            for path in rx {
                                progress.inc();
                                stdin.write_all(path.as_ref())?;
                                stdin.write_all(b"\n")?;
                            }
                            progress.show_throughput(start);
                            Ok(())
                        }
                    });

                    let stdout = std::io::BufReader::new(child.stdout.take().expect("we configured it"));
                    for line in stdout.lines() {
                        let line = line?;
                        if let Some(baseline) = parse_exclude(&line) {
                            if tx_base.send(baseline).is_err() {
                                child.kill().ok();
                                break;
                            }
                        } else {
                            eprintln!("Failed to parse line {line:?} - ignored");
                        }
                    }

                    Ok(())
                }
            });
            tx
        });
        drop(tx_base);

        std::thread::spawn(move || {
            for path in paths {
                if feed_attrs.send(path.clone()).is_err() {
                    break;
                }
                if let Some(ch) = feed_excludes.as_ref() {
                    if ch.send(path).is_err() {
                        break;
                    }
                }
            }
        });

        let (mut cache, _index) = attributes_cache(&repo)?;
        let mut matches = cache.attribute_matches();
        let mut progress = progress.add_child("validate");
        let mut mismatches = Vec::new();
        let start = std::time::Instant::now();
        progress.init(
            num_entries.map(|n| n + if ignore { n } else { 0 }),
            gix::progress::count("paths"),
        );

        for (rela_path, baseline) in rx_base {
            let entry = cache.at_entry(rela_path.as_str(), None)?;
            match baseline {
                Baseline::Attribute { assignments: expected } => {
                    entry.matching_attributes(&mut matches);
                    let fast_path_mismatch = matches
                        .iter()
                        .map(|m| m.assignment)
                        .zip(expected.iter().map(Assignment::as_ref))
                        .any(|(a, b)| a != b);
                    if fast_path_mismatch {
                        let actual_set = BTreeSet::from_iter(matches.iter().map(|m| m.assignment));
                        let expected_set = BTreeSet::from_iter(expected.iter().map(Assignment::as_ref));
                        let too_few_or_too_many =
                            !(expected_set.sub(&actual_set).is_empty() && actual_set.sub(&expected_set).is_empty());
                        if too_few_or_too_many {
                            mismatches.push((
                                rela_path,
                                Mismatch::Attributes {
                                    actual: matches.iter().map(|m| m.assignment.to_owned()).collect(),
                                    expected,
                                },
                            ));
                        }
                    }
                }
                Baseline::Exclude { location } => {
                    let match_ = entry.matching_exclude_pattern();
                    if match_.is_some() != location.is_some() {
                        mismatches.push((
                            rela_path,
                            Mismatch::Exclude {
                                actual: match_.map(Into::into),
                                expected: location,
                            },
                        ));
                    }
                }
            }
            progress.inc();
        }

        if let Some(stats) = statistics.then(|| cache.take_statistics()) {
            out.flush()?;
            writeln!(err, "{stats:#?}").ok();
        }
        progress.show_throughput(start);

        if mismatches.is_empty() {
            Ok(())
        } else {
            for (rela_path, mm) in &mismatches {
                writeln!(err, "{rela_path}: {mm:#?}").ok();
            }
            bail!(
                "{}: Validation failed with {} mismatches out of {}",
                gix::path::realpath(repo.workdir().unwrap_or(repo.git_dir()))?.display(),
                mismatches.len(),
                progress.counter().load(Ordering::Relaxed)
            );
        }
    }

    enum Baseline {
        Attribute { assignments: Vec<gix::attrs::Assignment> },
        Exclude { location: Option<ExcludeLocation> },
    }

    #[derive(Debug)]
    // See note on `Mismatch`
    #[allow(dead_code)]
    pub struct ExcludeLocation {
        pub line: usize,
        pub rela_source_file: String,
        pub pattern: String,
    }

    #[derive(Debug)]
    // We debug-print this structure, which makes all fields 'used', but it doesn't count.
    // TODO: find a way to not have to do more work, but make the warning go away.
    #[allow(dead_code)]
    pub enum Mismatch {
        Attributes {
            actual: Vec<gix::attrs::Assignment>,
            expected: Vec<gix::attrs::Assignment>,
        },
        Exclude {
            actual: Option<ExcludeMatch>,
            expected: Option<ExcludeLocation>,
        },
    }

    #[derive(Debug)]
    // See note on `Mismatch`
    #[allow(dead_code)]
    pub struct ExcludeMatch {
        pub pattern: gix::glob::Pattern,
        pub source: Option<PathBuf>,
        pub sequence_number: usize,
    }

    impl From<gix::ignore::search::Match<'_>> for ExcludeMatch {
        fn from(value: gix::ignore::search::Match<'_>) -> Self {
            ExcludeMatch {
                pattern: value.pattern.clone(),
                source: value.source.map(ToOwned::to_owned),
                sequence_number: value.sequence_number,
            }
        }
    }

    fn parse_exclude(line: &str) -> Option<(String, Baseline)> {
        let (left, value) = line.split_at(line.find('\t')?);
        let value = &value[1..];

        let location = if left == "::" {
            None
        } else {
            let mut tokens = left.split(':');
            let source = tokens.next()?;
            let line_number: usize = tokens.next()?.parse().ok()?;
            let pattern = tokens.next()?;
            Some(ExcludeLocation {
                line: line_number,
                rela_source_file: source.into(),
                pattern: pattern.into(),
            })
        };
        Some((value.to_string(), Baseline::Exclude { location }))
    }

    fn parse_attributes(lines: &mut Peekable<impl Iterator<Item = String>>) -> Option<(String, Baseline)> {
        let first = lines.next()?;
        let mut out = Vec::new();
        let (path, assignment) = parse_attribute_line(&first)?;

        let current = path.to_owned();
        out.push(assignment.to_owned());
        loop {
            let next_line = match lines.peek() {
                None => break,
                Some(l) => l,
            };
            let (next_path, next_assignment) = parse_attribute_line(next_line)?;
            if next_path != current {
                return Some((current, Baseline::Attribute { assignments: out }));
            } else {
                out.push(next_assignment.to_owned());
                lines.next();
            }
        }
        Some((current, Baseline::Attribute { assignments: out }))
    }

    fn parse_attribute_line(line: &str) -> Option<(&str, gix::attrs::AssignmentRef<'_>)> {
        use gix::{attrs::StateRef, bstr::ByteSlice};

        let mut prev = None;
        let mut tokens = line.splitn(3, |b| {
            let is_match = b == ' ' && prev.take() == Some(':');
            prev = Some(b);
            is_match
        });
        if let Some(((mut path, attr), info)) = tokens.next().zip(tokens.next()).zip(tokens.next()) {
            let state = match info {
                "set" => StateRef::Set,
                "unset" => StateRef::Unset,
                "unspecified" => StateRef::Unspecified,
                _ => StateRef::from_bytes(info.as_bytes()),
            };
            path = path.trim_end_matches(':');
            let attr = attr.trim_end_matches(':');
            let assignment = gix::attrs::AssignmentRef {
                name: gix::attrs::NameRef::try_from(attr.as_bytes().as_bstr()).ok()?,
                state,
            };
            Some((path, assignment))
        } else {
            None
        }
    }
}
