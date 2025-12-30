use std::collections::VecDeque;

use bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_hash::ObjectId;
use gix_object::{tree::EntryRef, Find, FindExt};
use gix_odb::HeaderExt;
use gix_traverse::tree::{visit::Action, Visit};

use crate::error::{Result, SdkError};
use crate::types::TreeEntry;
use crate::RepoHandle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntryWithPath {
    pub path: BString,
    pub entry: TreeEntry,
}

pub fn get_tree(repo: &RepoHandle, id: ObjectId) -> Result<Vec<TreeEntry>> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let header = local
        .objects
        .header(&id)
        .map_err(|_| SdkError::ObjectNotFound(id))?;

    if header.kind() != gix_object::Kind::Tree {
        return Err(SdkError::InvalidObjectType {
            expected: "tree".to_string(),
            actual: header.kind().to_string(),
        });
    }

    let tree_iter = local
        .objects
        .find_tree_iter(&id, &mut buf)
        .map_err(|_| SdkError::ObjectNotFound(id))?;

    let mut entries = Vec::new();
    for entry_result in tree_iter {
        let entry = entry_result?;
        entries.push(TreeEntry {
            name: entry.filename.to_owned(),
            id: entry.oid.to_owned(),
            mode: entry.mode.into(),
        });
    }

    Ok(entries)
}

pub fn get_tree_entry(
    repo: &RepoHandle,
    tree_id: ObjectId,
    path: &str,
) -> Result<TreeEntry> {
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

    let components: Vec<&[u8]> = path
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| s.as_bytes())
        .collect();

    if components.is_empty() {
        return Err(SdkError::TreeEntryNotFound(path.to_string()));
    }

    let mut buf = Vec::new();
    let mut current_tree_id = tree_id;

    for (idx, component) in components.iter().enumerate() {
        let tree_iter = local
            .objects
            .find_tree_iter(&current_tree_id, &mut buf)
            .map_err(|_| SdkError::ObjectNotFound(current_tree_id))?;

        let mut found_entry = None;
        for entry_result in tree_iter {
            let entry = entry_result?;
            if entry.filename == *component {
                found_entry = Some((entry.oid.to_owned(), entry.mode, entry.filename.to_owned()));
                break;
            }
        }

        match found_entry {
            Some((oid, mode, name)) => {
                if idx == components.len() - 1 {
                    return Ok(TreeEntry {
                        name,
                        id: oid,
                        mode: mode.into(),
                    });
                } else {
                    if !mode.is_tree() {
                        return Err(SdkError::TreeEntryNotFound(path.to_string()));
                    }
                    current_tree_id = oid;
                }
            }
            None => {
                return Err(SdkError::TreeEntryNotFound(path.to_string()));
            }
        }
    }

    Err(SdkError::TreeEntryNotFound(path.to_string()))
}

pub fn list_tree_recursive(
    repo: &RepoHandle,
    tree_id: ObjectId,
    max_depth: Option<usize>,
) -> Result<Vec<TreeEntryWithPath>> {
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

    let mut recorder = DepthLimitedRecorder::new(max_depth);
    let mut state = gix_traverse::tree::depthfirst::State::default();

    gix_traverse::tree::depthfirst(tree_id, &mut state, &local.objects, &mut recorder)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    Ok(recorder.entries)
}

pub fn get_path_at_commit(
    repo: &RepoHandle,
    commit_id: ObjectId,
    path: &str,
) -> Result<TreeEntry> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let commit_data = local
        .objects
        .try_find(&commit_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(commit_id))?;

    if commit_data.kind != gix_object::Kind::Commit {
        return Err(SdkError::InvalidObjectType {
            expected: "commit".to_string(),
            actual: commit_data.kind.to_string(),
        });
    }

    let commit = gix_object::CommitRef::from_bytes(&buf)?;
    let tree_id = commit.tree();

    get_tree_entry(repo, tree_id, path)
}

struct DepthLimitedRecorder {
    path_deque: VecDeque<BString>,
    path: BString,
    max_depth: Option<usize>,
    current_depth: usize,
    entries: Vec<TreeEntryWithPath>,
}

impl DepthLimitedRecorder {
    fn new(max_depth: Option<usize>) -> Self {
        Self {
            path_deque: VecDeque::new(),
            path: BString::default(),
            max_depth,
            current_depth: 0,
            entries: Vec::new(),
        }
    }

    fn pop_element(&mut self) {
        if let Some(pos) = self.path.rfind_byte(b'/') {
            self.path.resize(pos, 0);
        } else {
            self.path.clear();
        }
    }

    fn push_element(&mut self, name: &BStr) {
        if name.is_empty() {
            return;
        }
        if !self.path.is_empty() {
            self.path.push(b'/');
        }
        self.path.push_str(name);
    }

    fn path_clone(&self) -> BString {
        self.path.clone()
    }

    fn calculate_depth(&self) -> usize {
        if self.path.is_empty() {
            0
        } else {
            self.path.find_iter(b"/").count() + 1
        }
    }
}

impl Visit for DepthLimitedRecorder {
    fn pop_back_tracked_path_and_set_current(&mut self) {
        self.path = self.path_deque.pop_back().unwrap_or_default();
        self.current_depth = self.calculate_depth();
    }

    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.path = self
            .path_deque
            .pop_front()
            .expect("every call is matched with push_tracked_path_component");
        self.current_depth = self.calculate_depth();
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

    fn visit_tree(&mut self, entry: &EntryRef<'_>) -> Action {
        let entry_path = self.path_clone();
        let current_depth = self.calculate_depth();

        self.entries.push(TreeEntryWithPath {
            path: entry_path,
            entry: TreeEntry {
                name: entry.filename.to_owned(),
                id: entry.oid.to_owned(),
                mode: entry.mode.into(),
            },
        });

        if let Some(max) = self.max_depth {
            if current_depth >= max {
                return Action::Skip;
            }
        }

        Action::Continue
    }

    fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> Action {
        let entry_path = self.path_clone();

        self.entries.push(TreeEntryWithPath {
            path: entry_path,
            entry: TreeEntry {
                name: entry.filename.to_owned(),
                id: entry.oid.to_owned(),
                mode: entry.mode.into(),
            },
        });

        Action::Continue
    }
}
