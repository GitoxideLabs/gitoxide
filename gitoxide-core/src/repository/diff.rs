use gix::bstr::{BStr, BString, ByteSlice};
use gix::diff::blob::intern::TokenSource;
use gix::diff::blob::UnifiedDiffBuilder;
use gix::objs::tree::EntryMode;
use gix::odb::store::RefreshMode;
use gix::prelude::ObjectIdExt;
use gix::ObjectId;

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
                writeln!(out, "  -> {:o}", entry_mode.0)?;
            }
            gix::diff::tree_with_rewrites::Change::Deletion {
                location,
                id,
                entry_mode,
                ..
            } => {
                writeln!(out, "D: {}", typed_location(location, entry_mode))?;
                writeln!(out, "  {}", id.attach(repo).shorten_or_id())?;
                writeln!(out, "  {:o} ->", entry_mode.0)?;
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
                    writeln!(out, "  {:o} -> {:o}", previous_entry_mode.0, entry_mode.0)?;
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
                    writeln!(out, "  {:o} -> {:o}", source_entry_mode.0, entry_mode.0)?;
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

pub fn file(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    old_treeish: BString,
    new_treeish: BString,
    path: BString,
) -> Result<(), anyhow::Error> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));
    repo.objects.refresh = RefreshMode::Never;

    let old_tree_id = repo.rev_parse_single(old_treeish.as_bstr())?;
    let new_tree_id = repo.rev_parse_single(new_treeish.as_bstr())?;

    let old_tree = old_tree_id.object()?.peel_to_tree()?;
    let new_tree = new_tree_id.object()?.peel_to_tree()?;

    let mut old_tree_buf = Vec::new();
    let mut new_tree_buf = Vec::new();

    use gix::diff::object::FindExt;

    let old_tree_iter = repo.objects.find_tree_iter(&old_tree.id(), &mut old_tree_buf)?;
    let new_tree_iter = repo.objects.find_tree_iter(&new_tree.id(), &mut new_tree_buf)?;

    use gix::diff::tree::{
        recorder::{self, Location},
        Recorder,
    };

    struct FindChangeToPath {
        inner: Recorder,
        interesting_path: BString,
        change: Option<recorder::Change>,
    }

    impl FindChangeToPath {
        fn new(interesting_path: &BStr) -> Self {
            let inner = Recorder::default().track_location(Some(Location::Path));

            FindChangeToPath {
                inner,
                interesting_path: interesting_path.into(),
                change: None,
            }
        }
    }

    use gix::diff::tree::{visit, Visit};

    impl Visit for FindChangeToPath {
        fn pop_front_tracked_path_and_set_current(&mut self) {
            self.inner.pop_front_tracked_path_and_set_current();
        }

        fn push_back_tracked_path_component(&mut self, component: &BStr) {
            self.inner.push_back_tracked_path_component(component);
        }

        fn push_path_component(&mut self, component: &BStr) {
            self.inner.push_path_component(component);
        }

        fn pop_path_component(&mut self) {
            self.inner.pop_path_component();
        }

        fn visit(&mut self, change: visit::Change) -> visit::Action {
            if self.inner.path() == self.interesting_path {
                self.change = Some(match change {
                    visit::Change::Deletion {
                        entry_mode,
                        oid,
                        relation,
                    } => recorder::Change::Deletion {
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                        relation,
                    },
                    visit::Change::Addition {
                        entry_mode,
                        oid,
                        relation,
                    } => recorder::Change::Addition {
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                        relation,
                    },
                    visit::Change::Modification {
                        previous_entry_mode,
                        previous_oid,
                        entry_mode,
                        oid,
                    } => recorder::Change::Modification {
                        previous_entry_mode,
                        previous_oid,
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                    },
                });

                visit::Action::Cancel
            } else {
                visit::Action::Continue
            }
        }
    }

    let mut recorder = FindChangeToPath::new(path.as_ref());
    let state = gix::diff::tree::State::default();
    let result = gix::diff::tree(old_tree_iter, new_tree_iter, state, &repo.objects, &mut recorder);

    let change = match result {
        Ok(_) | Err(gix::diff::tree::Error::Cancelled) => recorder.change,
        Err(error) => return Err(error.into()),
    };

    let Some(change) = change else {
        anyhow::bail!(
            "There was no change to {} between {} and {}",
            &path,
            old_treeish,
            new_treeish
        )
    };

    let mut resource_cache = repo.diff_resource_cache(gix::diff::blob::pipeline::Mode::ToGit, Default::default())?;

    let (previous_oid, oid) = match change {
        recorder::Change::Addition { oid, .. } => {
            // Setting `previous_oid` to `ObjectId::empty_blob` makes `diff` see an addition.
            (ObjectId::empty_blob(gix::hash::Kind::Sha1), oid)
        }
        recorder::Change::Deletion { oid: previous_oid, .. } => {
            // Setting `oid` to `ObjectId::empty_blob` makes `diff` see a deletion.
            (previous_oid, ObjectId::empty_blob(gix::hash::Kind::Sha1))
        }
        recorder::Change::Modification { previous_oid, oid, .. } => (previous_oid, oid),
    };

    resource_cache.set_resource(
        previous_oid,
        gix::object::tree::EntryKind::Blob,
        path.as_slice().into(),
        gix::diff::blob::ResourceKind::OldOrSource,
        &repo.objects,
    )?;
    resource_cache.set_resource(
        oid,
        gix::object::tree::EntryKind::Blob,
        path.as_slice().into(),
        gix::diff::blob::ResourceKind::NewOrDestination,
        &repo.objects,
    )?;

    let outcome = resource_cache.prepare_diff()?;

    let old_data = String::from_utf8_lossy(outcome.old.data.as_slice().unwrap_or_default());
    let new_data = String::from_utf8_lossy(outcome.new.data.as_slice().unwrap_or_default());

    let input =
        gix::diff::blob::intern::InternedInput::new(tokens_for_diffing(&old_data), tokens_for_diffing(&new_data));

    let unified_diff_builder = UnifiedDiffBuilder::new(&input);

    use gix::diff::blob::platform::prepare_diff::Operation;

    let algorithm = match outcome.operation {
        Operation::InternalDiff { algorithm } => algorithm,
        Operation::ExternalCommand { .. } => {
            // `unreachable!` is also used in [`Platform::lines()`](gix::object::blob::diff::Platform::lines()).
            unreachable!("We disabled that")
        }
        Operation::SourceOrDestinationIsBinary => {
            anyhow::bail!("Source or destination is binary and we can't diff that")
        }
    };

    let unified_diff = gix::diff::blob::diff(algorithm, &input, unified_diff_builder);

    out.write_all(unified_diff.as_bytes())?;

    Ok(())
}

pub(crate) fn tokens_for_diffing(data: &str) -> impl TokenSource<Token = &str> {
    gix::diff::blob::sources::lines(data)
}
