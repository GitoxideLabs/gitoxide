use std::collections::VecDeque;

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_hash::ObjectId;
use gix_object::{tree::EntryRef, Find, FindExt};
use gix_odb::HeaderExt;
use gix_traverse::tree::{visit::Action, Visit};
use regex::{Regex, RegexBuilder};

use crate::error::{Result, SdkError};
use crate::RepoHandle;

#[derive(Debug, Clone)]
pub struct GrepOptions {
    pub case_insensitive: bool,
    pub max_matches_per_file: Option<usize>,
    pub include_binary: bool,
    pub path_pattern: Option<String>,
}

impl Default for GrepOptions {
    fn default() -> Self {
        GrepOptions {
            case_insensitive: false,
            max_matches_per_file: None,
            include_binary: false,
            path_pattern: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrepMatch {
    pub path: BString,
    pub blob_id: ObjectId,
    pub matches: Vec<LineMatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMatch {
    pub line_number: u32,
    pub content: BString,
    pub match_start: usize,
    pub match_end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickaxeMatch {
    pub commit_id: ObjectId,
    pub path: BString,
    pub change_type: PickaxeChangeType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickaxeChangeType {
    Added,
    Removed,
}

fn is_binary(content: &[u8]) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

fn build_regex(pattern: &str, case_insensitive: bool) -> Result<Regex> {
    RegexBuilder::new(pattern)
        .case_insensitive(case_insensitive)
        .build()
        .map_err(|e| SdkError::Operation(format!("invalid regex pattern: {}", e)))
}

fn matches_glob(path: &BStr, pattern: &str) -> bool {
    let path_str = match path.to_str() {
        Ok(s) => s,
        Err(_) => return false,
    };

    if !pattern.contains('/') {
        let basename = path_str.rsplit('/').next().unwrap_or(path_str);
        return glob_match_segment(pattern, basename);
    }

    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let path_parts: Vec<&str> = path_str.split('/').collect();

    glob_match_parts(&pattern_parts, &path_parts)
}

fn glob_match_parts(pattern_parts: &[&str], path_parts: &[&str]) -> bool {
    if pattern_parts.is_empty() && path_parts.is_empty() {
        return true;
    }
    if pattern_parts.is_empty() {
        return false;
    }

    let pat = pattern_parts[0];
    if pat == "**" {
        if pattern_parts.len() == 1 {
            return true;
        }
        for i in 0..=path_parts.len() {
            if glob_match_parts(&pattern_parts[1..], &path_parts[i..]) {
                return true;
            }
        }
        return false;
    }

    if path_parts.is_empty() {
        return false;
    }

    if glob_match_segment(pat, path_parts[0]) {
        glob_match_parts(&pattern_parts[1..], &path_parts[1..])
    } else {
        false
    }
}

fn glob_match_segment(pattern: &str, segment: &str) -> bool {
    let pattern_chars: Vec<char> = pattern.chars().collect();
    let segment_chars: Vec<char> = segment.chars().collect();
    glob_match_chars(&pattern_chars, &segment_chars)
}

fn glob_match_chars(pattern: &[char], segment: &[char]) -> bool {
    if pattern.is_empty() && segment.is_empty() {
        return true;
    }
    if pattern.is_empty() {
        return false;
    }

    match pattern[0] {
        '*' => {
            if pattern.len() == 1 {
                return true;
            }
            for i in 0..=segment.len() {
                if glob_match_chars(&pattern[1..], &segment[i..]) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if segment.is_empty() {
                false
            } else {
                glob_match_chars(&pattern[1..], &segment[1..])
            }
        }
        c => {
            if segment.is_empty() || segment[0] != c {
                false
            } else {
                glob_match_chars(&pattern[1..], &segment[1..])
            }
        }
    }
}

pub fn grep_blob(
    repo: &RepoHandle,
    blob_id: ObjectId,
    pattern: &str,
) -> Result<Vec<LineMatch>> {
    grep_blob_with_options(repo, blob_id, pattern, false, false, None)
}

fn grep_blob_with_options(
    repo: &RepoHandle,
    blob_id: ObjectId,
    pattern: &str,
    case_insensitive: bool,
    include_binary: bool,
    max_matches: Option<usize>,
) -> Result<Vec<LineMatch>> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let data = local
        .objects
        .try_find(&blob_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(blob_id))?;

    if data.kind != gix_object::Kind::Blob {
        return Err(SdkError::InvalidObjectType {
            expected: "blob".to_string(),
            actual: data.kind.to_string(),
        });
    }

    let content = &buf;

    if !include_binary && is_binary(content) {
        return Ok(Vec::new());
    }

    let regex = build_regex(pattern, case_insensitive)?;
    let mut matches = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_str = match line.to_str() {
            Ok(s) => s,
            Err(_) => continue,
        };

        if let Some(m) = regex.find(line_str) {
            matches.push(LineMatch {
                line_number: (line_idx + 1) as u32,
                content: BString::from(line),
                match_start: m.start(),
                match_end: m.end(),
            });

            if let Some(max) = max_matches {
                if matches.len() >= max {
                    break;
                }
            }
        }
    }

    Ok(matches)
}

pub fn grep_tree(
    repo: &RepoHandle,
    tree_id: ObjectId,
    pattern: &str,
    options: &GrepOptions,
) -> Result<Vec<GrepMatch>> {
    let local = repo.to_local();

    let header = local
        .objects
        .header(&tree_id)
        .map_err(|_| SdkError::ObjectNotFound(tree_id))?;

    if header.kind() != gix_object::Kind::Tree {
        return Err(SdkError::InvalidObjectType {
            expected: "tree".to_string(),
            actual: header.kind().to_string(),
        });
    }

    let mut collector = BlobCollector::new(options.path_pattern.clone());
    let mut state = gix_traverse::tree::depthfirst::State::default();

    gix_traverse::tree::depthfirst(tree_id, &mut state, &local.objects, &mut collector)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let regex = build_regex(pattern, options.case_insensitive)?;
    let mut results = Vec::new();

    for (path, blob_id) in collector.blobs {
        let mut buf = Vec::new();
        let data = local
            .objects
            .try_find(&blob_id, &mut buf)
            .map_err(|e| SdkError::Git(e))?;

        let content = match data {
            Some(d) if d.kind == gix_object::Kind::Blob => &buf,
            _ => continue,
        };

        if !options.include_binary && is_binary(content) {
            continue;
        }

        let mut line_matches = Vec::new();

        for (line_idx, line) in content.lines().enumerate() {
            let line_str = match line.to_str() {
                Ok(s) => s,
                Err(_) => continue,
            };

            if let Some(m) = regex.find(line_str) {
                line_matches.push(LineMatch {
                    line_number: (line_idx + 1) as u32,
                    content: BString::from(line),
                    match_start: m.start(),
                    match_end: m.end(),
                });

                if let Some(max) = options.max_matches_per_file {
                    if line_matches.len() >= max {
                        break;
                    }
                }
            }
        }

        if !line_matches.is_empty() {
            results.push(GrepMatch {
                path: path.clone(),
                blob_id,
                matches: line_matches,
            });
        }
    }

    Ok(results)
}

pub fn grep_commit(
    repo: &RepoHandle,
    commit_id: ObjectId,
    pattern: &str,
    options: &GrepOptions,
) -> Result<Vec<GrepMatch>> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let commit = local
        .objects
        .find_commit(&commit_id, &mut buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let tree_id = commit.tree();
    grep_tree(repo, tree_id, pattern, options)
}

pub fn pickaxe_search(
    repo: &RepoHandle,
    start_commit: ObjectId,
    pattern: &str,
    limit: Option<usize>,
) -> Result<Vec<PickaxeMatch>> {
    let local = repo.to_local();
    let regex = build_regex(pattern, false)?;

    let walk = gix_traverse::commit::Simple::new([start_commit], &local.objects)
        .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
            gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut results = Vec::new();
    let mut commit_count = 0;
    let max_commits = limit.unwrap_or(1000);
    let mut diff_state = gix_diff::tree::State::default();

    for info in walk {
        if commit_count >= max_commits {
            break;
        }
        commit_count += 1;

        let info = info.map_err(|e| SdkError::Git(Box::new(e)))?;
        let commit_id = info.id;

        let mut buf = Vec::new();
        let commit = local
            .objects
            .find_commit(&commit_id, &mut buf)
            .map_err(|e| SdkError::Git(Box::new(e)))?;
        let tree_id = commit.tree();

        if info.parent_ids.is_empty() {
            let matches = find_pattern_in_tree(&local.objects, tree_id, &regex)?;
            for path in matches {
                results.push(PickaxeMatch {
                    commit_id,
                    path,
                    change_type: PickaxeChangeType::Added,
                });
            }
        } else {
            for parent_id in &info.parent_ids {
                let mut parent_buf = Vec::new();
                let parent_commit = local
                    .objects
                    .find_commit(parent_id, &mut parent_buf)
                    .map_err(|e| SdkError::Git(Box::new(e)))?;
                let parent_tree_id = parent_commit.tree();

                let changes = diff_trees_for_pickaxe(
                    &local.objects,
                    parent_tree_id,
                    tree_id,
                    &mut diff_state,
                )?;

                for (path, old_id, new_id) in changes {
                    let old_count = count_pattern_in_blob(&local.objects, old_id, &regex)?;
                    let new_count = count_pattern_in_blob(&local.objects, new_id, &regex)?;

                    if old_count != new_count {
                        let change_type = if new_count > old_count {
                            PickaxeChangeType::Added
                        } else {
                            PickaxeChangeType::Removed
                        };

                        results.push(PickaxeMatch {
                            commit_id,
                            path,
                            change_type,
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

fn find_pattern_in_tree<O: Find>(
    objects: &O,
    tree_id: ObjectId,
    regex: &Regex,
) -> Result<Vec<BString>> {
    let mut collector = BlobCollector::new(None);
    let mut state = gix_traverse::tree::depthfirst::State::default();

    gix_traverse::tree::depthfirst(tree_id, &mut state, objects, &mut collector)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut paths = Vec::new();
    for (path, blob_id) in collector.blobs {
        let count = count_pattern_in_blob(objects, Some(blob_id), regex)?;
        if count > 0 {
            paths.push(path);
        }
    }

    Ok(paths)
}

fn count_pattern_in_blob<O: Find>(
    objects: &O,
    blob_id: Option<ObjectId>,
    regex: &Regex,
) -> Result<usize> {
    let blob_id = match blob_id {
        Some(id) => id,
        None => return Ok(0),
    };

    let mut buf = Vec::new();
    let data = objects
        .try_find(&blob_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?;

    match data {
        Some(d) if d.kind == gix_object::Kind::Blob => {}
        _ => return Ok(0),
    }

    if is_binary(&buf) {
        return Ok(0);
    }

    let content = match buf.to_str() {
        Ok(s) => s,
        Err(_) => return Ok(0),
    };

    Ok(regex.find_iter(content).count())
}

fn diff_trees_for_pickaxe<O: Find>(
    objects: &O,
    old_tree_id: ObjectId,
    new_tree_id: ObjectId,
    state: &mut gix_diff::tree::State,
) -> Result<Vec<(BString, Option<ObjectId>, Option<ObjectId>)>> {
    let mut old_buf = Vec::new();
    let mut new_buf = Vec::new();

    let old_tree_iter = objects
        .find_tree_iter(&old_tree_id, &mut old_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;
    let new_tree_iter = objects
        .find_tree_iter(&new_tree_id, &mut new_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut recorder = gix_diff::tree::Recorder::default();
    gix_diff::tree(old_tree_iter, new_tree_iter, state, objects, &mut recorder)?;

    let mut changes = Vec::new();

    for change in recorder.records {
        use gix_diff::tree::recorder::Change;
        match change {
            Change::Addition {
                entry_mode,
                oid,
                path,
                ..
            } => {
                if !entry_mode.is_tree() && entry_mode.is_blob() {
                    changes.push((path, None, Some(oid)));
                }
            }
            Change::Deletion {
                entry_mode,
                oid,
                path,
                ..
            } => {
                if !entry_mode.is_tree() && entry_mode.is_blob() {
                    changes.push((path, Some(oid), None));
                }
            }
            Change::Modification {
                previous_entry_mode,
                previous_oid,
                entry_mode,
                oid,
                path,
            } => {
                if !entry_mode.is_tree()
                    && entry_mode.is_blob()
                    && previous_entry_mode.is_blob()
                {
                    changes.push((path, Some(previous_oid), Some(oid)));
                }
            }
        }
    }

    Ok(changes)
}

struct BlobCollector {
    path_deque: VecDeque<BString>,
    path: BString,
    blobs: Vec<(BString, ObjectId)>,
    path_pattern: Option<String>,
}

impl BlobCollector {
    fn new(path_pattern: Option<String>) -> Self {
        Self {
            path_deque: VecDeque::new(),
            path: BString::default(),
            blobs: Vec::new(),
            path_pattern,
        }
    }

    fn pop_element(&mut self) {
        if let Some(pos) = self.path.rfind_byte(b'/') {
            self.path.truncate(pos);
        } else {
            self.path.clear();
        }
    }

    fn push_element(&mut self, name: &BStr) {
        if name.is_empty() {
            return;
        }
        if !self.path.is_empty() {
            self.path.push_byte(b'/');
        }
        self.path.extend_from_slice(name);
    }

    fn path_clone(&self) -> BString {
        self.path.clone()
    }

    fn should_include_path(&self, path: &BStr) -> bool {
        match &self.path_pattern {
            Some(pattern) => matches_glob(path, pattern),
            None => true,
        }
    }
}

impl Visit for BlobCollector {
    fn pop_back_tracked_path_and_set_current(&mut self) {
        self.path = self.path_deque.pop_back().unwrap_or_default();
    }

    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.path = self
            .path_deque
            .pop_front()
            .expect("every call is matched with push_tracked_path_component");
    }

    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        self.push_element(component);
        self.path_deque.push_back(self.path.clone());
    }

    fn push_path_component(&mut self, component: &BStr) {
        self.push_element(component);
    }

    fn pop_path_component(&mut self) {
        self.pop_element();
    }

    fn visit_tree(&mut self, _entry: &EntryRef<'_>) -> Action {
        Action::Continue
    }

    fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> Action {
        if entry.mode.is_blob() {
            let entry_path = self.path_clone();
            if self.should_include_path(entry_path.as_ref()) {
                self.blobs.push((entry_path, entry.oid.to_owned()));
            }
        }
        Action::Continue
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary() {
        assert!(!is_binary(b"hello world"));
        assert!(is_binary(b"hello\x00world"));
        assert!(!is_binary(b""));
    }

    #[test]
    fn test_glob_matching() {
        assert!(matches_glob(BStr::new(b"src/main.rs"), "*.rs"));
        assert!(matches_glob(BStr::new(b"src/main.rs"), "**/*.rs"));
        assert!(matches_glob(BStr::new(b"src/lib/mod.rs"), "**/*.rs"));
        assert!(!matches_glob(BStr::new(b"src/main.txt"), "*.rs"));
        assert!(matches_glob(BStr::new(b"test.rs"), "*.rs"));
        assert!(matches_glob(BStr::new(b"src/foo/bar.rs"), "src/**/*.rs"));
        assert!(!matches_glob(BStr::new(b"lib/foo.rs"), "src/**/*.rs"));
    }

    #[test]
    fn test_grep_options_default() {
        let opts = GrepOptions::default();
        assert!(!opts.case_insensitive);
        assert!(opts.max_matches_per_file.is_none());
        assert!(!opts.include_binary);
        assert!(opts.path_pattern.is_none());
    }
}
