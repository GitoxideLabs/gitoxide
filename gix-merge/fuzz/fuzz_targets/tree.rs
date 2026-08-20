#![no_main]

use std::{collections::BTreeMap, path::Path as FsPath};

use gix_diff::Rewrites;
use gix_hash::ObjectId;
use gix_merge::{
    blob::builtin_driver::binary,
    tree::{Options, ResolveWith, TreatAsUnresolved},
};
use gix_object::{
    Kind, Tree, Write,
    tree::{EntryKind, EntryMode},
};
use gix_worktree::stack::state::attributes;
use libfuzzer_sys::{Corpus, fuzz_target};

const OPERATION_SIZE: usize = 12;
const MAX_OPERATIONS: usize = 64;
const MAX_INPUT_SIZE: usize = 1 + OPERATION_SIZE * MAX_OPERATIONS;
const COMPONENTS: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];
const PAYLOAD_COUNT: usize = 8;

type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Path {
    len: u8,
    components: [u8; 3],
}

impl Path {
    fn from_bytes(bytes: &[u8]) -> Self {
        Path {
            len: 1 + bytes[0] % 3,
            components: [bytes[1] % 8, bytes[2] % 8, bytes[3] % 8],
        }
    }

    fn is_prefix_of(self, other: Path) -> bool {
        self.len <= other.len && self.components[..self.len as usize] == other.components[..self.len as usize]
    }

    fn below(self, source: Path, destination: Path) -> Option<Path> {
        if !source.is_prefix_of(self) {
            return None;
        }
        let suffix_len = self.len - source.len;
        let len = destination.len + suffix_len;
        if len > 3 {
            return None;
        }

        let mut components = destination.components;
        components[destination.len as usize..len as usize]
            .copy_from_slice(&self.components[source.len as usize..self.len as usize]);
        Some(Path { len, components })
    }

    fn components(self) -> impl Iterator<Item = &'static str> {
        self.components
            .into_iter()
            .take(self.len as usize)
            .map(|component| COMPONENTS[component as usize])
    }
}

#[derive(Clone, Copy)]
enum Entry {
    Blob(u8),
    Executable(u8),
    Link(u8),
    Commit(u8),
}

impl Entry {
    fn from_bytes(mode: u8, payload: u8) -> Self {
        let payload = payload % PAYLOAD_COUNT as u8;
        match mode % 4 {
            0 => Entry::Blob(payload),
            1 => Entry::Executable(payload),
            2 => Entry::Link(payload),
            _ => Entry::Commit(payload),
        }
    }

    fn mode(self) -> EntryMode {
        match self {
            Entry::Blob(_) => EntryKind::Blob.into(),
            Entry::Executable(_) => EntryKind::BlobExecutable.into(),
            Entry::Link(_) => EntryKind::Link.into(),
            Entry::Commit(_) => EntryKind::Commit.into(),
        }
    }

    fn payload(self) -> usize {
        match self {
            Entry::Blob(payload) | Entry::Executable(payload) | Entry::Link(payload) | Entry::Commit(payload) => {
                payload as usize
            }
        }
    }
}

#[derive(Clone, Copy)]
enum Action {
    Set,
    Remove,
    Modify,
    Rename,
}

#[derive(Clone, Copy)]
struct Operation {
    action: Action,
    path: Path,
    destination: Path,
    entry: Entry,
}

impl Operation {
    fn from_bytes(bytes: &[u8]) -> Self {
        Operation {
            action: match bytes[1] % 4 {
                0 => Action::Set,
                1 => Action::Remove,
                2 => Action::Modify,
                _ => Action::Rename,
            },
            path: Path::from_bytes(&bytes[2..6]),
            destination: Path::from_bytes(&bytes[6..10]),
            entry: Entry::from_bytes(bytes[10], bytes[11]),
        }
    }
}

#[derive(Clone, Default)]
struct State(BTreeMap<Path, Entry>);

