use crate::{File, Graph, MAX_COMMITS};
use gix_error::Result;
use gix_error::{
    OptionExt, ResourceExhaustionKind, ResultExt, bail, corruption, message, resource_exhaustion, validation,
};
use std::{
    io::{BufRead, BufReader, Read},
    path::Path,
};

/// Instantiate a `Graph` from various sources.
impl Graph {
    /// Instantiate a commit graph from `path` which may be a directory containing graph files or the graph file itself.
    ///
    /// Filesystem errors retain their [`std::io::Error`] source, including when `path` does not exist.
    pub fn at(path: &Path) -> Result<Self> {
        Self::try_from(path)
    }

    /// Open a graph, limiting the memory needed to collect a commit's parents during traversal.
    ///
    /// `alloc_limit_bytes` bounds a conservative capacity estimate for parent buffers derived from
    /// each file's EDGE chunk, including geometric buffer growth. `None` disables the limit.
    /// The mapped graph data itself is not allocated on the heap.
    pub fn at_with_alloc_limit(path: &Path, alloc_limit_bytes: Option<usize>) -> Result<Self> {
        let graph = Self::at(path)?;
        if let Some(limit) = alloc_limit_bytes {
            for file in &graph.files {
                if let Some(edges) = file.extra_edges_data() {
                    // shortcut: estimate from the whole EDGE chunk, scan individual lists if valid graphs hit the limit.
                    // Include the inline parent and spare capacity for traversal buffers that double when growing.
                    let parent_size = std::mem::size_of::<(gix_hash::ObjectId, (u32, i64))>()
                        .max(std::mem::size_of::<Result<gix_hash::ObjectId>>());
                    let parent_bytes = (edges.len() / 4 + 1).checked_mul(2 * parent_size);
                    if parent_bytes.is_none_or(|bytes| bytes > limit) {
                        bail!(resource_exhaustion(
                            ResourceExhaustionKind::AllocationLimit,
                            "Commit-graph parents require more memory than allowed",
                        ));
                    }
                }
            }
        }
        Ok(graph)
    }

    /// Instantiate a commit graph from the directory containing all of its files.
    pub fn from_commit_graphs_dir(path: &Path) -> Result<Self> {
        let commit_graphs_dir = path;
        let chain_file_path = commit_graphs_dir.join("commit-graph-chain");
        let chain_file = std::fs::File::open(&chain_file_path).or_raise(|| {
            message!(
                "Could not open commit-graph chain file at \"{}\"",
                chain_file_path.display()
            )
        })?;
        let mut files = Vec::new();
        let mut reader = BufReader::new(chain_file);
        let mut line = Vec::with_capacity(66);
        loop {
            line.clear();
            // A SHA-256 hex ID and CRLF are the longest possible chain entry.
            let bytes = reader.by_ref().take(67).read_until(b'\n', &mut line).or_raise(|| {
                message!(
                    "Could not read from commit-graph file at \"{}\"",
                    chain_file_path.display()
                )
            })?;
            if bytes == 0 {
                break;
            }
            if line.len() > 66 {
                bail!(corruption("Commit-graph chain entry exceeds the maximum hash length"));
            }
            if line.last() == Some(&b'\n') {
                line.pop();
                if line.last() == Some(&b'\r') {
                    line.pop();
                }
            }
            gix_hash::ObjectId::from_hex(&line)
                .or_raise(|| corruption("Commit-graph chain entry is not a valid object ID"))?;
            let hash = std::str::from_utf8(&line).expect("validated hexadecimal object IDs contain only ASCII");
            if files.len() == 256 {
                bail!(corruption("Commit-graph chain contains more than 256 files"));
            }
            let graph_file_path = commit_graphs_dir.join(format!("graph-{hash}.graph"));
            files.push(File::at(&graph_file_path)?);
        }
        Self::new(files).or_raise(|| {
            message!(
                "Could not assemble commit-graph from chain file at \"{}\"",
                chain_file_path.display()
            )
            .corrupted()
        })
    }

    /// Instantiate a commit graph from a `.git/objects/info/commit-graph` or
    /// `.git/objects/info/commit-graphs/graph-*.graph` file.
    pub fn from_file(path: &Path) -> Result<Self> {
        let file = File::at(path)?;
        Self::new(vec![file])
            .or_raise(|| message!("Could not assemble commit-graph from file at \"{}\"", path.display()).corrupted())
    }

