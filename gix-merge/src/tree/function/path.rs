use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Range,
};

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
pub(super) struct Stage<'input> {
    pub entry: Entry,
    /// The original path survives moves into normalized destination records.
    pub source: Cow<'input, BStr>,
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
pub(super) struct Path<'input> {
    /// Original base, current, and other entries survive normalization and forced resolutions.
    pub inputs: [Option<Entry>; 3],
    pub stages: [Option<Stage<'input>>; 3],
    /// Directory ownership survives normalization replacing a directory stage with a file version.
    pub directory_mask: u8,
    pub result: Resolution,
    /// Final index stages belong to this path, independently of diagnostic history.
    pub unmerged: Option<Box<Unmerged>>,
    pub path_conflict: bool,
    opaque: bool,
}

impl<'input> Path<'input> {
    fn from_inputs(path: Cow<'input, BStr>, inputs: [Option<Entry>; 3]) -> Self {
        let common = common_entry(inputs);
        let mut remaining = inputs.iter().filter(|entry| entry.is_some()).count();
        let mut source = Some(path);
        Path {
            inputs,
            stages: inputs.map(|entry| {
                entry.map(|entry| Stage {
                    entry,
                    source: {
                        remaining -= 1;
                        if remaining == 0 {
                            source.take().expect("the last input takes the original name")
                        } else {
                            source
                                .as_ref()
                                .expect("the original name is retained for later inputs")
                                .clone()
                        }
                    },
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
        trivial_choice(entries)
    }

    pub fn set_stage(&mut self, side: usize, stage: Option<Stage<'input>>) {
        debug_assert!(!self.opaque, "expand an opaque directory before normalizing its stages");
        if stage.as_ref().is_some_and(|stage| stage.entry.mode.is_tree()) {
            self.directory_mask |= 1 << side;
        }
        self.stages[side] = stage;
        self.invalidate();
    }

    pub fn take_stage(&mut self, side: usize) -> Option<Stage<'input>> {
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
pub(super) struct State<'input> {
    pub entries: BTreeMap<Cow<'input, BStr>, Box<Path<'input>>>,
    /// Clean leaf choices retain only their original inputs until another operation touches them.
    pub(super) trivial: BTreeMap<Cow<'input, BStr>, Trivial>,
    /// Identical inputs need one identity until a lookup or normalization touches them.
    unchanged: BTreeMap<Cow<'input, BStr>, Entry>,
    /// Chosen names remain occupied even before their resolved entry is installed.
    reserved: BTreeSet<BString>,
    /// Surviving descendants after normalization, maintained as paths are resolved or restored.
    live: Option<HashMap<BString, usize>>,
    buffers: [Vec<u8>; 3],
}

#[derive(Debug)]
pub(super) struct Trivial {
    pub(super) inputs: [Option<Entry>; 3],
    choice: usize,
}

impl Trivial {
    pub(super) fn entry(&self) -> Option<Entry> {
        self.inputs[self.choice].filter(|entry| !entry.id.is_null())
    }
}

#[derive(Default)]
pub(super) struct Collected {
    pub names: Vec<u8>,
    pub nested_names: bool,
    pub entries: Vec<(Range<usize>, [Option<Entry>; 3])>,
    pub unchanged: Vec<(Range<usize>, Entry)>,
}

impl Collected {
    pub fn collect(trees: [&gix_hash::oid; 3], objects: &impl gix_object::Find, ordered: bool) -> Result<Self> {
        let mut collected = Self::default();
        let mut buffers: [Vec<u8>; 3] = Default::default();
        let mut pending: Vec<_> = read_children(
            "".into(),
            trees.map(|tree_id| Some(tree_id.to_owned())),
            objects,
            &mut collected.names,
            &mut buffers,
            ordered,
            &mut collected.nested_names,
        )?
        .into_iter()
        .rev()
        .collect();
        while let Some((name, inputs)) = pending.pop() {
            let common = common_entry(inputs);
            if let Some(entry) = common.filter(|entry| entry.mode == entry.mode.kind().into()) {
                collected.unchanged.push((name, entry));
                continue;
            }
            let tree_ids = directory_ids(inputs);
            if common.is_none() && tree_ids.iter().any(Option::is_some) {
                let prefix = collected.names[name.clone()].to_owned();
                pending.extend(
                    read_children(
                        prefix.as_bstr(),
                        tree_ids,
                        objects,
                        &mut collected.names,
                        &mut buffers,
                        ordered,
                        &mut collected.nested_names,
                    )?
                    .into_iter()
                    .rev(),
                );
            }
            collected.entries.push((name, inputs));
        }
        collected.names.shrink_to_fit();
        Ok(collected)
    }
}

impl<'input> State<'input> {
    pub fn new(
        names: &'input [u8],
        entries: Vec<(Range<usize>, [Option<Entry>; 3])>,
        unchanged: Vec<(Range<usize>, Entry)>,
        resolve_trivial: bool,
    ) -> Self {
        let mut state = Self::default();
        let mut trivial = Vec::new();
        // Keys and original stages share names in the immutable input pool.
        for (name, inputs) in entries {
            let path = Cow::Borrowed(names[name].as_bstr());
            if resolve_trivial
                && inputs.iter().flatten().all(|entry| entry.mode.is_no_tree())
                && let Some(choice) = trivial_choice(inputs)
            {
                trivial.push((path, Trivial { inputs, choice }));
                continue;
            }
            let node = Path::from_inputs(path.clone(), inputs);
            state.entries.insert(path, Box::new(node));
        }
        // Bulk construction avoids a tree lookup for each clean input path.
        state.trivial = trivial.into_iter().collect();
        state.unchanged = unchanged
            .into_iter()
            .map(|(name, entry)| (Cow::Borrowed(names[name].as_bstr()), entry))
            .collect();
        state
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

    pub(super) fn materialize(&mut self, path: &BStr) {
        if let Some((path, trivial)) = self.trivial.remove_entry(path) {
            let mut node = Path::from_inputs(path.clone(), trivial.inputs);
            node.set_result(trivial.entry());
            self.entries.insert(path, Box::new(node));
        }
        if let Some((path, entry)) = self.unchanged.remove_entry(path) {
            let node = Path::from_inputs(path.clone(), [Some(entry); 3]);
            self.entries.insert(path, Box::new(node));
        }
    }

    /// Expose every original entry in a region that normalization or forced resolution will modify.
    pub fn ensure_subtree(&mut self, path: &BStr, objects: &impl gix_object::Find) -> Result<()> {
        self.ensure_path(path, objects)?;
        if !path.is_empty() && self.entries.get(path).is_none_or(|node| node.directory_mask == 0) {
            debug_assert!(
                !self.entries.keys().any(|candidate| candidate
                    .strip_prefix(path.as_bytes())
                    .is_some_and(|suffix| suffix.starts_with(b"/"))),
                "original directory ownership covers every subtree that can be expanded"
            );
            return Ok(());
        }
        let prefix = child_prefix(path);
        let mut compact: Vec<_> = self
            .unchanged
            .range(Cow::Owned(prefix.clone())..)
            .take_while(|(candidate, _)| candidate.starts_with(prefix.as_bytes()))
            .map(|(path, _)| path.clone())
            .collect();
        compact.extend(
            self.trivial
                .range(Cow::Owned(prefix.clone())..)
                .take_while(|(candidate, _)| candidate.starts_with(prefix.as_bytes()))
                .map(|(path, _)| path.clone()),
        );
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
                .range(Cow::Owned(prefix.clone())..)
                .take_while(|(candidate, _)| candidate.starts_with(prefix.as_bytes()))
                .filter(|(_, node)| node.opaque)
                .map(|(path, _)| path.clone().into_owned()),
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
        let mut names = Vec::new();
        let children = read_children(
            path,
            directory_ids(node.inputs),
            objects,
            &mut names,
            &mut self.buffers,
            false,
            &mut false,
        )?;
        let node = self
            .entries
            .get_mut(path)
            .expect("the opaque directory remains indexed");
        node.opaque = false;
        if matches!(node.result, Resolution::Present(entry) if entry.mode.is_tree()) {
            self.set_result(path, None);
        }
        let mut opaque_children = Vec::new();
        for (name, inputs) in children {
            let path = names[name].as_bstr().to_owned();
            debug_assert!(
                !self.entries.contains_key(path.as_bstr())
                    && !self.unchanged.contains_key(path.as_bstr())
                    && !self.trivial.contains_key(path.as_bstr()),
                "opaque directories have no indexed descendants"
            );
            if !explicit_children
                && retain_children
                && let Some(entry) = common_entry(inputs).filter(|entry| entry.mode == entry.mode.kind().into())
            {
                if let Some(live) = &mut self.live {
                    update_live(live, path.as_bstr(), true, 1);
                }
                self.unchanged.insert(Cow::Owned(path), entry);
                continue;
            }
            let mut node = Path::from_inputs(Cow::Owned(path.clone()), inputs);
            if node.opaque {
                opaque_children.push(path.clone());
            }
            if !retain_children {
                node.result = Resolution::Absent;
            }
            if matches!(node.result, Resolution::Present(_))
                && let Some(live) = &mut self.live
            {
                update_live(live, path.as_bstr(), true, 1);
            }
            self.entries.insert(Cow::Owned(path), Box::new(node));
        }
        Ok(opaque_children)
    }

    /// Snapshot the normalized paths so every descendant is resolved before its parent.
    pub fn paths_bottom_up(&self) -> Vec<Cow<'input, BStr>> {
        self.entries.keys().rev().cloned().collect()
    }

    pub fn prepare_resolution(&mut self) {
        let mut live = HashMap::new();
        for (path, node) in &self.entries {
            if matches!(node.result, Resolution::Present(_)) {
                update_live(&mut live, path.as_bstr(), true, 1);
            }
        }
        // Count identical siblings together instead of hashing every ancestor for each leaf.
        let mut unchanged = self
            .unchanged
            .keys()
            .chain(
                self.trivial
                    .iter()
                    .filter_map(|(path, node)| node.entry().map(|_| path)),
            )
            .peekable();
        while let Some(path) = unchanged.next() {
            let end = path.rfind_byte(b'/').unwrap_or(0);
            let mut count = 1;
            while unchanged
                .next_if(|candidate| candidate.rfind_byte(b'/').unwrap_or(0) == end && candidate[..end] == path[..end])
                .is_some()
            {
                count += 1;
            }
            update_live(&mut live, path.as_bstr(), true, count);
        }
        self.live = Some(live);
    }

    pub fn set_result(&mut self, path: &BStr, entry: Option<Entry>) {
        self.set_resolution(
            path,
            entry
                .filter(|entry| !entry.id.is_null())
                .map_or(Resolution::Absent, Resolution::Present),
        );
    }

    pub fn set_resolution(&mut self, path: &BStr, result: Resolution) {
        if !self.trivial.is_empty() && self.trivial.contains_key(path) {
            self.materialize(path);
        }
        let node = if let Some(node) = self.entries.get_mut(path) {
            node
        } else {
            self.entries.entry(Cow::Owned(path.to_owned())).or_default()
        };
        let was_live = matches!(node.result, Resolution::Present(_));
        let is_live = matches!(result, Resolution::Present(_));
        node.result = result;
        node.unmerged = None;
        if was_live != is_live
            && let Some(live) = &mut self.live
        {
            update_live(live, path, is_live, 1);
        }
    }

    pub fn restore_input(&mut self, path: &BStr, side: usize) {
        let entry = self
            .entries
            .get(path)
            .map(|node| node.inputs[side].filter(|entry| entry.mode.is_no_tree() || node.opaque))
            .or_else(|| self.trivial.get(path).map(|node| node.inputs[side]));
        if let Some(entry) = entry {
            self.set_result(path, entry);
        }
    }

    pub fn has_live_descendants(&self, path: &BStr) -> bool {
        if let Some(live) = &self.live {
            return live.get(path).is_some_and(|&count| count != 0);
        }
        let prefix = child_prefix(path);
        self.entries
            .range(Cow::Owned(prefix.clone())..)
            .take_while(|(path, _)| path.starts_with(prefix.as_bytes()))
            .any(|(_, node)| matches!(node.result, Resolution::Present(_)))
            || self
                .trivial
                .range(Cow::Owned(prefix.clone())..)
                .take_while(|(path, _)| path.starts_with(prefix.as_bytes()))
                .any(|(_, node)| node.entry().is_some())
            || self
                .unchanged
                .range(Cow::Owned(prefix.clone())..)
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
            || self.trivial.contains_key(path)
            || self.reserved.contains(path)
            || self
                .entries
                .range(Cow::Owned(prefix.clone())..)
                .next()
                .is_some_and(|(path, _)| path.starts_with(prefix.as_bytes()))
            || self
                .unchanged
                .range(Cow::Owned(prefix.clone())..)
                .next()
                .is_some_and(|(path, _)| path.starts_with(prefix.as_bytes()))
            || self
                .trivial
                .range(Cow::Owned(prefix.clone())..)
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
                || self.trivial.get(parent).is_some_and(|node| node.entry().is_some())
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

fn update_live(live: &mut HashMap<BString, usize>, path: &BStr, present: bool, amount: usize) {
    for end in std::iter::once(0).chain(
        path.iter()
            .enumerate()
            .filter_map(|(index, &byte)| (byte == b'/').then_some(index)),
    ) {
        let parent = path[..end].as_bstr();
        if let Some(count) = live.get_mut(parent) {
            *count = if present {
                *count + amount
            } else {
                count
                    .checked_sub(amount)
                    .expect("removing a live path balances its ancestor counts")
            };
        } else {
            debug_assert!(present, "a removed live path has counted ancestors");
            live.insert(parent.to_owned(), amount);
        }
    }
}

fn directory_ids(inputs: [Option<Entry>; 3]) -> [Option<ObjectId>; 3] {
    inputs.map(|entry| entry.filter(|entry| entry.mode.is_tree()).map(|entry| entry.id))
}

fn trivial_choice(entries: [Option<Entry>; 3]) -> Option<usize> {
    if entries[1] == entries[2] || entries[0] == entries[2] {
        Some(1)
    } else if entries[0] == entries[1] {
        Some(2)
    } else {
        None
    }
}

fn common_entry(inputs: [Option<Entry>; 3]) -> Option<Entry> {
    inputs[0].filter(|entry| inputs[1] == Some(*entry) && inputs[2] == Some(*entry))
}

#[derive(Default)]
struct Siblings<'a> {
    remaining: gix_object::TreeRefIter<'a>,
    front: Option<gix_object::tree::EntryRef<'a>>,
    origin: usize,
    consumed: Vec<usize>,
    malformed: bool,
}

impl<'a> Siblings<'a> {
    fn check_order(&mut self, previous: Option<gix_object::tree::EntryRef<'a>>, next: gix_object::tree::EntryRef<'a>) {
        self.malformed |= previous.is_some_and(|previous| {
            previous >= next || (previous.mode.is_tree() != next.mode.is_tree() && previous.filename == next.filename)
        });
    }

    fn advance(&mut self) -> Result<()> {
        let mut previous = self.front;
        loop {
            self.front = self.remaining.next().transpose()?;
            let Some(next) = self.front else { return Ok(()) };
            self.check_order(previous, next);
            if self.origin == 0 {
                self.origin = next.filename.as_ptr() as usize;
            }
            let offset = next.filename.as_ptr() as usize - self.origin;
            let Some(index) = self.consumed.iter().position(|consumed| *consumed == offset) else {
                return Ok(());
            };
            self.consumed.swap_remove(index);
            previous = self.front;
        }
    }

    fn take(&mut self, name: &BStr) -> Result<Option<gix_object::tree::EntryRef<'a>>> {
        let Some(front) = self.front else { return Ok(None) };
        // A directory named `a` can hide behind files such as `a-` and `a.`.
        let can_hide = |entry: gix_object::tree::EntryRef<'_>| {
            entry.filename.starts_with(name) && entry.filename.get(name.len()).is_some_and(|byte| *byte < b'/')
        };
        if front.filename == name {
            self.advance()?;
            // A second entry with the same name can only hide in this punctuation gap.
            if front.mode.is_no_tree() && self.front.is_some_and(can_hide) {
                for entry in self.remaining {
                    let entry = entry?;
                    if entry.filename == name {
                        self.malformed = true;
                        break;
                    }
                    if !can_hide(entry) {
                        break;
                    }
                }
            }
            return Ok(Some(front));
        }
        if !can_hide(front) {
            return Ok(None);
        }
        let mut probe = self.remaining;
        let mut previous = Some(front);
        for entry in &mut probe {
            let entry = entry?;
            self.check_order(previous, entry);
            previous = Some(entry);
            let offset = entry.filename.as_ptr() as usize - self.origin;
            if self.consumed.contains(&offset) {
                continue;
            }
            if entry.filename == name {
                self.consumed.push(offset);
                return Ok(Some(entry));
            }
            if !can_hide(entry) {
                break;
            }
        }
        Ok(None)
    }
}

#[expect(clippy::type_complexity)]
fn read_children_stream(
    prefix: &BStr,
    tree_ids: [Option<ObjectId>; 3],
    iterators: [gix_object::TreeRefIter<'_>; 3],
    names: &mut Vec<u8>,
) -> Result<Option<Vec<(Range<usize>, [Option<Entry>; 3])>>> {
    let mut siblings: [Siblings<'_>; 3] = std::array::from_fn(|_| Siblings::default());
    for (cursor, remaining) in siblings.iter_mut().zip(iterators) {
        cursor.remaining = remaining;
        cursor.advance()?;
    }
    let sources: [usize; 3] = std::array::from_fn(|side| {
        tree_ids[..side]
            .iter()
            .position(|tree_id| *tree_id == tree_ids[side])
            .unwrap_or(side)
    });
    let prefix = child_prefix(prefix);
    let mut output: Vec<(Range<usize>, [Option<Entry>; 3])> = Vec::new();
    while let Some(first) = siblings
        .iter()
        .filter_map(|cursor| cursor.front)
        .min_by_key(|entry| entry.filename)
    {
        if siblings.iter().any(|cursor| cursor.malformed) {
            return Ok(None);
        }
        let filename = first.filename;
        if filename.is_empty() || filename.contains(&b'/') {
            return Ok(None);
        }
        let mut inputs = [None; 3];
        for (side, cursor) in siblings.iter_mut().enumerate() {
            if let Some(entry) = cursor.take(filename)? {
                inputs[side] = Some(Entry {
                    mode: entry.mode,
                    id: entry.oid.to_owned(),
                });
            }
        }
        let start = names.len();
        names.extend_from_slice(&prefix);
        names.extend_from_slice(filename);
        output.push((start..names.len(), sources.map(|source| inputs[source])));
    }
    Ok((!siblings.iter().any(|cursor| cursor.malformed)).then_some(output))
}

#[expect(clippy::type_complexity)]
fn read_children(
    prefix: &BStr,
    tree_ids: [Option<ObjectId>; 3],
    objects: &impl gix_object::Find,
    names: &mut Vec<u8>,
    buffers: &mut [Vec<u8>; 3],
    ordered: bool,
    nested_names: &mut bool,
) -> Result<Vec<(Range<usize>, [Option<Entry>; 3])>> {
    let mut iterators = [gix_object::TreeRefIter::default(); 3];
    for (side, (tree_id, buf)) in tree_ids.into_iter().zip(buffers.iter_mut()).enumerate() {
        let Some(tree_id) = tree_id else { continue };
        if tree_ids[..side].contains(&Some(tree_id)) || tree_id == ObjectId::empty_tree(tree_id.kind()) {
            continue;
        }
        iterators[side] = objects
            .find_tree_iter(&tree_id, buf)
            .or_raise(|| message!("Could not collect merge input tree at {prefix:?}"))?;
    }
    let start = names.len();
    if let Some(children) = read_children_stream(prefix, tree_ids, iterators, names)? {
        return Ok(children);
    }
    names.truncate(start);
    // Preserve last-duplicate identity and sorted traversal for malformed input trees.
    read_children_buffered(prefix, tree_ids, objects, names, buffers, ordered, nested_names)
}

#[expect(clippy::type_complexity)]
fn read_children_buffered(
    prefix: &BStr,
    tree_ids: [Option<ObjectId>; 3],
    objects: &impl gix_object::Find,
    names: &mut Vec<u8>,
    buffers: &mut [Vec<u8>; 3],
    ordered: bool,
    nested_names: &mut bool,
) -> Result<Vec<(Range<usize>, [Option<Entry>; 3])>> {
    let mut positions = HashMap::<&BStr, usize>::new();
    let mut children = Vec::<(&BStr, [Option<Entry>; 3])>::new();
    let mut sides: [Vec<usize>; 3] = Default::default();
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
            let index = *positions.entry(entry.filename).or_insert_with(|| {
                *nested_names |= entry.filename.contains(&b'/');
                let index = children.len();
                children.push((entry.filename, [None; 3]));
                index
            });
            for (side, input_tree_id) in tree_ids.into_iter().enumerate() {
                if input_tree_id == Some(tree_id) {
                    children[index].1[side] = Some(version);
                    if ordered {
                        sides[side].push(index);
                    }
                }
            }
        }
    }
    let prefix = child_prefix(prefix);
    let mut output = Vec::with_capacity(children.len());
    let mut append = |(filename, inputs): (&BStr, [Option<Entry>; 3])| {
        let start = names.len();
        names.extend_from_slice(&prefix);
        names.extend_from_slice(filename);
        output.push((start..names.len(), inputs));
    };
    if !ordered {
        children.into_iter().for_each(append);
        return Ok(output);
    }
    for (side, entries) in sides.iter_mut().enumerate() {
        let compare = |&a: &usize, &b: &usize| {
            gix_object::tree::name_order(
                children[a].0,
                children[a].1[side].is_some_and(|entry| entry.mode.is_tree()),
                children[b].0,
                children[b].1[side].is_some_and(|entry| entry.mode.is_tree()),
            )
        };
        if !entries.is_sorted_by(|a, b| compare(a, b).is_le()) {
            entries.sort_by(compare);
        }
    }
    // Match traverse_trees(): consume matching names even beyond punctuation-sorted siblings.
    let mut consumed = vec![false; children.len()];
    let mut cursors = [0; 3];
    loop {
        for side in 0..3 {
            while sides[side].get(cursors[side]).is_some_and(|&index| consumed[index]) {
                cursors[side] += 1;
            }
        }
        let Some(next) = (0..3)
            .filter_map(|side| sides[side].get(cursors[side]).copied())
            .min_by_key(|&index| children[index].0)
        else {
            break;
        };
        consumed[next] = true;
        append(children[next]);
    }
    Ok(output)
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
    use bstr::{BStr, ByteSlice};
    use gix_error::TestResult;
    use gix_object::{FindExt, Write, WriteTo, tree::EntryKind};

    use super::{Collected, Entry, Path, Resolution, Stage, State, read_children, read_children_stream};

    fn write_unsorted_tree(
        objects: &impl Write,
        entries: Vec<gix_object::tree::Entry>,
    ) -> TestResult<gix_hash::ObjectId> {
        let mut data = Vec::new();
        // Serialize entries individually to preserve malformed ordering despite the tree writer's assertion.
        for entry in entries {
            gix_object::Tree { entries: vec![entry] }.write_to(&mut data)?;
        }
        Ok(objects.write_buf(gix_object::Kind::Tree, &data)?)
    }

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
            let collected = Collected::collect([&empty_tree_id; 3], &gix_object::find::Never::panic_on_access(), true)?;
            let state = State::new(&collected.names, collected.entries, collected.unchanged, false);
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
        let collected = Collected::collect([&base_tree_id, &our_tree_id, &their_tree_id], &objects, true)?;
        let mut state = State::new(&collected.names, collected.entries, collected.unchanged, false);
        let directory = state
            .entries
            .get_mut(BStr::new("d"))
            .expect("the path exists on two inputs");
        assert!(directory.inputs[0].expect("the base contains a file").mode.is_blob());
        assert!(directory.inputs[1].expect("ours contains a directory").mode.is_tree());
        assert_eq!(directory.inputs[2], None, "the other side deleted the path");
        assert!(
            std::ptr::eq(
                directory.stages[0]
                    .as_ref()
                    .expect("the base has a stage")
                    .source
                    .as_ptr(),
                directory.stages[1].as_ref().expect("ours has a stage").source.as_ptr(),
            ),
            "versions of one path share its original name storage"
        );
        let original = directory.inputs;
        let stage = directory.stages[1].take().expect("ours has a normalized stage");
        state.entries.entry(BStr::new("renamed").into()).or_default().stages[1] = Some(stage);
        assert_eq!(
            state.entries[BStr::new("d")].inputs,
            original,
            "normalization cannot erase input versions"
        );
        assert_eq!(
            state.entries[BStr::new("d")].directory_mask,
            0b010,
            "moving a file stage cannot erase the directory's original side"
        );
        assert_eq!(
            state.entries[BStr::new("renamed")].stages[1]
                .as_ref()
                .expect("the stage moved to its normalized destination")
                .source
                .as_bstr(),
            "d",
            "the renamed stage retains its input path"
        );
        assert!(
            state.entries.contains_key(b"d/child\xff".as_slice().as_bstr()),
            "Git paths remain byte-oriented"
        );
        Ok(())
    }

    #[test]
    fn collection_retains_three_input_order_across_file_directory_names() -> TestResult {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let blob_id = objects.write_buf(gix_object::Kind::Blob, b"original")?;
        let changed_blob_id = objects.write_buf(gix_object::Kind::Blob, b"changed")?;
        let entry = |name: &str, mode: EntryKind, oid| gix_object::tree::Entry {
            filename: name.into(),
            mode: mode.into(),
            oid,
        };
        let child_tree_id = objects.write(&gix_object::Tree {
            entries: vec![entry("child", EntryKind::Blob, blob_id)],
        })?;
        let base_tree_id = objects.write(&gix_object::Tree {
            entries: vec![
                entry("a.", EntryKind::Blob, blob_id),
                entry("a", EntryKind::Tree, child_tree_id),
                entry("a0", EntryKind::Blob, blob_id),
            ],
        })?;
        let our_tree_id = objects.write(&gix_object::Tree {
            entries: vec![
                entry("a", EntryKind::Blob, blob_id),
                entry("a.", EntryKind::Blob, changed_blob_id),
            ],
        })?;
        let common_front_tree_id = objects.write(&gix_object::Tree {
            entries: vec![
                entry("a-", EntryKind::Blob, blob_id),
                entry("a.", EntryKind::Blob, blob_id),
            ],
        })?;
        for (our_tree_id, expected) in [
            (our_tree_id, ["a", "a/child", "a.", "a0"]),
            (common_front_tree_id, ["a-", "a", "a/child", "a0"]),
        ] {
            let collected = Collected::collect([&base_tree_id, &our_tree_id, &base_tree_id], &objects, true)?;
            let traversal: Vec<_> = collected.entries.iter().map(|(name, _)| name.clone()).collect();
            let state = State::new(&collected.names, collected.entries, collected.unchanged, false);
            let unordered = Collected::collect([&base_tree_id, &our_tree_id, &base_tree_id], &objects, false)?;
            let unordered = State::new(&unordered.names, unordered.entries, unordered.unchanged, false);
            assert_eq!(
                unordered.unchanged, state.unchanged,
                "ordering cannot change compact input identities"
            );
            assert_eq!(
                unordered.entries.len(),
                state.entries.len(),
                "ordering retains every explicit input path"
            );
            for (path, node) in &state.entries {
                assert_eq!(
                    unordered.entries[path].inputs, node.inputs,
                    "ordering retains all three original versions"
                );
            }
            assert_eq!(
                traversal
                    .iter()
                    .map(|name| collected.names[name.clone()].as_bstr())
                    .collect::<Vec<_>>(),
                expected.map(|path| path.as_bytes().as_bstr()),
                "matching names are consumed without dropping common traversal fronts"
            );
        }
        Ok(())
    }

    #[test]
    fn streaming_siblings_match_a_directory_behind_multiple_punctuation_names() -> TestResult {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let blob_id = objects.write_buf(gix_object::Kind::Blob, b"payload")?;
        let empty_tree_id = gix_hash::ObjectId::empty_tree(gix_testtools::object_hash());
        let entry = |name: &str, kind: EntryKind, oid| gix_object::tree::Entry {
            filename: name.into(),
            mode: kind.into(),
            oid,
        };
        let base_tree_id = objects.write(&gix_object::Tree {
            entries: vec![
                entry("t-1", EntryKind::Blob, blob_id),
                entry("t-2", EntryKind::Blob, blob_id),
                entry("t", EntryKind::Tree, empty_tree_id),
                entry("t=1", EntryKind::Blob, blob_id),
            ],
        })?;
        let our_tree_id = objects.write(&gix_object::Tree {
            entries: vec![entry("t", EntryKind::Blob, blob_id)],
        })?;
        let mut buffers: [Vec<u8>; 2] = Default::default();
        let [base_buffer, our_buffer] = &mut buffers;
        let iterators = [
            objects.find_tree_iter(&base_tree_id, base_buffer)?,
            objects.find_tree_iter(&our_tree_id, our_buffer)?,
            gix_object::TreeRefIter::default(),
        ];
        let mut names = Vec::new();
        let children = read_children_stream(
            "".into(),
            [Some(base_tree_id), Some(our_tree_id), Some(base_tree_id)],
            iterators,
            &mut names,
        )?
        .expect("valid punctuation-sensitive trees use streaming collection");
        assert_eq!(
            children
                .iter()
                .map(|(range, _)| names[range.clone()].as_bstr())
                .collect::<Vec<_>>(),
            ["t", "t-1", "t-2", "t=1"],
            "the matched directory is returned early and skipped when its original cursor catches up"
        );
        let inputs = children[0].1;
        assert_eq!(inputs[0], inputs[2], "identical input trees share the directory entry");
        assert_eq!(inputs[0].expect("the base contains t").id, empty_tree_id);
        assert_eq!(inputs[1].expect("ours contains t").id, blob_id);
        Ok(())
    }

    #[test]
    fn sibling_collection_preserves_duplicate_unsorted_and_malformed_names() -> TestResult {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let original_blob_id = objects.write_buf(gix_object::Kind::Blob, b"original")?;
        let changed_blob_id = objects.write_buf(gix_object::Kind::Blob, b"changed")?;
        let empty_tree_id = gix_hash::ObjectId::empty_tree(gix_testtools::object_hash());
        let entry = |name: &str, mode: EntryKind, oid| gix_object::tree::Entry {
            filename: name.into(),
            mode: mode.into(),
            oid,
        };
        for (entries, expected, nested) in [
            (
                vec![
                    entry("a", EntryKind::Blob, original_blob_id),
                    entry("a.", EntryKind::Blob, original_blob_id),
                    entry("a", EntryKind::Tree, empty_tree_id),
                ],
                vec![("parent/a", empty_tree_id), ("parent/a.", original_blob_id)],
                false,
            ),
            (
                vec![
                    entry("z", EntryKind::Blob, original_blob_id),
                    entry("a", EntryKind::Blob, original_blob_id),
                    entry("z", EntryKind::Blob, changed_blob_id),
                ],
                vec![("parent/a", original_blob_id), ("parent/z", changed_blob_id)],
                false,
            ),
            (
                vec![entry("bad/path", EntryKind::Blob, changed_blob_id)],
                vec![("parent/a", original_blob_id), ("parent/bad/path", changed_blob_id)],
                true,
            ),
        ] {
            let base_tree_id = write_unsorted_tree(&objects, entries)?;
            let our_tree_id = objects.write(&gix_object::Tree {
                entries: vec![entry("a", EntryKind::Blob, original_blob_id)],
            })?;
            for ordered in [false, true] {
                let mut names = Vec::new();
                let mut buffers = Default::default();
                let mut nested_names = false;
                let children = read_children(
                    "parent".into(),
                    [Some(base_tree_id), Some(our_tree_id), Some(base_tree_id)],
                    &objects,
                    &mut names,
                    &mut buffers,
                    ordered,
                    &mut nested_names,
                )?;
                let mut actual: Vec<_> = children
                    .into_iter()
                    .map(|(name, inputs)| {
                        assert_eq!(inputs[0], inputs[2], "identical trees share original input entries");
                        (
                            names[name].as_bstr(),
                            inputs[0].map_or(original_blob_id, |entry| entry.id),
                        )
                    })
                    .collect();
                actual.sort();
                assert_eq!(
                    actual,
                    expected
                        .iter()
                        .map(|(name, id)| (name.as_bytes().as_bstr(), *id))
                        .collect::<Vec<_>>(),
                    "duplicate names keep the last input while every sibling survives reordered input"
                );
                assert_eq!(
                    nested_names, nested,
                    "separator-containing filenames disable compact resolution"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn trivial_inputs_preserve_occupancy_and_forced_restoration() -> TestResult {
        let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
        let mut base = leaf();
        base.id = objects.write_buf(gix_object::Kind::Blob, b"base")?;
        let mut changed = base;
        changed.id = objects.write_buf(gix_object::Kind::Blob, b"changed")?;
        let directory = Entry {
            mode: EntryKind::Tree.into(),
            id: gix_hash::ObjectId::empty_tree(gix_testtools::object_hash()),
        };
        let mut names = Vec::new();
        let entries = [
            ("dir", [None, Some(directory), Some(base)]),
            ("dir/file", [None, Some(changed), None]),
            ("gone", [Some(base), None, None]),
            ("taken~A", [Some(base), None, Some(base)]),
            ("duplicate", [Some(base), None, None]),
            ("duplicate", [Some(base), Some(base), Some(changed)]),
        ]
        .map(|(path, inputs)| {
            let start = names.len();
            names.extend_from_slice(path.as_bytes());
            (start..names.len(), inputs)
        });
        let mut state = State::new(&names, entries.into(), Vec::new(), true);
        assert_eq!(
            state.trivial[BStr::new("duplicate")].entry(),
            Some(changed),
            "compact construction preserves the last input for duplicate paths"
        );
        assert!(
            state.trivial.contains_key(BStr::new("gone")),
            "clean deletions retain compact original inputs"
        );
        state.prepare_resolution();
        assert!(
            state.has_live_descendants("dir".into()),
            "a compact selected child keeps its directory alive"
        );
        state.restore_input("gone".into(), 0);
        assert_eq!(
            state.entries[BStr::new("gone")].result,
            Resolution::Present(base),
            "forced restoration uses the original input"
        );
        state.ensure_path("dir/file".into(), &objects)?;
        assert_eq!(
            state.entries[BStr::new("dir/file")].inputs,
            [None, Some(changed), None],
            "materialization preserves all inputs"
        );
        state.set_result("dir/file".into(), None);
        assert!(
            !state.has_live_descendants("dir".into()),
            "removing a materialized child balances the compact live count"
        );
        assert_eq!(
            state.reserve_unique("taken".into(), "A".into(), &objects)?,
            "taken~A_0",
            "deleted input names still reserve their original namespace"
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
            state.entries.insert(BStr::new(path).into(), Box::new(node));
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
        state.entries.entry(BStr::new("d").into()).or_default().stages[1] = Some(Stage {
            merge: None,
            entry: leaf(),
            source: BStr::new("d").into(),
        });
        state.entries.entry(BStr::new("d/file").into()).or_default().result = Resolution::Present(leaf());
        state
            .entries
            .entry(BStr::new("d-other/file").into())
            .or_default()
            .result = Resolution::Present(leaf());
        state.unchanged.insert(BStr::new("common/a").into(), leaf());
        state.unchanged.insert(BStr::new("common/b").into(), leaf());
        state.prepare_resolution();
        state.materialize("common/a".into());
        state.set_result("common/a".into(), None);
        assert!(
            state.has_live_descendants("common".into()),
            "a common sibling still survives"
        );
        state.materialize("common/b".into());
        state.set_result("common/b".into(), None);
        assert!(
            !state.has_live_descendants("common".into()),
            "the last common child updates its parent's count"
        );
        assert!(
            state.has_live_descendants("d".into()),
            "a resolved descendant keeps its directory alive"
        );
        assert_eq!(
            state.reserve_unique("d/new".into(), "A".into(), &objects)?,
            "d~A/new",
            "a blocking component itself must move"
        );
        state.set_result("d/file".into(), None);
        state.set_result("d".into(), None);
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
                .position(|path| path.as_bstr() == "d/file")
                .expect("the child stays in the path set")
                < paths
                    .iter()
                    .position(|path| path.as_bstr() == "d")
                    .expect("the parent stays in the path set"),
            "children are visited before their parent"
        );
        state.set_resolution("d/file".into(), Resolution::Present(leaf()));
        assert!(
            state.has_live_descendants("d".into()),
            "restoring a child updates its parent count"
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
        let collected = Collected::collect([&fixture.root_tree_id; 3], &fixture.objects, true)?;
        let mut state = State::new(&collected.names, collected.entries, collected.unchanged, false);
        assert!(state.entries.is_empty(), "identical inputs need no mutable stages");
        assert_eq!(
            state.unchanged.len(),
            2,
            "identical descendants are represented by their two subtree roots"
        );
        state.ensure_path("dir".into(), &fixture.objects)?;
        let directory = state.entries.get_mut(BStr::new("dir")).expect("the subtree is indexed");
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
        let padded_tree_id = fixture.objects.write(&gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: gix_object::tree::EntryMode::from_bytes(b"040000 ").expect("zero-padded tree modes are valid"),
                filename: "dir".into(),
                oid: fixture.subtree_id,
            }],
        })?;
        let collected = Collected::collect([&padded_tree_id; 3], &fixture.objects, true)?;
        assert!(
            collected.unchanged.is_empty(),
            "a shared noncanonical directory does not collect its descendants"
        );
        let state = State::new(&collected.names, collected.entries, collected.unchanged, false);
        assert_eq!(state.entries.len(), 1, "only the noncanonical subtree root is indexed");
        assert!(
            state.entries[BStr::new("dir")].opaque,
            "the shared directory remains opaque"
        );
        Ok(())
    }

    #[test]
    fn namespace_expansion_sees_hidden_entries_without_resurrecting_discarded_subtrees() -> TestResult {
        let fixture = opaque_fixture()?;
        for retained in [true, false] {
            let collected = Collected::collect([&fixture.root_tree_id; 3], &fixture.objects, true)?;
            let mut state = State::new(&collected.names, collected.entries, collected.unchanged, false);
            state.ensure_path("dir".into(), &fixture.objects)?;
            if !retained {
                state
                    .entries
                    .get_mut(BStr::new("dir"))
                    .expect("the subtree is indexed")
                    .result = Resolution::Absent;
            }
            state.prepare_resolution();
            assert_eq!(
                state.reserve_unique("dir/taken".into(), "A".into(), &fixture.objects)?,
                "dir/taken~A_0",
                "a name hidden in an opaque subtree remains occupied"
            );
            assert_eq!(
                state.entries[BStr::new("dir")].result,
                Resolution::Absent,
                "expanded directories no longer emit their stale tree"
            );
            assert_eq!(
                state.has_live_descendants("dir".into()),
                retained,
                "namespace lookup must retain the directory's resolved fate"
            );
            assert!(
                state.unchanged.contains_key(BStr::new("dir-other")),
                "looking up one subtree leaves its sibling compact"
            );
            assert_eq!(
                state.entries[BStr::new("dir/taken~A")].stages[1]
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
        let collected = Collected::collect([&fixture.root_tree_id; 3], &fixture.objects, true)?;
        let mut state = State::new(&collected.names, collected.entries, collected.unchanged, false);
        state.ensure_path("dir/taken~A".into(), &fixture.objects)?;
        state.ensure_subtree("dir".into(), &fixture.objects)?;
        assert!(
            !state.entries[BStr::new("dir/nested")].opaque,
            "the touched region is fully expanded"
        );
        assert!(
            state.unchanged.contains_key(BStr::new("dir-other")),
            "unrelated regions remain compact"
        );
        let directory = state.entries.get_mut(BStr::new("dir")).expect("the subtree is indexed");
        directory.restore_input(0);
        assert_eq!(
            directory.result,
            Resolution::Absent,
            "restoring an expanded directory does not emit its stale input tree"
        );
        let leaf = state
            .entries
            .get_mut(BStr::new("dir/nested/kept"))
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
        let moved = state.entries.entry(BStr::new("moved").into()).or_default();
        moved.set_unmerged(inputs, None);
        moved.set_stage(1, Some(stage));
        assert!(moved.unmerged.is_none(), "a moved stage replaces the old index owner");
        assert_eq!(
            state.entries[BStr::new("moved")].stages[1]
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
                    source: BStr::new(source).into(),
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