impl State {
    fn apply(&mut self, operation: Operation) {
        match operation.action {
            Action::Set => self.set(operation.path, operation.entry),
            Action::Remove => self.remove(operation.path),
            Action::Modify => self.modify(operation.path, operation.entry),
            Action::Rename => self.rename(operation.path, operation.destination),
        }
    }

    fn set(&mut self, path: Path, entry: Entry) {
        self.0
            .retain(|existing, _| !existing.is_prefix_of(path) && !path.is_prefix_of(*existing));
        self.0.insert(path, entry);
    }

    fn remove(&mut self, path: Path) {
        self.0.retain(|existing, _| !path.is_prefix_of(*existing));
    }

    fn modify(&mut self, path: Path, entry: Entry) {
        let target = self
            .0
            .contains_key(&path)
            .then_some(path)
            .or_else(|| self.0.keys().copied().find(|candidate| path.is_prefix_of(*candidate)));
        if let Some(target) = target {
            self.0.insert(target, entry);
        }
    }

    fn rename(&mut self, source: Path, destination: Path) {
        let moved: Vec<_> = self
            .0
            .iter()
            .filter_map(|(path, entry)| path.below(source, destination).map(|path| (path, *entry)))
            .collect();
        if moved.is_empty() {
            return;
        }

        self.remove(source);
        for (path, entry) in moved {
            self.set(path, entry);
        }
    }
}

struct Objects {
    db: ObjectDb,
    blobs: [ObjectId; PAYLOAD_COUNT],
    commits: [ObjectId; PAYLOAD_COUNT],
}

impl Objects {
    fn new() -> Self {
        let db = ObjectDb::new(gix_object::find::Never, gix_hash::Kind::Sha1);
        let empty_tree = db.write(&Tree::default()).expect("the in-memory tree can be written");
        let blobs = std::array::from_fn(|index| {
            let data = format!("binary payload {index}\0\n");
            db.write_buf(Kind::Blob, data.as_bytes())
                .expect("the in-memory blob can be written")
        });
        let commits = std::array::from_fn(|index| {
            let data = format!(
                "tree {empty_tree}\nauthor A <a@example.com> 0 +0000\ncommitter A <a@example.com> 0 +0000\n\ncommit {index}\n"
            );
            db.write_buf(Kind::Commit, data.as_bytes())
                .expect("the in-memory commit can be written")
        });
        Objects { db, blobs, commits }
    }

    fn id(&self, entry: Entry) -> ObjectId {
        match entry {
            Entry::Commit(_) => self.commits[entry.payload()],
            Entry::Blob(_) | Entry::Executable(_) | Entry::Link(_) => self.blobs[entry.payload()],
        }
    }

    fn write_tree(&self, state: &State) -> ObjectId {
        let mut editor = gix_object::tree::Editor::new(Tree::default(), &gix_object::find::Never, gix_hash::Kind::Sha1);
        for (path, entry) in &state.0 {
            editor
                .upsert(path.components(), entry.mode().kind(), self.id(*entry))
                .expect("generated paths form a valid tree");
        }
        editor
            .write(|tree| self.db.write(tree))
            .expect("the generated tree can be written")
    }
}

