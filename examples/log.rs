use std::{
    io::{stdout, Write},
    path::{Path, PathBuf},
};

/// A toy-version of `git log`.
use clap::Parser;
use gix::{
    bstr::{BString, ByteSlice},
    date::time::format,
    revision::walk::Sorting,
};

fn main() {
    let args = Args::parse_from(gix::env::args_os());
    match run(args) {
        Ok(()) => {}
        Err(e) => eprintln!("error: {e}"),
    }
}

#[derive(Debug, clap::Parser)]
#[clap(name = "log", about = "git log example", version = option_env!("GIX_VERSION"))]
struct Args {
    /// Alternative git directory to use
    #[clap(name = "dir", long = "git-dir")]
    git_dir: Option<PathBuf>,
    /// Number of commits to return
    #[clap(short, long)]
    count: Option<usize>,
    /// Number of commits to skip
    #[clap(short, long)]
    skip: Option<usize>,
    /// Commits are sorted as they are mentioned in the commit graph.
    #[clap(short, long)]
    breadth_first: bool,
    /// Commits are sorted by their commit time in descending order.
    #[clap(short, long)]
    newest_first: bool,
    /// Show commits with the specified minimum number of parents
    #[clap(long)]
    min_parents: Option<usize>,
    /// Show commits with the specified maximum number of parents
    #[clap(long)]
    max_parents: Option<usize>,
    /// Show only merge commits (implies --min-parents=2)
    #[clap(long)]
    merges: bool,
    /// Show only non-merge commits (implies --max-parents=1)
    #[clap(long)]
    no_merges: bool,
    /// Reverse the commit sort order (and loads all of them into memory).
    #[clap(short, long)]
    reverse: bool,
    /// The ref-spec for the first commit to log, or HEAD.
    #[clap(name = "commit")]
    committish: Option<String>,
    /// The path interested in log history of
    #[clap(name = "path")]
    paths: Vec<PathBuf>,
}

fn run(args: Args) -> anyhow::Result<()> {
    let repo = gix::discover(args.git_dir.as_deref().unwrap_or(Path::new(".")))?;
    let commit = repo
        .rev_parse_single({
            args.committish
                .map(|mut c| {
                    c.push_str("^{commit}");
                    c
                })
                .as_deref()
                .unwrap_or("HEAD")
        })?
        .object()?
        .try_into_commit()?;

    let sorting = if args.breadth_first {
        Sorting::BreadthFirst
    } else {
        // else if args.newest_first {
        Sorting::ByCommitTime(Default::default())
    };

    let mut min_parents = args.min_parents.unwrap_or(0);
    let mut max_parents = args.max_parents.unwrap_or(usize::MAX);
    if args.merges {
        min_parents = 2;
    }
    if args.no_merges {
        max_parents = 1;
    }

    let mut log_iter: Box<dyn Iterator<Item = Result<LogEntryInfo, _>>> = Box::new(
        repo.rev_walk([commit.id])
            .sorting(sorting)
            .all()?
            .filter(|info| {
                info.as_ref().map_or(true, |info| {
                    info.parent_ids.len() <= max_parents &&
                    info.parent_ids.len() >= min_parents &&
                    // if the list of paths is empty the filter passes.
                    // if paths are provided check that any one of them are
                    // in fact relevant for the current commit.
                    (args.paths.is_empty() || args.paths.iter().any(|path| {
                        // TODO: should make use of the `git2::DiffOptions`
                        //       counterpart in gix for a set of files and also to
                        //       generate diffs. When ready, also make paths resistant
                        //       to illformed UTF8 by not using ".display()".
                        // PERFORMANCE WARNING: What follows is a clever implementation
                        //    that is also **very** slow - do not use on bigger sample
                        //    repositories as this needs native support in `gix` to 
                        //    be fast enough.
                        match repo.rev_parse_single(
                            format!("{}:{}", info.id, path.display()).as_str()
                        ) {
                            // check by parsing the revspec on the path with
                            // the prefix of the tree of the current commit,
                            // vs. the same counterpart but using each of
                            // commit's parents; if any pairs don't match,
                            // this indicates this path was changed in this
                            // commit thus should be included in output.
                            // naturally, root commits have no parents and
                            // by definition whatever paths in there must
                            // have been introduced there, so include them.
                            Ok(oid) => info.parent_ids.is_empty() || info
                                .parent_ids
                                .iter()
                                .any(|id| {
                                    repo.rev_parse_single(
                                        format!("{id}:{}", path.display()).as_str()
                                    ).ok() != Some(oid)
                                }),
                            // no oid for the path resolved with this commit
                            // so this commit can be omitted from output.
                            Err(_) => false,
                        }
                    }))
                })
            })
            .map(|info| -> anyhow::Result<_> {
                let info = info?;
                let commit = info.object()?;
                let commit_ref = commit.decode()?;
                let author = commit_ref.author()?;
                Ok(LogEntryInfo {
                    commit_id: commit.id().to_hex().to_string(),
                    parents: info.parent_ids().map(|id| id.shorten_or_id().to_string()).collect(),
                    author: {
                        let mut buf = Vec::new();
                        author.actor().write_to(&mut buf)?;
                        buf.into()
                    },
                    time: author.time()?.format_or_unix(format::DEFAULT),
                    message: commit_ref.message.to_owned(),
                })
            }),
    );
    if args.reverse {
        let mut results: Vec<_> = log_iter.collect();
        results.reverse();
        log_iter = Box::new(results.into_iter());
    }

    let mut log_iter = log_iter
        .skip(args.skip.unwrap_or_default())
        .take(args.count.unwrap_or(usize::MAX))
        .peekable();

    let mut out = stdout().lock();
    let mut buf = Vec::new();
    while let Some(entry) = log_iter.next() {
        buf.clear();
        let entry = entry?;
        writeln!(buf, "commit {}", entry.commit_id)?;
        if entry.parents.len() > 1 {
            writeln!(buf, "Merge: {}", entry.parents.join(" "))?;
        }
        writeln!(buf, "Author: {}", entry.author)?;
        writeln!(buf, "Date:   {}\n", entry.time)?;
        for line in entry.message.lines() {
            write!(buf, "    ")?;
            buf.write_all(line)?;
            writeln!(buf)?;
        }
        // only include newline if more log entries, mimicking `git log`
        if log_iter.peek().is_some() {
            writeln!(buf)?;
        }
        out.write_all(&buf)?;
    }

    Ok(())
}

struct LogEntryInfo {
    commit_id: String,
    parents: Vec<String>,
    author: BString,
    time: String,
    message: BString,
}
