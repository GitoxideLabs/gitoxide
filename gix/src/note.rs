//! Access Git notes.

pub use gix_note::*;

use gix_error::{ErrorExt, ResultExt, message};

use crate::{
    Blob, Repository,
    bstr::{BStr, BString, ByteSlice, ByteVec},
    config::tree::Core,
    refs::{FullName, transaction::PreviousValue},
};

/// A note and the reference from which it originated.
pub struct Note<'a> {
    /// The source notes reference.
    pub reference: FullName,
    /// The note blob.
    pub blob: Blob<'a>,
}

/// Cached access to one or more notes references.
pub struct Platform {
    pub(crate) repo: Repository,
    pub(crate) default_ref: Option<FullName>,
    pub(crate) refs: Vec<FullName>,
    pub(crate) roots: Vec<Option<Option<gix_hash::ObjectId>>>,
    pub(crate) cache: gix_note::Cache,
}

impl Platform {
    pub(crate) fn new(repo: Repository) -> Result<Self, Error> {
        let default_ref = match repo.config_snapshot().string(Core::NOTES_REF) {
            Some(value) if value.is_empty() => None,
            Some(value) => Some(
                FullName::try_from(value)
                    .or_raise(|| message("core.notesRef must be a fully qualified reference name"))?,
            ),
            None => Some(
                FullName::try_from("refs/notes/commits")
                    .expect("the standard notes reference is a valid full reference name"),
            ),
        };
        let mut refs = default_ref.iter().cloned().collect::<Vec<_>>();
        let display_from_environment = repo
            .open_options()
            .permissions
            .env
            .git_prefix
            .check_opt("GIT_NOTES_DISPLAY_REF")
            .and_then(std::env::var_os);
        let display_refs = match display_from_environment {
            Some(value) => {
                let value = gix_path::os_string_into_bstring(value)
                    .or_raise(|| message("GIT_NOTES_DISPLAY_REF is not representable as bytes"))?;
                value
                    .split(|byte| *byte == b':')
                    .filter(|value| !value.is_empty())
                    .map(BString::from)
                    .collect()
            }
            None => repo
                .config_snapshot()
                .plumbing()
                .strings("notes.displayRef")
                .unwrap_or_default(),
        };
        for pattern in display_refs {
            add_refs(&repo, pattern.as_bstr(), &mut refs)?;
        }
        let roots = vec![None; refs.len()];
        Ok(Platform {
            repo,
            default_ref,
            refs,
            roots,
            cache: Default::default(),
        })
    }

    /// Replace configured display references with `refs`.
    pub fn with_refs(mut self, refs: impl IntoIterator<Item = impl Into<BString>>) -> Result<Self, Error> {
        let mut selected = Vec::new();
        for pattern in refs {
            let pattern = pattern.into();
            add_refs(&self.repo, pattern.as_bstr(), &mut selected)?;
        }
        self.roots = vec![None; selected.len()];
        self.refs = selected;
        Ok(self)
    }

    /// Return the default notes reference selected by configuration.
    pub fn default_ref(&self) -> Option<&gix_ref::FullNameRef> {
        self.default_ref.as_ref().map(AsRef::as_ref)
    }

