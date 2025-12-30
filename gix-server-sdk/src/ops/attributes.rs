use bstr::{BStr, BString, ByteSlice};

use crate::error::{Result, SdkError};
use crate::RepoHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IgnoreResult {
    pub path: BString,
    pub is_ignored: bool,
    pub pattern: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeValue {
    pub name: String,
    pub state: AttributeState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeState {
    Set,
    Unset,
    Value(String),
    Unspecified,
}

pub fn check_ignore(repo: &RepoHandle, paths: &[impl AsRef<BStr>]) -> Result<Vec<IgnoreResult>> {
    let local = repo.to_local();
    let index = local.index_or_empty().map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut excludes = local
        .excludes(
            &index,
            None,
            gix::worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
        )
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut results = Vec::with_capacity(paths.len());

    for path in paths {
        let path_ref = path.as_ref();
        let is_dir = path_ref.ends_with(b"/");
        let mode = if is_dir {
            Some(gix::index::entry::Mode::DIR)
        } else {
            None
        };

        let platform = excludes
            .at_entry(path_ref, mode)
            .map_err(|e| SdkError::Io(e))?;

        let match_result = platform.matching_exclude_pattern();

        let (is_ignored, pattern, source) = match match_result {
            Some(m) if !m.pattern.is_negative() => {
                let pattern_str = m.pattern.to_string();
                let source_str = m.source.map(|p| p.display().to_string());
                (true, Some(pattern_str), source_str)
            }
            Some(m) => {
                let pattern_str = m.pattern.to_string();
                let source_str = m.source.map(|p| p.display().to_string());
                (false, Some(pattern_str), source_str)
            }
            None => (false, None, None),
        };

        results.push(IgnoreResult {
            path: path_ref.to_owned(),
            is_ignored,
            pattern,
            source,
        });
    }

    Ok(results)
}

pub fn get_attributes(
    repo: &RepoHandle,
    path: impl AsRef<BStr>,
    attrs: &[&str],
) -> Result<Vec<AttributeValue>> {
    let local = repo.to_local();
    let index = local.index_or_empty().map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut attr_stack = local
        .attributes_only(
            &index,
            gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping,
        )
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let path_ref = path.as_ref();
    let is_dir = path_ref.ends_with(b"/");
    let mode = if is_dir {
        Some(gix::index::entry::Mode::DIR)
    } else {
        None
    };

    let mut outcome = attr_stack.selected_attribute_matches(attrs.iter().copied());

    let platform = attr_stack
        .at_entry(path_ref, mode)
        .map_err(|e| SdkError::Io(e))?;

    platform.matching_attributes(&mut outcome);

    let mut results = Vec::with_capacity(attrs.len());

    for m in outcome.iter_selected() {
        let state = match m.assignment.state {
            gix::attrs::StateRef::Set => AttributeState::Set,
            gix::attrs::StateRef::Unset => AttributeState::Unset,
            gix::attrs::StateRef::Value(v) => {
                AttributeState::Value(v.as_bstr().to_str_lossy().into_owned())
            }
            gix::attrs::StateRef::Unspecified => AttributeState::Unspecified,
        };

        results.push(AttributeValue {
            name: m.assignment.name.as_str().to_owned(),
            state,
        });
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bstr::ByteSlice;

    fn create_test_repo() -> Result<(tempfile::TempDir, crate::RepoPool)> {
        let temp_dir = tempfile::tempdir().map_err(SdkError::Io)?;
        let repo_path = temp_dir.path();

        std::process::Command::new("git")
            .args(["init"])
            .current_dir(repo_path)
            .output()
            .map_err(SdkError::Io)?;

        std::fs::write(repo_path.join(".gitignore"), "*.log\n/target/\n")
            .map_err(SdkError::Io)?;

        std::fs::write(
            repo_path.join(".gitattributes"),
            "*.rs text diff=rust\n*.md text\n*.bin binary\n",
        )
        .map_err(SdkError::Io)?;

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(repo_path)
            .output()
            .map_err(SdkError::Io)?;

        let pool = crate::RepoPool::new(crate::SdkConfig::default());

        Ok((temp_dir, pool))
    }

    #[test]
    fn test_check_ignore_basic() -> Result<()> {
        let (temp_dir, pool) = create_test_repo()?;
        let handle = pool.get(temp_dir.path())?;

        let paths: Vec<&BStr> = vec![
            b"foo.log".as_bstr(),
            b"src/main.rs".as_bstr(),
            b"target/debug".as_bstr(),
        ];

        let results = check_ignore(&handle, &paths)?;

        assert_eq!(results.len(), 3);
        assert!(results[0].is_ignored, "*.log should be ignored");
        assert!(!results[1].is_ignored, "*.rs should not be ignored");

        Ok(())
    }

    #[test]
    fn test_get_attributes_basic() -> Result<()> {
        let (temp_dir, pool) = create_test_repo()?;
        let handle = pool.get(temp_dir.path())?;

        let attrs = get_attributes(&handle, b"src/main.rs".as_bstr(), &["text", "diff", "binary"])?;

        assert!(!attrs.is_empty());

        Ok(())
    }
}
