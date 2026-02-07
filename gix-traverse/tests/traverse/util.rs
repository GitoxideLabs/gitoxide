use gix_hash::ObjectId;
use std::path::PathBuf;

pub use gix_testtools::Result;

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

/// Get the path to a fixture directory from a script that creates a single repository.
pub fn fixture(script_name: &str) -> Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only_standalone(script_name)
}

/// Get an object database handle from a fixture script that creates a single repository.
pub fn fixture_odb(script_name: &str) -> Result<gix_odb::Handle> {
    let dir = fixture(script_name)?;
    Ok(gix_odb::at(dir.join(".git").join("objects"))?)
}

/// Get a fixture path and object database for a named sub-repository within a fixture.
pub fn named_fixture(script_name: &str, repo_name: &str) -> Result<(PathBuf, gix_odb::Handle)> {
    let dir = fixture(script_name)?;
    let repo_dir = dir.join(repo_name);
    let odb = gix_odb::at(repo_dir.join(".git").join("objects"))?;
    Ok((repo_dir, odb))
}

/// Load a commit graph if available for the given object store.
pub fn commit_graph(store: &gix_odb::Store) -> Option<gix_commitgraph::Graph> {
    gix_commitgraph::at(store.path().join("info")).ok()
}

/// Execute `git log --oneline --graph --decorate --all` in the given repository
/// and return the output as a string. Useful for snapshot testing.
pub fn git_graph(repo_dir: impl AsRef<std::path::Path>) -> Result<String> {
    git_graph_internal(repo_dir, false)
}

/// Like `git_graph`, but includes commit timestamps (Unix epoch seconds).
/// Use this for tests where commit ordering depends on time.
pub fn git_graph_with_time(repo_dir: impl AsRef<std::path::Path>) -> Result<String> {
    git_graph_internal(repo_dir, true)
}

fn git_graph_internal(repo_dir: impl AsRef<std::path::Path>, with_time: bool) -> Result<String> {
    use gix_object::bstr::{ByteSlice, ByteVec};
    let format = if with_time {
        "--pretty=format:%H %ct%d %s"
    } else {
        "--pretty=format:%H %d %s"
    };
    let out = std::process::Command::new(gix_path::env::exe_invocation())
        .current_dir(repo_dir)
        .args(["log", "--oneline", "--graph", "--decorate", "--all", format])
        .output()?;
    if !out.status.success() {
        return Err(format!("git log failed: {err}", err = out.stderr.to_str_lossy()).into());
    }
    Ok(out.stdout.into_string_lossy())
}

/// Parse commit names to IDs from git log output.
/// Returns a map of commit message (first word) to ObjectId.
pub fn parse_commit_names(repo_path: &std::path::Path) -> Result<std::collections::HashMap<String, ObjectId>> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .args(["log", "--all", "--format=%H %s"])
        .output()?;
    let mut commits = std::collections::HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.split_whitespace();
        if let (Some(hash), Some(name)) = (parts.next(), parts.next()) {
            commits.insert(name.to_string(), hex_to_id(hash));
        }
    }
    Ok(commits)
}

/// Run `git rev-list` with the given arguments and return the resulting commit IDs.
/// Useful for verifying traversal results against git's baseline behavior.
pub fn git_rev_list(repo_path: &std::path::Path, args: &[&str]) -> Result<Vec<ObjectId>> {
    let output = std::process::Command::new("git")
        .current_dir(repo_path)
        .arg("rev-list")
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| hex_to_id(s.trim()))
        .collect())
}