    /// Return all notes associated with `object` in configured display order.
    pub fn get(&mut self, object: impl Into<gix_hash::ObjectId>) -> Result<Vec<Note<'_>>, Error> {
        let object = object.into();
        let mut found = Vec::new();
        for index in 0..self.refs.len() {
            let Some(root) = self.root(index)? else { continue };
            if let Some(note) = gix_note::get(root, &object, &self.repo, &mut self.cache)
                .or_raise(|| message!("Could not find notes for {object}"))?
            {
                found.push((self.refs[index].clone(), note));
            }
        }
        found
            .into_iter()
            .map(|(reference, id)| {
                let blob = self
                    .repo
                    .find_blob(id)
                    .or_raise(|| message!("Could not load note {id} from {reference}"))?;
                Ok(Note { reference, blob })
            })
            .collect()
    }

    /// Add or replace a note in `notes_ref`, returning the previous note id.
    pub fn add(
        &mut self,
        notes_ref: impl Into<BString>,
        object: impl Into<gix_hash::ObjectId>,
        data: impl AsRef<[u8]>,
    ) -> Result<Option<gix_hash::ObjectId>, Error> {
        let notes_ref = expand_notes_ref(notes_ref.into())?;
        self.add_to_ref(notes_ref.as_ref(), object, data)
    }

    /// Add or replace a note in the fully qualified `notes_ref`, returning the previous note id.
    ///
    /// Unlike [`Platform::add()`], this does not apply Git's `refs/notes/` shorthand expansion.
    pub fn add_to_ref(
        &mut self,
        notes_ref: &gix_ref::FullNameRef,
        object: impl Into<gix_hash::ObjectId>,
        data: impl AsRef<[u8]>,
    ) -> Result<Option<gix_hash::ObjectId>, Error> {
        let (root, parent) = self.edit_root(notes_ref)?;
        let object = object.into();
        let note = self
            .repo
            .write_blob(data)
            .or_raise(|| message!("Could not write note for {object}"))?
            .detach();
        let edit = gix_note::add(root, object, note, &self.repo, &mut self.cache)
            .or_raise(|| message!("Could not add note for {object}"))?;
        self.commit_edit(notes_ref.to_owned(), parent, edit, "Notes added by gitoxide")?;
        Ok(edit.previous)
    }

    /// Remove a note in `notes_ref`, returning the removed note id.
    pub fn remove(
        &mut self,
        notes_ref: impl Into<BString>,
        object: impl Into<gix_hash::ObjectId>,
    ) -> Result<Option<gix_hash::ObjectId>, Error> {
        let notes_ref = expand_notes_ref(notes_ref.into())?;
        let (root, parent) = self.edit_root(notes_ref.as_ref())?;
        let object = object.into();
        let edit = gix_note::remove(root, object, &self.repo, &mut self.cache)
            .or_raise(|| message!("Could not remove note for {object}"))?;
        if edit.previous.is_some() {
            self.commit_edit(notes_ref, parent, edit, "Notes removed by gitoxide")?;
        }
        Ok(edit.previous)
    }

    fn root(&mut self, index: usize) -> Result<Option<gix_hash::ObjectId>, Error> {
        if let Some(root) = self.roots[index] {
            return Ok(root);
        }
        let name = self.refs[index].clone();
        let root = match self
            .repo
            .try_find_reference(name.as_ref())
            .or_raise(|| message!("Could not find notes reference {name}"))?
        {
            Some(mut reference) => Some(
                reference
                    .peel_to_tree()
                    .or_raise(|| message!("Could not peel notes reference {name} to a tree"))?
                    .id,
            ),
            None => None,
        };
        self.roots[index] = Some(root);
        Ok(root)
    }

    fn edit_root(
        &self,
        notes_ref: &gix_ref::FullNameRef,
    ) -> Result<(gix_hash::ObjectId, Option<gix_hash::ObjectId>), Error> {
        match self
            .repo
            .try_find_reference(notes_ref)
            .or_raise(|| message!("Could not find notes reference {notes_ref}"))?
        {
            Some(mut reference) => {
                let parent = reference
                    .try_id()
                    .ok_or_else(|| message!("Notes reference {notes_ref} must be direct").raise())?
                    .detach();
                let root = reference
                    .peel_to_tree()
                    .or_raise(|| message!("Could not peel notes reference {notes_ref} to a tree"))?
                    .id;
                Ok((root, Some(parent)))
            }
            None => Ok((gix_hash::ObjectId::empty_tree(self.repo.object_hash()), None)),
        }
    }

    fn commit_edit(
        &mut self,
        notes_ref: FullName,
        parent: Option<gix_hash::ObjectId>,
        edit: gix_note::Edit,
        message: &str,
    ) -> Result<(), Error> {
        let commit = self
            .repo
            .new_commit(message, edit.tree, parent)
            .or_raise(|| message!("Could not create commit for {notes_ref}"))?;
        let expected = parent.map_or(PreviousValue::MustNotExist, |id| {
            PreviousValue::MustExistAndMatch(gix_ref::Target::Object(id))
        });
        self.repo
            .reference(notes_ref.as_ref(), commit.id, expected, format!("notes: {message}"))
            .or_raise(|| message!("Could not update notes reference {notes_ref}"))?;
        for (index, reference) in self.refs.iter().enumerate() {
            if reference == &notes_ref {
                self.roots[index] = Some(Some(edit.tree));
            }
        }
        Ok(())
    }
}

fn add_refs(repo: &Repository, pattern: &BStr, out: &mut Vec<FullName>) -> Result<(), Error> {
    let parsed =
        gix_glob::parse(pattern).ok_or_else(|| message("Notes display references must not be empty").raise())?;
    if parsed.first_wildcard_pos.is_some() {
        let platform = repo
            .references()
            .or_raise(|| message!("Could not iterate notes references matching {pattern}"))?;
        let references = platform
            .all()
            .or_raise(|| message!("Could not iterate notes references matching {pattern}"))?;
        for reference in references {
            let reference = reference.map_err(|err| message!("Could not read reference: {err}").raise())?;
            if parsed.matches(reference.name().as_bstr(), gix_glob::wildmatch::Mode::empty()) {
                push_unique(out, reference.inner.name);
            }
        }
    } else {
        push_unique(
            out,
            FullName::try_from(pattern)
                .or_raise(|| message!("Notes display reference {pattern} is not fully qualified"))?,
        );
    }
    Ok(())
}

fn push_unique(out: &mut Vec<FullName>, reference: FullName) {
    if !out.contains(&reference) {
        out.push(reference);
    }
}

fn expand_notes_ref(mut name: BString) -> Result<FullName, Error> {
    if name.starts_with_str("refs/notes/") {
    } else if name.starts_with_str("notes/") {
        name.insert_str(0, "refs/");
    } else {
        name.insert_str(0, "refs/notes/");
    }
    FullName::try_from(name).or_raise(|| message("The notes reference name is invalid"))
}
