use bstr::BString;
use gix_hash::ObjectId;
use gix_object::FindExt;

use crate::error::{Result, SdkError};
use crate::pool::RepoHandle;
use crate::types::{CommitInfo, Signature};

pub fn get_commit(repo: &RepoHandle, id: ObjectId) -> Result<CommitInfo> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let commit = local
        .objects
        .find_commit(&id, &mut buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let tree_id = commit.tree();
    let parent_ids: Vec<ObjectId> = commit.parents().collect();
    let author_sig = commit.author().map_err(|e| SdkError::Git(Box::new(e)))?;
    let committer_sig = commit.committer().map_err(|e| SdkError::Git(Box::new(e)))?;

    Ok(CommitInfo {
        id,
        tree_id,
        parent_ids,
        author: Signature {
            name: author_sig.name.into(),
            email: author_sig.email.into(),
            time: author_sig.time().map(|t| t.seconds).unwrap_or(0),
        },
        committer: Signature {
            name: committer_sig.name.into(),
            email: committer_sig.email.into(),
            time: committer_sig.time().map(|t| t.seconds).unwrap_or(0),
        },
        message: BString::from(commit.message),
    })
}

pub fn log(repo: &RepoHandle, start: ObjectId, limit: Option<usize>) -> Result<Vec<CommitInfo>> {
    let local = repo.to_local();

    let walk = gix_traverse::commit::Simple::new([start], &local.objects)
        .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
            gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut commits = Vec::new();
    let mut buf = Vec::new();

    for info in walk {
        let info = info.map_err(|e| SdkError::Git(Box::new(e)))?;

        let commit = local
            .objects
            .find_commit(&info.id, &mut buf)
            .map_err(|e| SdkError::Git(Box::new(e)))?;

        let tree_id = commit.tree();
        let parent_ids: Vec<ObjectId> = commit.parents().collect();
        let author_sig = commit.author().map_err(|e| SdkError::Git(Box::new(e)))?;
        let committer_sig = commit.committer().map_err(|e| SdkError::Git(Box::new(e)))?;

        commits.push(CommitInfo {
            id: info.id,
            tree_id,
            parent_ids,
            author: Signature {
                name: author_sig.name.into(),
                email: author_sig.email.into(),
                time: author_sig.time().map(|t| t.seconds).unwrap_or(0),
            },
            committer: Signature {
                name: committer_sig.name.into(),
                email: committer_sig.email.into(),
                time: committer_sig.time().map(|t| t.seconds).unwrap_or(0),
            },
            message: BString::from(commit.message),
        });

        if let Some(max) = limit {
            if commits.len() >= max {
                break;
            }
        }
    }

    Ok(commits)
}

pub fn log_with_path(
    repo: &RepoHandle,
    start: ObjectId,
    path: &str,
    limit: Option<usize>,
) -> Result<Vec<CommitInfo>> {
    let local = repo.to_local();
    let path_bytes = path.as_bytes();

    let walk = gix_traverse::commit::Simple::new([start], &local.objects)
        .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
            gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut commits = Vec::new();
    let mut buf = Vec::new();
    let mut diff_state = gix_diff::tree::State::default();

    for info in walk {
        let info = info.map_err(|e| SdkError::Git(Box::new(e)))?;

        let commit = local
            .objects
            .find_commit(&info.id, &mut buf)
            .map_err(|e| SdkError::Git(Box::new(e)))?;

        let tree_id = commit.tree();

        let modified_path = if info.parent_ids.is_empty() {
            tree_contains_path(&local.objects, tree_id, path_bytes)?
        } else {
            let mut found = false;
            for parent_id in &info.parent_ids {
                let mut parent_buf = Vec::new();
                let parent_commit = local
                    .objects
                    .find_commit(parent_id, &mut parent_buf)
                    .map_err(|e| SdkError::Git(Box::new(e)))?;
                let parent_tree_id = parent_commit.tree();

                if path_changed_between_trees(
                    &local.objects,
                    parent_tree_id,
                    tree_id,
                    path_bytes,
                    &mut diff_state,
                )? {
                    found = true;
                    break;
                }
            }
            found
        };

        if modified_path {
            let author_sig = commit.author().map_err(|e| SdkError::Git(Box::new(e)))?;
            let committer_sig = commit.committer().map_err(|e| SdkError::Git(Box::new(e)))?;

            commits.push(CommitInfo {
                id: info.id,
                tree_id,
                parent_ids: info.parent_ids.to_vec(),
                author: Signature {
                    name: author_sig.name.into(),
                    email: author_sig.email.into(),
                    time: author_sig.time().map(|t| t.seconds).unwrap_or(0),
                },
                committer: Signature {
                    name: committer_sig.name.into(),
                    email: committer_sig.email.into(),
                    time: committer_sig.time().map(|t| t.seconds).unwrap_or(0),
                },
                message: BString::from(commit.message),
            });

            if let Some(max) = limit {
                if commits.len() >= max {
                    break;
                }
            }
        }
    }

    Ok(commits)
}

