use anyhow::{Context, anyhow};
use gix::{diff::blob::ResourceKind, filter::plumbing::driver::apply::Delay, revision::Spec};

use crate::repository::revision::resolve::{BlobFormat, TreeMode};

pub fn display_object(
    repo: &gix::Repository,
    spec: Spec<'_>,
    tree_mode: TreeMode,
    cache: Option<(BlobFormat, &mut gix::diff::blob::Platform)>,
    mut out: impl std::io::Write,
) -> anyhow::Result<()> {
    let id = spec.single().context("rev-spec must resolve to a single object")?;
    let header = id.header()?;
    match header.kind() {
        gix::object::Kind::Tree if matches!(tree_mode, TreeMode::Pretty) => {
            for entry in id.object()?.into_tree().iter() {
                writeln!(out, "{}", entry?)?;
            }
        }
        gix::object::Kind::Blob if cache.is_some() && spec.path_and_mode().is_some() => {
            let (path, mode) = spec.path_and_mode().expect("is present");
            match cache.expect("is some") {
                (BlobFormat::Git, _) => unreachable!("no need for a cache when querying object db"),
                (BlobFormat::Worktree, cache) => {
                    let platform = cache.attr_stack.at_entry(path, Some(mode.into()), &repo.objects)?;
                    let object = id.object()?;
                    let mut converted = cache.filter.worktree_filter.convert_to_worktree(
                        &object.data,
                        path,
                        &mut |_path, attrs| {
                            let _ = platform.matching_attributes(attrs);
                        },
                        Delay::Forbid,
                    )?;
                    std::io::copy(&mut converted, &mut out)?;
                }
                (BlobFormat::Diff | BlobFormat::DiffOrGit, cache) => {
                    cache.set_resource(id.detach(), mode.kind(), path, ResourceKind::OldOrSource, &repo.objects)?;
                    let resource = cache.resource(ResourceKind::OldOrSource).expect("just set");
                    let data = resource
                        .data
                        .as_slice()
                        .ok_or_else(|| anyhow!("Binary data at {path} cannot be diffed"))?;
                    out.write_all(data)?;
                }
            }
        }
        _ => out.write_all(&id.object()?.data)?,
    }
    Ok(())
}

pub(super) mod function {
    use crate::repository::revision::resolve::TreeMode;

    pub fn cat(repo: gix::Repository, revspec: &str, out: impl std::io::Write) -> anyhow::Result<()> {
        super::display_object(&repo, repo.rev_parse(revspec)?, TreeMode::Pretty, None, out)?;
        Ok(())
    }
}
