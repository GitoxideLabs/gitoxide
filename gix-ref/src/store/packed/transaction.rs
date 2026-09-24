use std::{borrow::Cow, fmt::Formatter, io::Write};

use gix_error::{ErrorExt, ExnMessageResult, ExnResult, Message, ResultExt, message, not_found};

use crate::{
    FullNameRef, Namespace, Target, file,
    store_impl::{packed, packed::Edit},
    transaction::{Change, RefEdit},
};

pub(crate) const HEADER_LINE: &[u8] = b"# pack-refs with: peeled fully-peeled sorted \n";

/// Access and instantiation
impl packed::Transaction {
    pub(crate) fn new_from_pack_and_lock(
        buffer: Option<file::packed::SharedBufferSnapshot>,
        lock: gix_lock::File,
        precompose_unicode: bool,
        namespace: Option<Namespace>,
    ) -> Self {
        packed::Transaction {
            buffer,
            edits: None,
            lock: Some(lock),
            closed_lock: None,
            precompose_unicode,
            namespace,
        }
    }
}

impl std::fmt::Debug for packed::Transaction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("packed::Transaction")
            .field("edits", &self.edits.as_ref().map(Vec::len))
            .field("lock", &self.lock)
            .finish_non_exhaustive()
    }
}

/// Access
impl packed::Transaction {
    /// Returns our packed buffer
    pub fn buffer(&self) -> Option<&packed::Buffer> {
        self.buffer.as_ref().map(|b| &***b)
    }
}

/// Lifecycle
impl packed::Transaction {
    /// Prepare the transaction by checking all edits for applicability.
    /// Use `objects` to access objects for the purpose of peeling them - this is only used if packed-refs are involved.
    /// Object lookup failures include [metadata](gix_error::Exn::metadata()) `object_id` (hex text) and `reference`
    /// (name bytes).
    /// Missing objects are classified as not found; lookup errors retain their own classifications.
    pub fn prepare(
        mut self,
        edits: &mut dyn Iterator<Item = RefEdit>,
        objects: &dyn gix_object::Find,
    ) -> ExnResult<Self> {
        assert!(self.edits.is_none(), "BUG: cannot call prepare(…) more than once");
        let buffer = &self.buffer;
        // Remove all edits which are deletions that aren't here in the first place
        let mut edits: Vec<Edit> = edits
            .into_iter()
            .map(|mut edit| {
                use gix_object::bstr::ByteSlice;
                if self.precompose_unicode {
                    let precomposed = edit
                        .name
                        .0
                        .to_str()
                        .ok()
                        .map(|name| gix_utils::str::precompose(name.into()));
                    match precomposed {
                        None | Some(Cow::Borrowed(_)) => edit,
                        Some(Cow::Owned(precomposed)) => {
                            edit.name.0 = precomposed.into();
                            edit
                        }
                    }
                } else {
                    edit
                }
            })
            .map(|mut edit| {
                if let Some(namespace) = &self.namespace {
                    edit.name = namespace.clone().into_namespaced_name(edit.name.as_ref());
                }
                edit
            })
            .filter(|edit| {
                if let Change::Delete { .. } = edit.change {
                    buffer.as_ref().is_none_or(|b| b.find(edit.name.as_ref()).is_ok())
                } else {
                    true
                }
            })
            .map(|change| Edit {
                inner: change,
                peeled: None,
            })
            .collect();

        let mut buf = Vec::new();
        for edit in &mut edits {
            if let Change::Update {
                new: Target::Object(new),
                ..
            } = edit.inner.change
            {
                let mut next_id = new;
                edit.peeled = loop {
                    let data = objects
                        .try_find(&next_id, &mut buf)
                        .or_raise_erased(|| peel_reference_error(&next_id, edit.inner.name.as_ref()))?;
                    match data {
                        Some(gix_object::Data {
                            kind: gix_object::Kind::Tag,
                            data,
                            object_hash: hash_kind,
                        }) => {
                            next_id = gix_object::TagRefIter::from_bytes(data, hash_kind)
                                .target_id()
                                .or_raise(|| gix_error::message!("Couldn't get target object id from tag {next_id}"))
                                .or_raise_erased(|| peel_reference_error(&next_id, edit.inner.name.as_ref()))?;
                        }
                        Some(_) => {
                            break if next_id == new { None } else { Some(next_id) };
                        }
                        None => {
                            return Err(not_found("Could not peel packed reference: object could not be found")
                                .with("object_id", next_id.to_string())
                                .with("reference", edit.inner.name.as_bstr())
                                .raise_erased());
                        }
                    }
                };
            }
        }

        if edits.is_empty() {
            self.closed_lock = self
                .lock
                .take()
                .map(gix_lock::File::close)
                .transpose()
                .or_raise_erased(|| message("Could not close unused packed reference lock"))?;
        } else {
            // NOTE that we don't do any additional checks here but apply all edits unconditionally.
            // This is because this transaction system is internal and will be used correctly from the
            // loose ref store transactions, which do the necessary checking.
        }
        self.edits = Some(edits);
        Ok(self)
    }

