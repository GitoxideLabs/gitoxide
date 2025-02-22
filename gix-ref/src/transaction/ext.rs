use gix_object::bstr::BString;

use crate::{
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog, Target},
    PartialNameRef,
};

/// An extension trait to perform commonly used operations on edits across different ref stores.
pub trait RefEditsExt<T>
where
    T: std::borrow::Borrow<RefEdit> + std::borrow::BorrowMut<RefEdit>,
{
    /// Return true if each ref `name` has exactly one `edit` across multiple ref edits
    fn assure_one_name_has_one_edit(&self) -> Result<(), BString>;

    /// Split all symbolic refs into updates for the symbolic ref as well as all their referents if the `deref` flag is enabled.
    ///
    /// Note no action is performed if deref isn't specified.
    fn extend_with_splits_of_symbolic_refs(
        &mut self,
        find: &mut dyn FnMut(&PartialNameRef) -> Option<Target>,
        make_entry: &mut dyn FnMut(usize, RefEdit) -> T,
    ) -> Result<(), std::io::Error>;

    /// All processing steps in one and in the correct order.
    ///
    /// Users call this to assure derefs are honored and duplicate checks are done.
    fn pre_process(
        &mut self,
        find: &mut dyn FnMut(&PartialNameRef) -> Option<Target>,
        make_entry: &mut dyn FnMut(usize, RefEdit) -> T,
    ) -> Result<(), std::io::Error> {
        self.extend_with_splits_of_symbolic_refs(find, make_entry)?;
        self.assure_one_name_has_one_edit().map_err(|name| {
            std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("A reference named '{name}' has multiple edits"),
            )
        })
    }
}

impl<E> RefEditsExt<E> for Vec<E>
where
    E: std::borrow::Borrow<RefEdit> + std::borrow::BorrowMut<RefEdit>,
{
    fn assure_one_name_has_one_edit(&self) -> Result<(), BString> {
        let mut names: Vec<_> = self.iter().map(|e| &e.borrow().name).collect();
        names.sort();
        match names.windows(2).find(|v| v[0] == v[1]) {
            Some(name) => Err(name[0].as_bstr().to_owned()),
            None => Ok(()),
        }
    }

    fn extend_with_splits_of_symbolic_refs(
        &mut self,
        find: &mut dyn FnMut(&PartialNameRef) -> Option<Target>,
        make_entry: &mut dyn FnMut(usize, RefEdit) -> E,
    ) -> Result<(), std::io::Error> {
        let mut new_edits = Vec::new();
        let mut first = 0;
        let mut round = 1;
        loop {
            for (eid, edit) in self[first..].iter_mut().enumerate().map(|(eid, v)| (eid + first, v)) {
                let edit = edit.borrow_mut();
                if !edit.deref {
                    continue;
                }

                // we can't tell what happened and we are here because it's a non-existing ref or an invalid one.
                // In any case, we don't want the following algorithms to try dereffing it and assume they deal with
                // broken refs gracefully.
                edit.deref = false;
                if let Some(Target::Symbolic(referent)) = find(edit.name.as_ref().as_partial_name()) {
                    new_edits.push(make_entry(
                        eid,
                        match &mut edit.change {
                            Change::Delete {
                                expected: previous,
                                log: mode,
                            } => {
                                let current_mode = *mode;
                                *mode = RefLog::Only;
                                RefEdit {
                                    change: Change::Delete {
                                        expected: previous.clone(),
                                        log: current_mode,
                                    },
                                    name: referent,
                                    deref: true,
                                }
                            }
                            Change::Update { log, expected, new } => {
                                let current = std::mem::replace(
                                    log,
                                    LogChange {
                                        message: log.message.clone(),
                                        mode: RefLog::Only,
                                        force_create_reflog: log.force_create_reflog,
                                    },
                                );
                                let next = std::mem::replace(expected, PreviousValue::Any);
                                RefEdit {
                                    change: Change::Update {
                                        expected: next,
                                        new: new.clone(),
                                        log: current,
                                    },
                                    name: referent,
                                    deref: true,
                                }
                            }
                        },
                    ));
                }
            }
            if new_edits.is_empty() {
                break Ok(());
            }
            if round == 5 {
                break Err(std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    format!("Could not follow all splits after {round} rounds, assuming reference cycle"),
                ));
            }
            round += 1;
            first = self.len();

            self.append(&mut new_edits);
        }
    }
}
