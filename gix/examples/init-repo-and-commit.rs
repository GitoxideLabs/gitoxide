// creates a repo with user-specified path (which must not exist)
// adds initial commit with empty tree

use anyhow::Context;
use gix::{config::tree::Author, objs::tree};

fn main() -> anyhow::Result<()> {
    // Note use of args_os:
    // paths may not be UTF-8 encoded and thus can't be forced into a String.
    // gitoxide does not assume encodings that aren't there
    // to match the way git does it as a bare minimum and be just as flexible.
    let git_dir = std::env::args_os()
        .nth(1)
        .context("First argument needs to be the directory to initialize the repository in")?;
    let mut repo = gix::init_bare(git_dir)?;

    println!("Repo (bare): {:?}", repo.git_dir());

    let mut tree = gix::objs::Tree::empty();
    let empty_tree_id = repo.write_object(&tree)?.detach();

    let mut config = repo.config_snapshot_mut();
    config.set_raw_value(&Author::NAME, "Maria Sanchez")?;
    config.set_raw_value(&Author::EMAIL, "maria@example.com")?;
    {
        let repo = config.commit_auto_rollback()?;
        let initial_commit_id = repo.commit("HEAD", "initial commit", empty_tree_id, gix::commit::NO_PARENT_IDS)?;

        println!("initial commit id with empty tree: {initial_commit_id:?}");

        let blob_id = repo.write_blob("hello world")?.into();
        let entry = tree::Entry {
            mode: tree::EntryKind::Blob.into(),
            oid: blob_id,
            filename: "hello.txt".into(),
        };

        tree.entries.push(entry);
        let hello_tree_id = repo.write_object(&tree)?;

        let blob_commit_id = repo.commit("HEAD", "hello commit", hello_tree_id, [initial_commit_id])?;

        println!("commit id for 'hello world' blob: {blob_commit_id:?}");
    }

    Ok(())
}