fn tree_contains_path(
    objects: &impl gix_object::Find,
    tree_id: ObjectId,
    path: &[u8],
) -> Result<bool> {
    let mut buf = Vec::new();
    let mut current_tree_id = tree_id;

    for component in path.split(|&b| b == b'/') {
        if component.is_empty() {
            continue;
        }

        let tree = objects
            .find_tree_iter(&current_tree_id, &mut buf)
            .map_err(|e| SdkError::Git(Box::new(e)))?;

        let mut found = None;
        for entry in tree {
            let entry = entry.map_err(|e| SdkError::Git(Box::new(e)))?;
            if entry.filename == component {
                found = Some((entry.oid.to_owned(), entry.mode.is_tree()));
                break;
            }
        }

        match found {
            Some((oid, is_tree)) => {
                if is_tree {
                    current_tree_id = oid;
                } else {
                    return Ok(true);
                }
            }
            None => return Ok(false),
        }
    }

    Ok(true)
}

fn path_changed_between_trees(
    objects: &impl gix_object::Find,
    lhs_tree: ObjectId,
    rhs_tree: ObjectId,
    path: &[u8],
    state: &mut gix_diff::tree::State,
) -> Result<bool> {
    if lhs_tree == rhs_tree {
        return Ok(false);
    }

    let mut lhs_buf = Vec::new();
    let mut rhs_buf = Vec::new();

    let lhs_iter = objects
        .find_tree_iter(&lhs_tree, &mut lhs_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;
    let rhs_iter = objects
        .find_tree_iter(&rhs_tree, &mut rhs_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut recorder = PathChangeRecorder {
        target_path: path.to_vec(),
        current_path: Vec::new(),
        changed: false,
    };

    gix_diff::tree(lhs_iter, rhs_iter, state, objects, &mut recorder)?;

    Ok(recorder.changed)
}

struct PathChangeRecorder {
    target_path: Vec<u8>,
    current_path: Vec<u8>,
    changed: bool,
}

impl gix_diff::tree::Visit for PathChangeRecorder {
    fn pop_front_tracked_path_and_set_current(&mut self) {}

    fn push_back_tracked_path_component(&mut self, component: &bstr::BStr) {
        if !self.current_path.is_empty() {
            self.current_path.push(b'/');
        }
        self.current_path.extend_from_slice(component);
    }

    fn push_path_component(&mut self, component: &bstr::BStr) {
        if !self.current_path.is_empty() {
            self.current_path.push(b'/');
        }
        self.current_path.extend_from_slice(component);
    }

    fn pop_path_component(&mut self) {
        if let Some(pos) = self.current_path.iter().rposition(|&b| b == b'/') {
            self.current_path.truncate(pos);
        } else {
            self.current_path.clear();
        }
    }

    fn visit(&mut self, _change: gix_diff::tree::visit::Change) -> gix_diff::tree::visit::Action {
        if self.current_path == self.target_path
            || self.target_path.starts_with(&self.current_path)
            || self.current_path.starts_with(&self.target_path)
        {
            self.changed = true;
            return gix_diff::tree::visit::Action::Cancel;
        }
        gix_diff::tree::visit::Action::Continue
    }
}

pub fn merge_base(
    repo: &RepoHandle,
    commit1: ObjectId,
    commit2: ObjectId,
) -> Result<ObjectId> {
    let local = repo.to_local();
    let cache = local.commit_graph_if_enabled().ok().flatten();
    let mut graph = gix_revwalk::Graph::new(&local.objects, cache.as_ref());

    let bases = gix_revision::merge_base(commit1, &[commit2], &mut graph)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    match bases {
        Some(ids) if !ids.is_empty() => Ok(ids[0]),
        _ => Err(SdkError::Operation(format!(
            "no merge base found between {} and {}",
            commit1, commit2
        ))),
    }
}

pub fn is_ancestor(repo: &RepoHandle, ancestor: ObjectId, descendant: ObjectId) -> Result<bool> {
    if ancestor == descendant {
        return Ok(true);
    }

    let local = repo.to_local();

    let walk = gix_traverse::commit::Simple::new([descendant], &local.objects)
        .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
            gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    for info in walk {
        let info = info.map_err(|e| SdkError::Git(Box::new(e)))?;
        if info.id == ancestor {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn count_commits(
    repo: &RepoHandle,
    start: ObjectId,
    stop: Option<ObjectId>,
) -> Result<usize> {
    let local = repo.to_local();

    let walk = gix_traverse::commit::Simple::new([start], &local.objects)
        .sorting(gix_traverse::commit::simple::Sorting::ByCommitTime(
            gix_traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut count = 0;

    for info in walk {
        let info = info.map_err(|e| SdkError::Git(Box::new(e)))?;

        if let Some(stop_id) = stop {
            if info.id == stop_id {
                break;
            }
        }

        count += 1;
    }

    Ok(count)
}