fn fuzz(data: &[u8]) {
    let Some((&configuration, operations)) = data.split_first() else {
        return;
    };

    let mut base = State::default();
    let mut ours = Vec::new();
    let mut theirs = Vec::new();
    // A small component alphabet makes unrelated operations collide often. Set, remove, modify,
    // and subtree rename records can thereby form additions, type changes, file/directory
    // replacements, and rename interactions on either side of the same valid base tree.
    for bytes in operations.chunks_exact(OPERATION_SIZE).take(MAX_OPERATIONS) {
        let operation = Operation::from_bytes(bytes);
        match bytes[0] % 3 {
            0 => base.set(operation.path, operation.entry),
            1 => ours.push(operation),
            _ => theirs.push(operation),
        }
    }

    let mut ours_state = base.clone();
    for operation in ours {
        ours_state.apply(operation);
    }
    let mut theirs_state = base.clone();
    for operation in theirs {
        theirs_state.apply(operation);
    }

    let objects = Objects::new();
    let base = objects.write_tree(&base);
    let ours = objects.write_tree(&ours_state);
    let theirs = objects.write_tree(&theirs_state);
    let options = options(configuration);
    let mut diff_state = gix_diff::tree::State::default();
    let mut diff_resource_cache = new_diff_resource_cache();
    let mut blob_merge = new_blob_merge_platform();

    for (current, other) in [(ours, theirs), (theirs, ours)] {
        let outcome = gix_merge::tree(
            &base,
            &current,
            &other,
            gix_merge::blob::builtin_driver::text::Labels {
                ancestor: Some("BASE".into()),
                current: Some("OURS".into()),
                other: Some("THEIRS".into()),
            },
            &objects.db,
            |buf| objects.db.write_buf(Kind::Blob, buf),
            &mut diff_state,
            &mut diff_resource_cache,
            &mut blob_merge,
            options.clone(),
        );
        let mut outcome = match outcome {
            Ok(outcome) => outcome,
            // Resolving a binary add/add conflict with its absent ancestor cannot
            // produce a resource. This is a valid configuration-dependent error.
            Err(err) if err.downcast_any_ref::<gix_error::NotFoundError>().is_some() => continue,
            Err(err) => panic!("generated trees and objects are valid: {err:?}"),
        };
        outcome
            .tree
            .write(|tree| objects.db.write(tree))
            .expect("the merged tree remains valid");
    }
}

fn options(configuration: u8) -> Options {
    let binary_resolution = |value| match value % 4 {
        0 => None,
        1 => Some(binary::ResolveWith::Ancestor),
        2 => Some(binary::ResolveWith::Ours),
        _ => Some(binary::ResolveWith::Theirs),
    };
    let mut options = Options {
        // Identity-only rewrite tracking exercises rename handling without invoking blob similarity
        // diffing. Rename operations preserve object IDs so they remain discoverable.
        rewrites: Some(Rewrites {
            copies: None,
            percentage: Some(1.0),
            limit: 0,
            track_empty: false,
        }),
        fail_on_conflict: (configuration & 0b1000_0000 != 0).then(TreatAsUnresolved::git),
        marker_size_multiplier: configuration % 4,
        symlink_conflicts: binary_resolution(configuration >> 2),
        tree_conflicts: match (configuration >> 4) % 3 {
            0 => None,
            1 => Some(ResolveWith::Ancestor),
            _ => Some(ResolveWith::Ours),
        },
        ..Default::default()
    };
    options.blob_merge.resolve_binary_with = binary_resolution(configuration);
    // All generated blobs are binary by both the NUL-byte and size rules, so tree fuzzing never
    // reaches text diffing. Keep Histogram explicit as a final guard if that invariant changes.
    options.blob_merge.text.diff_algorithm = imara_diff::Algorithm::Histogram;
    options
}

fn new_diff_resource_cache() -> gix_diff::blob::Platform {
    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
        Default::default(),
        gix_worktree::Stack::new(
            FsPath::new("gix-merge-tree-fuzz-no-worktree"),
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}

fn new_blob_merge_platform() -> gix_merge::blob::Platform {
    let attributes = gix_worktree::Stack::new(
        FsPath::new("gix-merge-tree-fuzz-no-worktree"),
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
            Default::default(),
            None,
            attributes::Source::WorktreeThenIdMapping,
            Default::default(),
        )),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    );
    gix_merge::blob::Platform::new(
        gix_merge::blob::Pipeline::new(
            Default::default(),
            gix_filter::Pipeline::default(),
            gix_merge::blob::pipeline::Options {
                large_file_threshold_bytes: 1,
            },
        ),
        gix_merge::blob::pipeline::Mode::ToGit,
        attributes,
        vec![],
        Default::default(),
    )
}

fuzz_target!(|data: &[u8]| -> Corpus {
    if data.len() > MAX_INPUT_SIZE {
        return Corpus::Reject;
    }
    fuzz(data);
    Corpus::Keep
});