    /// Instantiate a commit graph from an `.git/objects/info` directory.
    pub fn from_info_dir(info_dir: &Path) -> Result<Self> {
        Self::from_file(&info_dir.join("commit-graph"))
            .or_else(|_| Self::from_commit_graphs_dir(&info_dir.join("commit-graphs")))
    }

    /// Create a new commit graph from a list of `files`.
    ///
    /// Invalid file combinations are classified as [`gix_error::Class::Validation`].
    pub fn new(files: Vec<File>) -> Result<Self> {
        if files.len() > 256 {
            bail!(validation("Commit-graph must contain at most 256 files"));
        }
        let mut files = nonempty::NonEmpty::from_vec(files)
            .ok_or_raise(|| validation("Commit-graph must contain at least one file"))?;
        let num_commits: u64 = files.iter().map(|f| u64::from(f.num_commits())).sum();
        if num_commits > u64::from(MAX_COMMITS) {
            bail!(
                "Commit-graph files contain {num_commits} commits altogether, but only {MAX_COMMITS} commits are allowed".validation()
            );
        }

        let mut f1 = files.first();
        for f2 in files.tail() {
            if f1.object_hash() != f2.object_hash() {
                bail!(
                    "Commit-graph files mismatch: \"{path1}\" uses hash {hash1:?}, but \"{path2}\" uses hash {hash2:?}"
                        .validation(),
                    path1 = f1.path().display(),
                    hash1 = f1.object_hash(),
                    path2 = f2.path().display(),
                    hash2 = f2.object_hash(),
                );
            }
            f1 = f2;
        }

        let mut parent_position_limit = 0;
        for file in files.iter_mut() {
            parent_position_limit += file.num_commits();
            file.parent_position_limit = parent_position_limit;
        }
        Ok(Self { files })
    }
}

impl TryFrom<&Path> for Graph {
    type Error = gix_error::Error;

    fn try_from(path: &Path) -> Result<Self> {
        let metadata = path
            .metadata()
            .or_raise(|| message!("Could not access commit-graph path at \"{}\"", path.display()))?;
        if metadata.is_file() {
            // Assume we are looking at `.git/objects/info/commit-graph` or
            // `.git/objects/info/commit-graphs/graph-*.graph`.
            Self::from_file(path)
        } else if metadata.is_dir() {
            if path.join("commit-graph-chain").is_file() {
                Self::from_commit_graphs_dir(path)
            } else {
                Self::from_info_dir(path)
            }
        } else {
            Err(message!(
                "Did not find any files that look like commit graphs at \"{}\"",
                path.display()
            )
            .validation_error())
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn chain_filenames_preserve_hexadecimal_case() -> gix_error::TestResult {
        let repo = gix_testtools::scripted_fixture_writable("single_commit.sh")?;
        let info = repo.path().join(".git/objects/info");
        let graph_path = info.join("commit-graph");
        let checksum = crate::File::at(&graph_path)?.checksum().to_string();
        let graph_data = std::fs::read(&graph_path)?;
        std::fs::remove_file(graph_path)?;
        let directory = info.join("commit-graphs");
        std::fs::create_dir(&directory)?;
        let mixed_case: String = checksum
            .chars()
            .enumerate()
            .map(|(index, character)| {
                if index % 2 == 0 {
                    character.to_ascii_uppercase()
                } else {
                    character
                }
            })
            .collect();
        for hash in [checksum.to_ascii_uppercase(), mixed_case] {
            let filename = format!("graph-{hash}.graph");
            let path = directory.join(&filename);
            std::fs::write(&path, &graph_data)?;
            std::fs::write(directory.join("commit-graph-chain"), format!("{hash}\n"))?;
            let graph = super::Graph::from_commit_graphs_dir(&directory)?;
            assert_eq!(
                graph.files.first().path().file_name(),
                Some(std::ffi::OsStr::new(&filename)),
                "validation must preserve the chain entry's filename spelling even on case-insensitive filesystems"
            );
            graph.verify_integrity(|_| Ok::<_, gix_error::Message>(()))?;
            gix_testtools::git(repo.path(), "commit-graph verify")?;
            std::fs::remove_file(path)?;
        }
        Ok(())
    }
}
