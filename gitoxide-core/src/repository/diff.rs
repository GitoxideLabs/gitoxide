use anyhow::Context;
use gix::diff::blob::unified_diff::ConsumeBinaryHunk;
use gix::{
    bstr::{BString, ByteSlice},
    diff::blob::{intern::TokenSource, unified_diff::ContextSize, UnifiedDiff},
    objs::tree::EntryMode,
    odb::store::RefreshMode,
    prelude::ObjectIdExt,
    ObjectId,
};

pub fn tree(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    old_treeish: BString,
    new_treeish: BString,
) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));
    repo.objects.refresh = RefreshMode::Never;

    let old_tree_id = repo.rev_parse_single(old_treeish.as_bstr())?;
    let new_tree_id = repo.rev_parse_single(new_treeish.as_bstr())?;

    let old_tree = old_tree_id.object()?.peel_to_tree()?;
    let new_tree = new_tree_id.object()?.peel_to_tree()?;

    let changes = repo.diff_tree_to_tree(&old_tree, &new_tree, None)?;

    writeln!(
        out,
        "Diffing trees `{old_treeish}` ({old_tree_id}) -> `{new_treeish}` ({new_tree_id})\n"
    )?;
    write_changes(&repo, out, changes)?;

    Ok(())
}

fn write_changes(
    repo: &gix::Repository,
    mut out: impl std::io::Write,
    changes: Vec<gix::diff::tree_with_rewrites::Change>,
) -> Result<(), std::io::Error> {
    for change in changes {
        match change {
            gix::diff::tree_with_rewrites::Change::Addition {
                location,
                id,
                entry_mode,
                ..
            } => {
                writeln!(out, "A: {}", typed_location(location, entry_mode))?;
                writeln!(out, "  {}", id.attach(repo).shorten_or_id())?;
                writeln!(out, "  -> {entry_mode:o}")?;
            }
            gix::diff::tree_with_rewrites::Change::Deletion {
                location,
                id,
                entry_mode,
                ..
            } => {
                writeln!(out, "D: {}", typed_location(location, entry_mode))?;
                writeln!(out, "  {}", id.attach(repo).shorten_or_id())?;
                writeln!(out, "  {entry_mode:o} ->")?;
            }
            gix::diff::tree_with_rewrites::Change::Modification {
                location,
                previous_id,
                id,
                previous_entry_mode,
                entry_mode,
            } => {
                writeln!(out, "M: {}", typed_location(location, entry_mode))?;
                writeln!(
                    out,
                    "  {previous_id} -> {id}",
                    previous_id = previous_id.attach(repo).shorten_or_id(),
                    id = id.attach(repo).shorten_or_id()
                )?;
                if previous_entry_mode != entry_mode {
                    writeln!(out, "  {previous_entry_mode:o} -> {entry_mode:o}")?;
                }
            }
            gix::diff::tree_with_rewrites::Change::Rewrite {
                source_location,
                source_id,
                id,
                location,
                source_entry_mode,
                entry_mode,
                ..
            } => {
                writeln!(
                    out,
                    "R: {source} -> {dest}",
                    source = typed_location(source_location, source_entry_mode),
                    dest = typed_location(location, entry_mode)
                )?;
                writeln!(
                    out,
                    "  {source_id} -> {id}",
                    source_id = source_id.attach(repo).shorten_or_id(),
                    id = id.attach(repo).shorten_or_id()
                )?;
                if source_entry_mode != entry_mode {
                    writeln!(out, "  {source_entry_mode:o} -> {entry_mode:o}")?;
                }
            }
        }
    }

    Ok(())
}

fn typed_location(mut location: BString, mode: EntryMode) -> BString {
    if mode.is_tree() {
        location.push(b'/');
    }
    location
}

fn resolve_revspec(
    repo: &gix::Repository,
    revspec: BString,
) -> Result<(ObjectId, Option<std::path::PathBuf>, BString), anyhow::Error> {
    let result = repo.rev_parse(revspec.as_bstr());

    match result {
        Err(err) => {
            // When the revspec is just a name, the delegate tries to resolve a reference which fails.
            // We extract the error from the tree to learn the name, and treat it as file.
            let not_found = err
                .iter_frames()
                .find_map(|f| f.downcast::<gix::refs::file::find::existing::Error>());
            if let Some(gix::refs::file::find::existing::Error::NotFound { name }) = not_found {
                let root = repo.workdir().map(ToOwned::to_owned);
                let name = gix::path::os_string_into_bstring(name.into())?;

                Ok((ObjectId::null(gix::hash::Kind::Sha1), root, name))
            } else {
                Err(err.into())
            }
        }
        Ok(resolved_revspec) => {
            let blob_id = resolved_revspec
                .single()
                .context(format!("rev-spec '{revspec}' must resolve to a single object"))?;

            let (path, _) = resolved_revspec
                .path_and_mode()
                .context(format!("rev-spec '{revspec}' must contain a path"))?;

            Ok((blob_id.into(), None, path.into()))
        }
    }
}

pub fn file(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    old_revspec: BString,
    new_revspec: BString,
) -> Result<(), anyhow::Error> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));
    repo.objects.refresh = RefreshMode::Never;

    let (old_blob_id, old_root, old_path) = resolve_revspec(&repo, old_revspec)?;
    let (new_blob_id, new_root, new_path) = resolve_revspec(&repo, new_revspec)?;

    let worktree_roots = gix::diff::blob::pipeline::WorktreeRoots { old_root, new_root };

    let mut resource_cache = repo.diff_resource_cache(
        gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        worktree_roots,
    )?;

    resource_cache.set_resource(
        old_blob_id,
        gix::object::tree::EntryKind::Blob,
        old_path.as_ref(),
        gix::diff::blob::ResourceKind::OldOrSource,
        &repo.objects,
    )?;
    resource_cache.set_resource(
        new_blob_id,
        gix::object::tree::EntryKind::Blob,
        new_path.as_ref(),
        gix::diff::blob::ResourceKind::NewOrDestination,
        &repo.objects,
    )?;

    let outcome = resource_cache.prepare_diff()?;

    use gix::diff::blob::platform::prepare_diff::Operation;

    let algorithm = match outcome.operation {
        Operation::InternalDiff { algorithm } => algorithm,
        Operation::ExternalCommand { .. } => {
            unreachable!("We disabled that")
        }
        Operation::SourceOrDestinationIsBinary => {
            anyhow::bail!("Source or destination is binary and we can't diff that")
        }
    };

    let interner = gix::diff::blob::intern::InternedInput::new(
        tokens_for_diffing(outcome.old.data.as_slice().unwrap_or_default()),
        tokens_for_diffing(outcome.new.data.as_slice().unwrap_or_default()),
    );

    let unified_diff = UnifiedDiff::new(
        &interner,
        ConsumeBinaryHunk::new(BString::default(), "\n"),
        ContextSize::symmetrical(3),
    );

    let unified_diff = gix::diff::blob::diff(algorithm, &interner, unified_diff)?;

    out.write_all(unified_diff.as_bytes())?;

    Ok(())
}

pub(crate) fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix::diff::blob::sources::byte_lines(data)
}
