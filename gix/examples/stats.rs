use std::io::Write;

use gix::{prelude::ObjectIdExt, Reference};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut repo = gix::discover(".")?;
    println!("Repo: {}", repo.workdir().unwrap_or_else(|| repo.git_dir()).display());
    let mut max_parents = 0;
    let mut avg_parents = 0;
    repo.object_cache_size(32 * 1024);
    let mut most_recent_commit_id = None;
    let num_commits = repo
        .head()?
        .try_into_peeled_id()?
        .ok_or("Cannot provide meaningful stats on empty repos")?
        .ancestors()
        .all()?
        .map_while(Result::ok)
        .inspect(|commit| {
            if most_recent_commit_id.is_none() {
                most_recent_commit_id = Some(commit.id);
            }
            avg_parents += commit.parent_ids.len();
            max_parents = max_parents.max(commit.parent_ids.len());
        })
        .count();
    println!("Num Commits: {num_commits}");
    println!("Max parents: {max_parents}");
    println!("Avg parents: {}", avg_parents / num_commits);

    println!("Most recent commit message");

    let object = most_recent_commit_id.expect("already checked").attach(&repo).object()?;
    let commit = object.into_commit();
    println!("{}", commit.message_raw()?);
    std::io::stdout().flush()?;

    let tree = commit.tree()?;

    let mut delegate = visit::Tree::new(repo.clone());
    tree.traverse().breadthfirst(&mut delegate)?;

    println!("num trees: {}", delegate.num_trees);
    println!("num blobs: {}", delegate.num_blobs);
    println!("num blobs_executable: {}", delegate.num_blobs_exec);
    println!("num links: {}", delegate.num_links);
    println!("num submodules: {}", delegate.num_submodules);
    println!("total size in bytes: {}\n", delegate.num_bytes);

    // let num_branches = repo.branches()?;
    // let num_branches = repo.branches.remote("origin")?;
    let num_branches = repo.references()?.local_branches()?.count();
    let num_remote_branches = repo.references()?.remote_branches()?.count();
    let num_tags = repo.references()?.tags()?.count();
    let broken_refs = repo
        .references()?
        .all()?
        .filter_map(Result::ok)
        .filter_map(|r: Reference| r.into_fully_peeled_id().err())
        .count();
    let inaccessible_refs = repo.references()?.all()?.filter(Result::is_err).count();

    println!("num local branches: {num_branches}");
    println!("num remote branches: {num_remote_branches}");
    println!("num tags: {num_tags}");
    println!("refs with inaccessible objects: {broken_refs}");
    println!("inaccessible refs: {inaccessible_refs}");

    Ok(())
}

mod visit {
    use gix_hash::oid;
    use gix_object::{bstr::BStr, tree::EntryRef};
    use gix_traverse::tree::visit::Action;

    pub(crate) struct Tree {
        pub num_trees: usize,
        pub num_links: usize,
        pub num_blobs: usize,
        pub num_blobs_exec: usize,
        pub num_submodules: usize,
        pub num_bytes: u64,
        pub repo: gix::Repository,
    }

    impl Tree {
        pub fn new(repo: gix::Repository) -> Self {
            Tree {
                num_trees: 0,
                num_links: 0,
                num_blobs: 0,
                num_blobs_exec: 0,
                num_submodules: 0,
                num_bytes: 0,
                repo,
            }
        }

        pub(crate) fn count_bytes(&mut self, oid: &oid) {
            if let Ok(obj) = self.repo.find_object(oid) {
                self.num_bytes += obj.data.len() as u64;
            }
        }
    }
    impl gix_traverse::tree::Visit for Tree {
        fn pop_back_tracked_path_and_set_current(&mut self) {}

        fn pop_front_tracked_path_and_set_current(&mut self) {}

        fn push_back_tracked_path_component(&mut self, _component: &BStr) {}

        fn push_path_component(&mut self, _component: &BStr) {}

        fn pop_path_component(&mut self) {}

        fn visit_tree(&mut self, _entry: &EntryRef<'_>) -> Action {
            self.num_trees += 1;
            Action::Continue(true)
        }

        fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> Action {
            use gix::objs::tree::EntryKind::*;
            match entry.mode.kind() {
                Commit => self.num_submodules += 1,
                Blob => {
                    self.count_bytes(entry.oid);
                    self.num_blobs += 1;
                }
                BlobExecutable => {
                    self.count_bytes(entry.oid);
                    self.num_blobs_exec += 1;
                }
                Link => self.num_links += 1,
                Tree => unreachable!("BUG"),
            }
            Action::Continue(true)
        }
    }
}
