use std::collections::VecDeque;

use gix_error::{Result, ResultExt, bail, corruption, message};
use gix_hash::ObjectId;
use gix_object::{Find, Kind, ObjectRef};

/// One object in a reachable object graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The object's identifier.
    pub id: ObjectId,
    /// The kind read from the object database.
    pub kind: Kind,
}

/// Walk all objects reachable from arbitrary tips in breadth-first order.
///
/// Follow annotated tags, commit parents and trees, and tree entries. Gitlinks
/// name commits in other repositories and are not followed. Each object is
/// returned once, even when tips or graph edges repeat. Missing or malformed
/// objects, and edges pointing to the wrong object kind, stop the iterator with
/// an error. Object checksums are the responsibility of the supplied finder.
///
/// Memory grows with the number of reachable objects and the traversal frontier.
/// Order follows the supplied tips, then stored object edges; it is not Git's
/// `rev-list` display order. Callers can report progress or interrupt between items.
pub struct BreadthFirst<Find> {
    find: Find,
    pending: VecDeque<(ObjectId, Option<Kind>)>,
    seen: gix_hashtable::HashMap<ObjectId, Kind>,
    shallow: gix_hashtable::HashSet<ObjectId>,
    buffer: Vec<u8>,
    failed: bool,
    ignore_missing: bool,
    missing: gix_hashtable::HashSet<ObjectId>,
}

impl<Find> BreadthFirst<Find> {
    /// Start a traversal at `tips`, reading objects through `find`.
    pub fn new(tips: impl IntoIterator<Item = ObjectId>, find: Find) -> Self {
        Self {
            find,
            pending: tips.into_iter().map(|id| (id, None)).collect(),
            seen: Default::default(),
            shallow: Default::default(),
            buffer: Vec::new(),
            failed: false,
            ignore_missing: false,
            missing: Default::default(),
        }
    }

    /// Skip missing objects, while still failing on malformed objects and invalid edge kinds.
    ///
    /// This is useful when retaining recent unreachable objects: an older collection may have
    /// already removed some of their dependencies. It is not suitable for validating connectivity
    /// or collecting the graph of ordinary repository roots. Missing IDs are visited only once.
    pub fn with_missing_objects_ignored(mut self, ignore: bool) -> Self {
        self.ignore_missing = ignore;
        self
    }

    /// Do not follow parent edges of the given shallow commits.
    ///
    /// The commits themselves and their complete trees are still visited. The
    /// caller supplies the repository's shallow boundary; it is not discovered.
    pub fn with_shallow_commits(mut self, commits: impl IntoIterator<Item = ObjectId>) -> Self {
        self.shallow = commits.into_iter().collect();
        self
    }
}

impl<F: Find> BreadthFirst<F> {
    fn next_entry(&mut self) -> Result<Option<Entry>> {
        while let Some((object_id, expected_kind)) = self.pending.pop_front() {
            if let Some(kind) = self.seen.get(&object_id) {
                check_kind(object_id, *kind, expected_kind)?;
                continue;
            }
            if self.missing.contains(&object_id) {
                continue;
            }
            let object = self
                .find
                .try_find(&object_id, &mut self.buffer)
                .or_raise(|| message!("Could not read reachable object {object_id}"))?;
            let Some(object) = object else {
                if self.ignore_missing {
                    self.missing.insert(object_id);
                    continue;
                }
                bail!(message!("Reachable object {object_id} is missing").not_found());
            };
            let kind = object.kind;
            check_kind(object_id, kind, expected_kind)?;
            match object
                .decode()
                .or_raise(|| message!("Could not decode reachable object {object_id}"))?
            {
                ObjectRef::Blob(_) => {}
                ObjectRef::Tag(tag) => self.pending.push_back((tag.target(), Some(tag.target_kind))),
                ObjectRef::Commit(commit) => {
                    self.pending.push_back((commit.tree(), Some(Kind::Tree)));
                    if !self.shallow.contains(&object_id) {
                        self.pending.extend(commit.parents().map(|id| (id, Some(Kind::Commit))));
                    }
                }
                ObjectRef::Tree(tree) => {
                    for entry in tree.entries {
                        let kind = match entry.mode.kind() {
                            gix_object::tree::EntryKind::Commit => continue,
                            gix_object::tree::EntryKind::Tree => Kind::Tree,
                            _ => Kind::Blob,
                        };
                        self.pending.push_back((entry.oid.to_owned(), Some(kind)));
                    }
                }
            }
            self.seen.insert(object_id, kind);
            return Ok(Some(Entry { id: object_id, kind }));
        }
        Ok(None)
    }
}

impl<F: Find> Iterator for BreadthFirst<F> {
    type Item = Result<Entry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed {
            return None;
        }
        match self.next_entry() {
            Ok(entry) => entry.map(Ok),
            Err(err) => {
                self.failed = true;
                self.pending.clear();
                Some(Err(err))
            }
        }
    }
}

impl<F: Find> std::iter::FusedIterator for BreadthFirst<F> {}

fn check_kind(object_id: ObjectId, actual: Kind, expected: Option<Kind>) -> Result<()> {
    if let Some(expected) = expected
        && expected != actual
    {
        bail!(
            corruption("A reachable object has the wrong kind")
                .with("object", object_id.to_string())
                .with("expected", expected.to_string())
                .with("actual", actual.to_string())
        );
    }
    Ok(())
}
