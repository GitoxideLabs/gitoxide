use gix_hash::ObjectId;
use gix_object::{
    bstr::{BStr, BString, ByteSlice, ByteVec},
    tree,
};

use crate::tree::{visit::Action, Recorder, Visit};

/// Describe how to track the location of an entry.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Location {
    /// Track the entire path, relative to the repository.
    Path,
    /// Keep only the file-name as location, which may be enough for some calculations.
    ///
    /// This is less expensive than tracking the entire `Path`.
    FileName,
}

/// An owned entry as observed by a call to [`visit_(tree|nontree)(…)`][Visit::visit_tree()], enhanced with the full path to it.
/// Otherwise similar to [`gix_object::tree::EntryRef`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The kind of entry, similar to entries in a unix directory tree.
    pub mode: tree::EntryMode,
    /// The full path to the entry. A root entry would be `d`, and a file `a` within the directory would be `d/a`.
    ///
    /// This is independent of the platform and the path separators actually used there.
    pub filepath: BString,
    /// The id of the entry which can be used to locate it in an object database.
    pub oid: ObjectId,
}

impl Entry {
    fn new(entry: &tree::EntryRef<'_>, filepath: BString) -> Self {
        Entry {
            filepath,
            oid: entry.oid.to_owned(),
            mode: entry.mode,
        }
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Recorder {
            path_deque: Default::default(),
            path: Default::default(),
            location: Location::Path.into(),
            records: vec![],
        }
    }
}

impl Recorder {
    fn pop_element(&mut self) {
        if let Some(pos) = self.path.rfind_byte(b'/') {
            self.path.resize(pos, 0);
        } else {
            self.path.clear();
        }
    }

    fn push_element(&mut self, name: &BStr) {
        if name.is_empty() {
            return;
        }
        if !self.path.is_empty() {
            self.path.push(b'/');
        }
        self.path.push_str(name);
    }
}

/// Builder
impl Recorder {
    /// Obtain a copy of the currently tracked, full path of the entry.
    pub fn track_location(mut self, location: Option<Location>) -> Self {
        self.location = location;
        self
    }
}

/// Access
impl Recorder {
    /// Obtain a copy of the currently tracked, full path of the entry.
    pub fn path_clone(&self) -> BString {
        self.path.clone()
    }

    /// Return the currently set path.
    pub fn path(&self) -> &BStr {
        self.path.as_ref()
    }
}

impl Visit for Recorder {
    fn pop_back_tracked_path_and_set_current(&mut self) {
        if let Some(Location::Path) = self.location {
            self.path = self.path_deque.pop_back().unwrap_or_default();
        }
    }

    fn pop_front_tracked_path_and_set_current(&mut self) {
        if let Some(Location::Path) = self.location {
            self.path = self
                .path_deque
                .pop_front()
                .expect("every call is matched with push_tracked_path_component");
        }
    }

    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        if let Some(Location::Path) = self.location {
            self.push_element(component);
            self.path_deque.push_back(self.path.clone());
        }
    }

    fn push_path_component(&mut self, component: &BStr) {
        match self.location {
            None => {}
            Some(Location::Path) => {
                self.push_element(component);
            }
            Some(Location::FileName) => {
                self.path.clear();
                self.path.extend_from_slice(component);
            }
        }
    }

    fn pop_path_component(&mut self) {
        if let Some(Location::Path) = self.location {
            self.pop_element();
        }
    }

    fn visit_tree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        self.records.push(Entry::new(entry, self.path_clone()));
        std::ops::ControlFlow::Continue(true)
    }

    fn visit_nontree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        self.records.push(Entry::new(entry, self.path_clone()));
        std::ops::ControlFlow::Continue(true)
    }
}