    /// Commit the prepared transaction.
    ///
    /// Please note that actual edits invalidated existing packed buffers.
    /// Note: There is the potential to write changes into memory and return such a packed-refs buffer for reuse.
    pub fn commit(self) -> ExnResult {
        let mut edits = self.edits.expect("BUG: cannot call commit() before prepare(…)");
        if edits.is_empty() {
            return Ok(());
        }

        let mut file = self.lock.expect("a write lock for applying changes");
        let refs_sorted: Box<dyn Iterator<Item = ExnMessageResult<packed::Reference<'_>>>> = match self.buffer.as_ref()
        {
            Some(buffer) => Box::new(buffer.iter().or_erased()?),
            None => Box::new(std::iter::empty()),
        };

        let mut refs_sorted = refs_sorted.peekable();

        edits.sort_by(|l, r| l.inner.name.as_bstr().cmp(r.inner.name.as_bstr()));
        let mut peekable_sorted_edits = edits.iter().peekable();

        file.with_mut(|f| f.write_all(HEADER_LINE))
            .or_raise_erased(|| message("Could not write packed refs header"))?;

        let mut num_written_lines = 0;
        loop {
            match (refs_sorted.peek(), peekable_sorted_edits.peek()) {
                (Some(Err(_)), _) => {
                    let err = refs_sorted.next().expect("next").expect_err("err");
                    return Err(err.erased());
                }
                (None, None) => {
                    break;
                }
                (Some(Ok(_)), None) => {
                    let pref = refs_sorted.next().expect("next").expect("no err");
                    num_written_lines += 1;
                    file.with_mut(|out| write_packed_ref(out, pref))
                        .or_raise_erased(|| message("Could not write packed reference"))?;
                }
                (Some(Ok(pref)), Some(edit)) => {
                    use std::cmp::Ordering::*;
                    match pref.name.as_bstr().cmp(edit.inner.name.as_bstr()) {
                        Less => {
                            let pref = refs_sorted.next().expect("next").expect("valid");
                            num_written_lines += 1;
                            file.with_mut(|out| write_packed_ref(out, pref))
                                .or_raise_erased(|| message("Could not write packed reference"))?;
                        }
                        Greater => {
                            let edit = peekable_sorted_edits.next().expect("next");
                            file.with_mut(|out| write_edit(out, edit, &mut num_written_lines))
                                .or_raise_erased(|| message("Could not write packed reference edit"))?;
                        }
                        Equal => {
                            let _pref = refs_sorted.next().expect("next").expect("valid");
                            let edit = peekable_sorted_edits.next().expect("next");
                            file.with_mut(|out| write_edit(out, edit, &mut num_written_lines))
                                .or_raise_erased(|| message("Could not write packed reference edit"))?;
                        }
                    }
                }
                (None, Some(_)) => {
                    let edit = peekable_sorted_edits.next().expect("next");
                    file.with_mut(|out| write_edit(out, edit, &mut num_written_lines))
                        .or_raise_erased(|| message("Could not write packed reference edit"))?;
                }
            }
        }

        if num_written_lines == 0 {
            std::fs::remove_file(file.resource_path())
                .or_raise_erased(|| message("Could not delete empty packed refs"))?;
        } else {
            file.commit()
                .or_raise_erased(|| message("Could not commit packed refs"))?;
        }
        drop(refs_sorted);
        Ok(())
    }
}

/// The raised error's [metadata](gix_error::Exn::metadata()) `object_id` (hex text) and `reference` (name bytes)
/// identify the object and packed reference being peeled.
fn peel_reference_error(object_id: &gix_hash::oid, reference: &FullNameRef) -> Message {
    Message::new("Could not peel packed reference")
        .with("object_id", object_id.to_string())
        .with("reference", reference.as_bstr())
}

fn write_packed_ref(out: &mut dyn std::io::Write, pref: packed::Reference<'_>) -> std::io::Result<()> {
    write!(out, "{} ", pref.target)?;
    out.write_all(pref.name.as_bstr())?;
    out.write_all(b"\n")?;
    if let Some(object) = pref.object {
        writeln!(out, "^{object}")?;
    }
    Ok(())
}

fn write_edit(out: &mut dyn std::io::Write, edit: &Edit, lines_written: &mut i32) -> std::io::Result<()> {
    match edit.inner.change {
        Change::Delete { .. } => {}
        Change::Update {
            new: Target::Object(target_oid),
            ..
        } => {
            write!(out, "{target_oid} ")?;
            out.write_all(edit.inner.name.as_bstr())?;
            out.write_all(b"\n")?;
            if let Some(object) = edit.peeled {
                writeln!(out, "^{object}")?;
            }
            *lines_written += 1;
        }
        Change::Update {
            new: Target::Symbolic(_),
            ..
        } => unreachable!("BUG: packed refs cannot contain symbolic refs, catch that in prepare(…)"),
    }
    Ok(())
}

/// Convert this buffer to be used as the basis for a transaction.
pub(crate) fn buffer_into_transaction(
    buffer: file::packed::SharedBufferSnapshot,
    lock_mode: gix_lock::acquire::Fail,
    precompose_unicode: bool,
    namespace: Option<Namespace>,
) -> ExnResult<packed::Transaction> {
    let lock = gix_lock::File::acquire_to_update_resource(&buffer.path, lock_mode, None)?;
    Ok(packed::Transaction {
        buffer: Some(buffer),
        lock: Some(lock),
        closed_lock: None,
        edits: None,
        precompose_unicode,
        namespace,
    })
}
