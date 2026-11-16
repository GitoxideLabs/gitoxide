use std::collections::{BTreeMap, BTreeSet, HashMap};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{Result, ResultExt, message};
use gix_hash::ObjectId;
use gix_object::FindExt;

pub(super) use crate::tree::index::{Entry, Unmerged};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct EntryMerge {
    pub entry: Entry,
    pub resolution: MergeResolution,
    /// A selected tree policy resolves gitlink conflicts without manufacturing blob metadata.
    pub forced: bool,
    /// Original merge inputs in global base/current/other order, before destination collisions.
    pub stages: [Option<Entry>; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MergeResolution {
    Content(crate::blob::Resolution),
    Submodule,
    Ancestor,
}

impl EntryMerge {
    pub fn matches_entry(&self, entry: Entry) -> bool {
        self.entry.id == entry.id
            && match self.resolution {
                MergeResolution::Content(_) => entry.mode.is_blob() || entry.mode.is_link(),
                MergeResolution::Submodule => entry.mode.is_commit(),
                MergeResolution::Ancestor => entry.mode.is_no_tree(),
            }
    }

    pub fn content_merge(&self) -> Option<crate::tree::ContentMerge> {
        match self.resolution {
            MergeResolution::Content(resolution) => Some(crate::tree::ContentMerge {
                merged_blob_id: self.entry.id,
                resolution,
            }),
            MergeResolution::Submodule | MergeResolution::Ancestor => None,
        }
    }

    pub fn content_resolution(&self) -> Option<crate::blob::Resolution> {
        match self.resolution {
            MergeResolution::Content(resolution) => Some(resolution),
            MergeResolution::Ancestor => Some(crate::blob::Resolution::CompleteWithAutoResolvedConflict),
            MergeResolution::Submodule => None,
        }
    }

    pub fn resolution(&self) -> std::result::Result<crate::tree::Resolution, crate::tree::ResolutionFailure> {
        match self.resolution {
            MergeResolution::Content(_) => Ok(
                crate::tree::Resolution::OursModifiedTheirsModifiedThenBlobContentMerge {
                    merged_blob: self.content_merge().expect("content merges retain blob metadata"),
                },
            ),
            MergeResolution::Ancestor => Ok(crate::tree::Resolution::OursModifiedTheirsModifiedThenAncestor),
            MergeResolution::Submodule => Err(if self.stages[0].is_some_and(|entry| entry.mode.is_commit()) {
                crate::tree::ResolutionFailure::SubmoduleMerge
            } else {
                crate::tree::ResolutionFailure::SubmoduleAddAdd
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stage {
    pub entry: Entry,
    /// The original path survives moves into normalized destination records.
    pub source: BString,
    /// A source merge remains attached when its result moves into another path conflict.
    pub merge: Option<Box<EntryMerge>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum Resolution {
    #[default]
    Pending,
    Absent,
    Present(Entry),
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct Path {
    /// Original base, current, and other entries survive normalization and forced resolutions.
    pub inputs: [Option<Entry>; 3],
    pub stages: [Option<Stage>; 3],
    /// Directory ownership survives normalization replacing a directory stage with a file version.
    pub directory_mask: u8,
    pub result: Resolution,
    /// Final index stages belong to this path, independently of diagnostic history.
    pub unmerged: Option<Box<Unmerged>>,
    pub path_conflict: bool,
    opaque: bool,
}

impl Path {
    fn from_inputs(path: &BStr, inputs: [Option<Entry>; 3]) -> Self {
        let common = common_entry(inputs);
        Path {
            inputs,
            stages: inputs.map(|entry| {
                entry.map(|entry| Stage {
                    entry,
                    source: path.to_owned(),
                    merge: None,
                })
            }),
            directory_mask: inputs.iter().enumerate().fold(0, |mask, (side, entry)| {
                mask | if entry.is_some_and(|entry| entry.mode.is_tree()) {
                    1 << side
                } else {
                    0
                }
            }),
            result: common.map_or(Resolution::Pending, Resolution::Present),
            unmerged: None,
            opaque: common.is_some_and(|entry| entry.mode.is_tree()),
            path_conflict: false,
        }
    }

    pub fn trivial_choice(&self) -> Option<usize> {
        if self.path_conflict {
            return None;
        }
        let entries = self
            .stages
            .each_ref()
            .map(|stage| stage.as_ref().map(|stage| stage.entry));
        if entries[1] == entries[2] || entries[0] == entries[2] {
            Some(1)
        } else if entries[0] == entries[1] {
            Some(2)
        } else {
            None
        }
    }

    pub fn set_stage(&mut self, side: usize, stage: Option<Stage>) {
        debug_assert!(!self.opaque, "expand an opaque directory before normalizing its stages");
        if stage.as_ref().is_some_and(|stage| stage.entry.mode.is_tree()) {
            self.directory_mask |= 1 << side;
        }
        self.stages[side] = stage;
        self.invalidate();
    }

    pub fn take_stage(&mut self, side: usize) -> Option<Stage> {
        debug_assert!(!self.opaque, "expand an opaque directory before normalizing its stages");
        self.invalidate();
        self.stages[side].take()
    }

    pub fn invalidate(&mut self) {
        self.result = Resolution::Pending;
        self.unmerged = None;
    }

    pub fn set_result(&mut self, entry: Option<Entry>) {
        self.result = entry
            // Expanded directories follow their resolved children instead of reinstalling an input subtree.
            .filter(|entry| !entry.id.is_null() && (entry.mode.is_no_tree() || self.opaque))
            .map_or(Resolution::Absent, Resolution::Present);
        self.unmerged = None;
    }

    pub fn set_unmerged(&mut self, entries: [Option<Entry>; 3], content: Option<crate::blob::Resolution>) {
        let entries = entries.map(|entry| entry.filter(|entry| entry.mode.is_no_tree()));
        self.unmerged = entries
            .iter()
            .any(Option::is_some)
            .then(|| Box::new(Unmerged { entries, content }));
    }

    /// Restore one original input while expanded directories keep their independently resolved children.
    pub fn restore_input(&mut self, side: usize) {
        self.set_result(self.inputs[side].filter(|entry| entry.mode.is_no_tree() || self.opaque));
    }
}

#[derive(Debug, Default)]
pub(super) struct State {
    pub entries: BTreeMap<BString, Path>,
    /// Identical inputs need one identity until a lookup or normalization touches them.
    unchanged: BTreeMap<BString, Entry>,
    /// Chosen names remain occupied even before their resolved entry is installed.
    reserved: BTreeSet<BString>,
}

impl State {
    pub fn collect(trees: [&gix_hash::oid; 3], objects: &impl gix_object::Find) -> Result<Self> {
        let mut state = Self::default();
        let mut pending = vec![(BString::default(), trees.map(|tree_id| Some(tree_id.to_owned())))];
        while let Some((prefix, tree_ids)) = pending.pop() {
            for (path, inputs) in read_children(prefix.as_bstr(), tree_ids, objects)? {
                if let Some(entry) = common_entry(inputs).filter(|entry| entry.mode == entry.mode.kind().into()) {
                    state.unchanged.insert(path, entry);
                    continue;
                }
                let node = Path::from_inputs(path.as_bstr(), inputs);
                if !node.opaque && node.directory_mask != 0 {
                    pending.push((path.clone(), directory_ids(&node)));
                }
                state.entries.insert(path, node);
            }
        }
        Ok(state)
    }

    /// Original sibling modes, including identical entries needed as traversal-front placeholders.
    pub fn input_modes(&self) -> impl Iterator<Item = (&BStr, [Option<gix_object::tree::EntryMode>; 3])> {
        self.entries
            .iter()
            .map(|(path, node)| {
                (
                    path.as_bstr(),
                    node.inputs
                        .each_ref()
                        .map(|entry| entry.as_ref().map(|entry| entry.mode)),
                )
            })
            .chain(
                self.unchanged
                    .iter()
                    .map(|(path, entry)| (path.as_bstr(), [Some(entry.mode); 3])),
            )
    }

    /// Make a lookup and its ancestors explicit without expanding unrelated siblings.
    pub fn ensure_path(&mut self, path: &BStr, objects: &impl gix_object::Find) -> Result<()> {
        for (idx, &byte) in path.iter().enumerate() {
            if byte == b'/' {
                let parent = path[..idx].as_bstr();
                self.materialize(parent);
                self.expand(parent, objects, false)?;
            }
        }
        self.materialize(path);
        Ok(())
    }

    fn materialize(&mut self, path: &BStr) {
        if let Some((path, entry)) = self.unchanged.remove_entry(path) {
            let node = Path::from_inputs(path.as_bstr(), [Some(entry); 3]);
            self.entries.insert(path, node);
        }
    }

    /// Expose every original entry in a region that normalization or forced resolution will modify.
    pub fn ensure_subtree(&mut self, path: &BStr, objects: &impl gix_object::Find) -> Result<()> {
        self.ensure_path(path, objects)?;
        let prefix = child_prefix(path);
        let compact: Vec<_> = self
            .unchanged
            .range(prefix.clone()..)
            .take_while(|(candidate, _)| candidate.starts_with(prefix.as_bytes()))
            .map(|(path, _)| path.clone())
            .collect();
        for path in compact {
            self.materialize(path.as_bstr());
        }
        let mut pending: Vec<_> = self
            .entries
            .get(path)
            .filter(|node| node.opaque)
            .map(|_| path.to_owned())
            .into_iter()
            .collect();
        pending.extend(
            self.entries
                .range(prefix.clone()..)
                .take_while(|(candidate, _)| candidate.starts_with(prefix.as_bytes()))
                .filter(|(_, node)| node.opaque)
                .map(|(path, _)| path.clone()),
        );
        while let Some(path) = pending.pop() {
            pending.extend(self.expand(path.as_bstr(), objects, true)?);
        }
        Ok(())
    }

    fn expand(
        &mut self,
        path: &BStr,
        objects: &impl gix_object::Find,
        explicit_children: bool,
    ) -> Result<Vec<BString>> {
        let Some(node) = self.entries.get(path).filter(|node| node.opaque) else {
            return Ok(Vec::new());
        };
        let retain_children = match node.result {
            Resolution::Present(entry) => entry.mode.is_tree(),
            Resolution::Pending => true,
            Resolution::Absent => false,
        };
        let children = read_children(path, directory_ids(node), objects)?;
        let node = self
            .entries
            .get_mut(path)
            .expect("the opaque directory remains indexed");
        node.opaque = false;
        if matches!(node.result, Resolution::Present(entry) if entry.mode.is_tree()) {
            node.result = Resolution::Absent;
        }
        let mut opaque_children = Vec::new();
        for (path, inputs) in children {
            debug_assert!(
                !self.entries.contains_key(&path) && !self.unchanged.contains_key(&path),
                "opaque directories have no indexed descendants"
            );
            if !explicit_children
                && retain_children
                && let Some(entry) = common_entry(inputs).filter(|entry| entry.mode == entry.mode.kind().into())
            {
                self.unchanged.insert(path, entry);
                continue;
            }
            let mut node = Path::from_inputs(path.as_bstr(), inputs);
            if node.opaque {
                opaque_children.push(path.clone());
            }
            if !retain_children {
                node.result = Resolution::Absent;
            }
            self.entries.insert(path, node);
        }
        Ok(opaque_children)
    }

    /// Snapshot the normalized paths so every descendant is resolved before its parent.
    pub fn paths_bottom_up(&self) -> Vec<BString> {
        self.entries.keys().rev().cloned().collect()
    }

    pub fn has_live_descendants(&self, path: &BStr) -> bool {
        let prefix = child_prefix(path);
        self.entries
            .range(prefix.clone()..)
            .take_while(|(path, _)| path.starts_with(prefix.as_bytes()))
            .any(|(_, node)| matches!(node.result, Resolution::Present(_)))
            || self
                .unchanged
                .range(prefix.clone()..)
                .next()
                .is_some_and(|(path, _)| path.starts_with(prefix.as_bytes()))
    }

    /// Reserve against all input paths and earlier allocations, independently of resolution order.
    pub fn reserve_unique(&mut self, path: &BStr, label: &BStr, objects: &impl gix_object::Find) -> Result<BString> {
        let mut qualifier = BString::from("~");
        qualifier.extend(label.iter().map(|&byte| if byte == b'/' { b'_' } else { byte }));
        let mut component_end = path.len();
        loop {
            let at_root = !path[..component_end].contains(&b'/');
            let mut suffix = None::<usize>;
            loop {
                let mut candidate = path[..component_end].to_owned();
                candidate.extend_from_slice(&qualifier);
                if let Some(suffix) = suffix {
                    candidate.extend_from_slice(format!("_{suffix}").as_bytes());
                }
                candidate.extend_from_slice(&path[component_end..]);
                self.ensure_path(candidate.as_bstr(), objects)?;
                let blocked = self.has_blocking_parent(candidate.as_bstr());
                if blocked && !at_root {
                    break;
                }
                if !blocked && !self.contains_path(candidate.as_bstr()) {
                    self.reserved.insert(candidate.clone());
                    return Ok(candidate);
                }
                suffix = Some(suffix.map_or(0, |suffix| suffix + 1));
            }
            component_end = path[..component_end]
                .iter()
                .rposition(|byte| *byte == b'/')
                .expect("a non-root component has a preceding slash");
        }
    }

    fn contains_path(&self, path: &BStr) -> bool {
        let prefix = child_prefix(path);
        self.entries.contains_key(path)
            || self.unchanged.contains_key(path)
            || self.reserved.contains(path)
            || self
                .entries
                .range(prefix.clone()..)
                .next()
                .is_some_and(|(path, _)| path.starts_with(prefix.as_bytes()))
            || self
                .unchanged
                .range(prefix.clone()..)
                .next()
                .is_some_and(|(path, _)| path.starts_with(prefix.as_bytes()))
            || self
                .reserved
                .range(prefix.clone()..)
                .next()
                .is_some_and(|path| path.starts_with(prefix.as_bytes()))
    }

    fn has_blocking_parent(&self, path: &BStr) -> bool {
        path.iter().enumerate().any(|(idx, &byte)| {
            if byte != b'/' {
                return false;
            }
            let parent = path[..idx].as_bstr();
            self.reserved.contains(parent)
                || self.unchanged.get(parent).is_some_and(|entry| entry.mode.is_no_tree())
                || self.entries.get(parent).is_some_and(|node| match node.result {
                    Resolution::Present(entry) => !entry.mode.is_tree(),
                    Resolution::Absent => false,
                    Resolution::Pending => node.stages[1..]
                        .iter()
                        .flatten()
                        .any(|stage| !stage.entry.mode.is_tree()),
                })
        })
    }
}

fn directory_ids(node: &Path) -> [Option<ObjectId>; 3] {
    node.inputs
        .map(|entry| entry.filter(|entry| entry.mode.is_tree()).map(|entry| entry.id))
}

fn common_entry(inputs: [Option<Entry>; 3]) -> Option<Entry> {
    inputs[0].filter(|entry| inputs[1] == Some(*entry) && inputs[2] == Some(*entry))
}

fn read_children(
    prefix: &BStr,
    tree_ids: [Option<ObjectId>; 3],
    objects: &impl gix_object::Find,
) -> Result<Vec<(BString, [Option<Entry>; 3])>> {
    let mut children = HashMap::<&BStr, [Option<Entry>; 3]>::new();
    let mut buffers: [Vec<u8>; 3] = Default::default();
    for (side, (tree_id, buf)) in tree_ids.into_iter().zip(buffers.iter_mut()).enumerate() {
        let Some(tree_id) = tree_id else { continue };
        if tree_ids[..side].contains(&Some(tree_id)) || tree_id == ObjectId::empty_tree(tree_id.kind()) {
            continue;
        }
        let tree = objects
            .find_tree_iter(&tree_id, buf)
            .or_raise(|| message!("Could not collect merge input tree at {prefix:?}"))?;
        for entry in tree {
            let entry = entry?;
            let version = Entry {
                mode: entry.mode,
                id: entry.oid.to_owned(),
            };
            let inputs = children.entry(entry.filename).or_default();
            for (side, input_tree_id) in tree_ids.into_iter().enumerate() {
                if input_tree_id == Some(tree_id) {
                    inputs[side] = Some(version);
                }
            }
        }
    }
    let prefix = child_prefix(prefix);
    Ok(children
        .into_iter()
        .map(|(filename, inputs)| {
            let mut path = Vec::with_capacity(prefix.len() + filename.len());
            path.extend_from_slice(&prefix);
            path.extend_from_slice(filename);
            (path.into(), inputs)
        })
        .collect())
}

fn child_prefix(path: &BStr) -> BString {
    let mut prefix = path.to_owned();
    if !prefix.is_empty() {
        prefix.push(b'/');
    }
    prefix
}

#[cfg(test)]
mod tests {
    use bstr::ByteSlice;
    use gix_error::TestResult;
    use gix_object::{Write, tree::EntryKind};

    use super::{Entry, Path, Resolution, Stage, State};

    fn leaf() -> Entry {
        Entry {
            mode: EntryKind::Blob.into(),
            id: gix_testtools::object_hash().null(),
        }
    }

    #[test]
    fn empty_trees_need_not_exist_in_the_object_database() -> TestResult {
        for hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
            let empty_tree_id = gix_hash::ObjectId::empty_tree(hash);
            let state = State::collect([&empty_tree_id; 3], &gix_object::find::Never::panic_on_access())?;
            assert!(
                state.entries.is_empty(),
                "the known empty tree requires no object lookup"
            );
        }
        Ok(())
    }

    #[test]
    fn collection_preserves_inputs_and_original_paths_after_normalization() -> TestResult {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let blob_id = objects.write_buf(gix_object::Kind::Blob, b"payload")?;
        let child_tree_id = objects.write(&gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: EntryKind::Blob.into(),
                filename: b"child\xff".as_slice().into(),
                oid: blob_id,
            }],
        })?;
        let base_tree_id = objects.write(&gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: EntryKind::Blob.into(),
                filename: "d".into(),
                oid: blob_id,
            }],
        })?;
        let our_tree_id = objects.write(&gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: EntryKind::Tree.into(),
                filename: "d".into(),
                oid: child_tree_id,
            }],
        })?;
        let their_tree_id = objects.write(&gix_object::Tree::default())?;
        let mut state = State::collect([&base_tree_id, &our_tree_id, &their_tree_id], &objects)?;
        let directory = state
            .entries
            .get_mut("d".as_bytes())
            .expect("the path exists on two inputs");
        assert!(directory.inputs[0].expect("the base contains a file").mode.is_blob());
        assert!(directory.inputs[1].expect("ours contains a directory").mode.is_tree());
        assert_eq!(directory.inputs[2], None, "the other side deleted the path");
        let original = directory.inputs;
        let stage = directory.stages[1].take().expect("ours has a normalized stage");
        state.entries.entry("renamed".into()).or_default().stages[1] = Some(stage);
        assert_eq!(
            state.entries["d".as_bytes()].inputs,
            original,
            "normalization cannot erase input versions"
        );
        assert_eq!(
            state.entries["d".as_bytes()].directory_mask,
            0b010,
            "moving a file stage cannot erase the directory's original side"
        );
        assert_eq!(
            state.entries["renamed".as_bytes()].stages[1]
                .as_ref()
                .expect("the stage moved to its normalized destination")
                .source
                .as_bstr(),
            "d",
            "the renamed stage retains its input path"
        );
        assert!(
            state.entries.contains_key(b"d/child\xff".as_slice()),
            "Git paths remain byte-oriented"
        );
        Ok(())
    }

    #[test]
    fn reservations_consider_all_inputs_and_previous_allocations() -> TestResult {
        let mut state = State::default();
        let objects = gix_object::find::Never::panic_on_access();
        for (side, path) in ["d~A", "d~A_0", "d~A_1"].into_iter().enumerate() {
            let mut node = Path::default();
            node.inputs[side] = Some(leaf());
            state.entries.insert(path.into(), node);
        }
        assert_eq!(
            state.reserve_unique("d".into(), "A".into(), &objects)?,
            "d~A_2",
            "all three inputs reserve names"
        );
        assert_eq!(
            state.reserve_unique("d".into(), "A".into(), &objects)?,
            "d~A_3",
            "new names are reserved immediately"
        );
        assert_eq!(
            state.reserve_unique("d".into(), "feature/one".into(), &objects)?,
            "d~feature_one",
            "side labels cannot add path components"
        );
        Ok(())
    }

    #[test]
    fn live_children_and_blocking_parents_follow_results() -> TestResult {
        let mut state = State::default();
        let objects = gix_object::find::Never::panic_on_access();
        state.entries.entry("d".into()).or_default().stages[1] = Some(Stage {
            merge: None,
            entry: leaf(),
            source: "d".into(),
        });
        state.entries.entry("d/file".into()).or_default().result = Resolution::Present(leaf());
        state.entries.entry("d-other/file".into()).or_default().result = Resolution::Present(leaf());
        assert!(
            state.has_live_descendants("d".into()),
            "a resolved descendant keeps its directory alive"
        );
        assert_eq!(
            state.reserve_unique("d/new".into(), "A".into(), &objects)?,
            "d~A/new",
            "a blocking component itself must move"
        );
        state
            .entries
            .get_mut("d/file".as_bytes())
            .expect("the child exists")
            .result = Resolution::Absent;
        state.entries.get_mut("d".as_bytes()).expect("the parent exists").result = Resolution::Absent;
        assert!(
            !state.has_live_descendants("d".into()),
            "cancelled children and similarly named directories do not count"
        );
        assert_eq!(
            state.reserve_unique("d/new".into(), "A".into(), &objects)?,
            "d/new~A",
            "a resolved deletion no longer blocks descendants"
        );
        let paths = state.paths_bottom_up();
        assert!(
            paths
                .iter()
                .position(|path| path == "d/file")
                .expect("the child stays in the path set")
                < paths
                    .iter()
                    .position(|path| path == "d")
                    .expect("the parent stays in the path set"),
            "children are visited before their parent"
        );
        Ok(())
    }

    struct OpaqueFixture {
        objects: gix_odb::memory::Proxy<gix_object::find::Never>,
        root_tree_id: gix_hash::ObjectId,
        subtree_id: gix_hash::ObjectId,
    }

    fn opaque_fixture() -> TestResult<OpaqueFixture> {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let blob_id = objects.write_buf(gix_object::Kind::Blob, b"unchanged")?;
        let nested_tree_id = objects.write(&gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: EntryKind::Blob.into(),
                filename: "kept".into(),
                oid: blob_id,
            }],
        })?;
        let subtree_id = objects.write(&gix_object::Tree {
            entries: vec![
                gix_object::tree::Entry {
                    mode: EntryKind::Tree.into(),
                    filename: "nested".into(),
                    oid: nested_tree_id,
                },
                gix_object::tree::Entry {
                    mode: EntryKind::Blob.into(),
                    filename: "taken~A".into(),
                    oid: blob_id,
                },
            ],
        })?;
        let root_tree_id = objects.write(&gix_object::Tree {
            entries: vec![
                gix_object::tree::Entry {
                    mode: EntryKind::Tree.into(),
                    filename: "dir-other".into(),
                    oid: nested_tree_id,
                },
                gix_object::tree::Entry {
                    mode: EntryKind::Tree.into(),
                    filename: "dir".into(),
                    oid: subtree_id,
                },
            ],
        })?;
        Ok(OpaqueFixture {
            objects,
            root_tree_id,
            subtree_id,
        })
    }

    #[test]
    fn identical_subtrees_remain_opaque_and_emit_the_original_tree() -> TestResult {
        let fixture = opaque_fixture()?;
        let mut state = State::collect([&fixture.root_tree_id; 3], &fixture.objects)?;
        assert!(state.entries.is_empty(), "identical inputs need no mutable stages");
        assert_eq!(
            state.unchanged.len(),
            2,
            "identical descendants are represented by their two subtree roots"
        );
        state.ensure_path("dir".into(), &fixture.objects)?;
        let directory = state.entries.get_mut("dir".as_bytes()).expect("the subtree is indexed");
        assert!(directory.opaque, "the unchanged subtree has not been expanded");
        directory.result = Resolution::Absent;
        directory.restore_input(0);
        assert_eq!(
            directory.result,
            Resolution::Present(Entry {
                mode: EntryKind::Tree.into(),
                id: fixture.subtree_id
            })
        );
        let mut editor = gix_object::tree::Editor::new(
            gix_object::Tree::default(),
            &fixture.objects,
            gix_testtools::object_hash(),
        );
        for (path, node) in state.entries {
            if let Resolution::Present(entry) = node.result {
                editor.upsert(path.split(|&byte| byte == b'/'), entry.mode.kind(), entry.id)?;
            }
        }
        for (path, entry) in state.unchanged {
            editor.upsert(path.split(|&byte| byte == b'/'), entry.mode.kind(), entry.id)?;
        }
        assert_eq!(
            editor.write(|tree| fixture.objects.write(tree))?,
            fixture.root_tree_id,
            "opaque subtrees preserve the complete output without visiting their leaves"
        );
        Ok(())
    }

    #[test]
    fn namespace_expansion_sees_hidden_entries_without_resurrecting_discarded_subtrees() -> TestResult {
        let fixture = opaque_fixture()?;
        for retained in [true, false] {
            let mut state = State::collect([&fixture.root_tree_id; 3], &fixture.objects)?;
            state.ensure_path("dir".into(), &fixture.objects)?;
            if !retained {
                state
                    .entries
                    .get_mut("dir".as_bytes())
                    .expect("the subtree is indexed")
                    .result = Resolution::Absent;
            }
            assert_eq!(
                state.reserve_unique("dir/taken".into(), "A".into(), &fixture.objects)?,
                "dir/taken~A_0",
                "a name hidden in an opaque subtree remains occupied"
            );
            assert_eq!(
                state.entries["dir".as_bytes()].result,
                Resolution::Absent,
                "expanded directories no longer emit their stale tree"
            );
            assert_eq!(
                state.has_live_descendants("dir".into()),
                retained,
                "namespace lookup must retain the directory's resolved fate"
            );
            assert!(
                state.unchanged.contains_key("dir-other".as_bytes()),
                "looking up one subtree leaves its sibling compact"
            );
            assert_eq!(
                state.entries["dir/taken~A".as_bytes()].stages[1]
                    .as_ref()
                    .expect("the input leaf is materialized")
                    .source
                    .as_bstr(),
                "dir/taken~A",
                "expansion preserves original input paths"
            );
        }
        Ok(())
    }

    #[test]
    fn expanding_for_normalization_preserves_inputs_and_invalidates_changed_stages() -> TestResult {
        let fixture = opaque_fixture()?;
        let mut state = State::collect([&fixture.root_tree_id; 3], &fixture.objects)?;
        state.ensure_path("dir/taken~A".into(), &fixture.objects)?;
        state.ensure_subtree("dir".into(), &fixture.objects)?;
        assert!(
            !state.entries["dir/nested".as_bytes()].opaque,
            "the touched region is fully expanded"
        );
        assert!(
            state.unchanged.contains_key("dir-other".as_bytes()),
            "unrelated regions remain compact"
        );
        let directory = state.entries.get_mut("dir".as_bytes()).expect("the subtree is indexed");
        directory.restore_input(0);
        assert_eq!(
            directory.result,
            Resolution::Absent,
            "restoring an expanded directory does not emit its stale input tree"
        );
        let leaf = state
            .entries
            .get_mut("dir/nested/kept".as_bytes())
            .expect("all original leaves are available");
        let inputs = leaf.inputs;
        assert!(
            matches!(leaf.result, Resolution::Present(_)),
            "expansion supplies an already resolved original leaf"
        );
        leaf.set_unmerged(inputs, None);
        let stage = leaf.take_stage(1).expect("the current input is present");
        assert_eq!(
            leaf.result,
            Resolution::Pending,
            "removing a normalized stage invalidates the old result"
        );
        assert!(
            leaf.unmerged.is_none(),
            "normalization invalidates the previous index owner"
        );
        assert_eq!(leaf.inputs, inputs, "normalization keeps immutable input identities");
        leaf.set_unmerged(inputs, Some(crate::blob::Resolution::Conflict));
        leaf.restore_input(1);
        assert_eq!(
            leaf.result,
            Resolution::Present(stage.entry),
            "restoration uses immutable input identity after the normalized stage moved"
        );
        assert!(
            leaf.unmerged.is_none(),
            "restoration discards cancelled content conflicts"
        );
        let moved = state.entries.entry("moved".into()).or_default();
        moved.set_unmerged(inputs, None);
        moved.set_stage(1, Some(stage));
        assert!(moved.unmerged.is_none(), "a moved stage replaces the old index owner");
        assert_eq!(
            state.entries["moved".as_bytes()].stages[1]
                .as_ref()
                .expect("the stage was moved")
                .source
                .as_bstr(),
            "dir/nested/kept",
            "a moved expanded stage retains its original source"
        );
        let mut node = Path::default();
        let entry = inputs[0].expect("the base leaf exists");
        for (side, source) in ["base", "current", "other"].into_iter().enumerate() {
            node.set_stage(
                side,
                Some(Stage {
                    entry,
                    source: source.into(),
                    merge: None,
                }),
            );
        }
        assert_eq!(
            node.trivial_choice(),
            Some(1),
            "different original paths do not change identical entry contents"
        );
        node.path_conflict = true;
        assert_eq!(
            node.trivial_choice(),
            None,
            "equal contents cannot erase a structural path conflict"
        );
        Ok(())
    }
}
