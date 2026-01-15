use crate::{File, Graph, MAX_COMMITS};
use gix_error::{message, ErrorExt, Exn, Message, ResultExt};
use std::{
    io::{BufRead, BufReader},
    path::Path,
};

/// Instantiate a `Graph` from various sources.
impl Graph {
    /// Instantiate a commit graph from `path` which may be a directory containing graph files or the graph file itself.
    pub fn at(path: &Path) -> Result<Self, Exn<Message>> {
        Self::try_from(path)
    }

    /// Instantiate a commit graph from the directory containing all of its files.
    pub fn from_commit_graphs_dir(path: &Path) -> Result<Self, Exn<Message>> {
        let commit_graphs_dir = path;
        let chain_file_path = commit_graphs_dir.join("commit-graph-chain");
        let chain_file = std::fs::File::open(&chain_file_path).or_raise(|| {
            message!(
                "Could not open commit-graph chain file at '{}'",
                chain_file_path.display()
            )
        })?;
        let mut files = Vec::new();
        for line in BufReader::new(chain_file).lines() {
            let hash = line.or_raise(|| {
                message!(
                    "Could not read from commit-graph file at '{}'",
                    chain_file_path.display()
                )
            })?;
            let graph_file_path = commit_graphs_dir.join(format!("graph-{hash}.graph"));
            files.push(
                File::at(&graph_file_path)
                    .or_raise(|| message!("Could not open commit-graph file at '{}'", chain_file_path.display()))?,
            );
        }
        Ok(Self::new(files)?)
    }

    /// Instantiate a commit graph from a `.git/objects/info/commit-graph` or
    /// `.git/objects/info/commit-graphs/graph-*.graph` file.
    pub fn from_file(path: &Path) -> Result<Self, Exn<Message>> {
        let file = File::at(path).or_raise(|| message!("Could not open commit-graph file at '{}'", path.display()))?;
        Ok(Self::new(vec![file])?)
    }

    /// Instantiate a commit graph from an `.git/objects/info` directory.
    pub fn from_info_dir(info_dir: &Path) -> Result<Self, Exn<Message>> {
        Self::from_file(&info_dir.join("commit-graph"))
            .or_else(|_| Self::from_commit_graphs_dir(&info_dir.join("commit-graphs")))
    }

    /// Create a new commit graph from a list of `files`.
    pub fn new(files: Vec<File>) -> Result<Self, Message> {
        let num_commits: u64 = files.iter().map(|f| u64::from(f.num_commits())).sum();
        if num_commits > u64::from(MAX_COMMITS) {
            return Err(message!(
                "Commit-graph files contain {} commits altogether, but only {} commits are allowed",
                num_commits,
                MAX_COMMITS
            ));
        }

        for window in files.windows(2) {
            let f1 = &window[0];
            let f2 = &window[1];
            if f1.object_hash() != f2.object_hash() {
                return Err(message!(
                    "Commit-graph files mismatch: '{path1}' uses hash {hash1:?}, but '{path2}' uses hash {hash2:?}",
                    path1 = f1.path().display(),
                    hash1 = f1.object_hash(),
                    path2 = f2.path().display(),
                    hash2 = f2.object_hash(),
                ));
            }
        }

        Ok(Self { files })
    }
}

impl TryFrom<&Path> for Graph {
    type Error = Exn<Message>;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        if path.is_file() {
            // Assume we are looking at `.git/objects/info/commit-graph` or
            // `.git/objects/info/commit-graphs/graph-*.graph`.
            Self::from_file(path)
        } else if path.is_dir() {
            if path.join("commit-graph-chain").is_file() {
                Self::from_commit_graphs_dir(path)
            } else {
                Self::from_info_dir(path)
            }
        } else {
            Err(message!(
                "Did not find any files that look like commit graphs at '{}'",
                path.display()
            )
            .raise())
        }
    }
}
