use bstr::BStr;
use gix_object::TreeRefIter;

use super::{Action, ChangeRef, Error, Options};
use crate::{rewrites, rewrites::tracker};

/// Call `for_each` repeatedly with all changes that are needed to convert `lhs` to `rhs`.
/// Provide a `resource_cache` to speed up obtaining blobs for similarity checks.
/// `tree_diff_state` can be used to re-use tree-diff memory between calls.
/// `objects` are used to lookup trees while performing the diff.
/// Use `options` to further configure how the rename tracking is performed.
///
/// Reusing `resource_cache` between multiple invocations saves a lot of IOps as it avoids the creation
/// of a temporary `resource_cache` that triggers reading or checking for multiple gitattribute files.
/// Note that it's recommended to call [`clear_resource_cache()`](`crate::blob::Platform::clear_resource_cache()`)
/// between the calls to avoid runaway memory usage, as the cache isn't limited.
///
/// Note that to do rename tracking like `git` does, one has to configure the `resource_cache` with
/// a conversion pipeline that uses [`crate::blob::pipeline::Mode::ToGit`].
///
/// `rhs` or `lhs` can be empty to indicate deletion or addition of an entire tree.
///
/// Note that the rewrite outcome is only available if [rewrite-tracking was enabled](Options::rewrites).
pub fn diff<E>(
    lhs: TreeRefIter<'_>,
    rhs: TreeRefIter<'_>,
    resource_cache: &mut crate::blob::Platform,
    tree_diff_state: &mut crate::tree::State,
    objects: &impl gix_object::FindObjectOrHeader,
    for_each: impl FnMut(ChangeRef<'_>) -> Result<Action, E>,
    options: Options,
) -> Result<Option<rewrites::Outcome>, Error>
where
    E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
{
    let mut delegate = Delegate {
        src_tree: lhs,
        recorder: crate::tree::Recorder::default().track_location(options.location),
        visit: for_each,
        location: options.location,
        objects,
        tracked: options.rewrites.map(rewrites::Tracker::new),
        err: None,
    };
    match crate::tree(lhs, rhs, tree_diff_state, objects, &mut delegate) {
        Ok(()) => {
            let outcome = delegate.process_tracked_changes(resource_cache)?;
            match delegate.err {
                Some(err) => Err(Error::ForEach(err.into())),
                None => Ok(outcome),
            }
        }
        Err(crate::tree::Error::Cancelled) => delegate
            .err
            .map_or(Err(Error::Diff(crate::tree::Error::Cancelled)), |err| {
                Err(Error::ForEach(err.into()))
            }),
        Err(err) => Err(err.into()),
    }
}

struct Delegate<'a, 'old, VisitFn, E, Objects> {
    src_tree: TreeRefIter<'old>,
    recorder: crate::tree::Recorder,
    objects: &'a Objects,
    visit: VisitFn,
    tracked: Option<rewrites::Tracker<crate::tree::visit::Change>>,
    location: Option<crate::tree::recorder::Location>,
    err: Option<E>,
}

impl<VisitFn, E, Objects> Delegate<'_, '_, VisitFn, E, Objects>
where
    Objects: gix_object::FindObjectOrHeader,
    VisitFn: for<'delegate> FnMut(ChangeRef<'_>) -> Result<Action, E>,
    E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
{
    /// Call `visit` on an attached version of `change`.
    fn emit_change(
        change: crate::tree::visit::Change,
        location: &BStr,
        visit: &mut VisitFn,
        stored_err: &mut Option<E>,
    ) -> crate::tree::visit::Action {
        use crate::tree::visit::Change::*;
        let change = match change {
            Addition {
                entry_mode,
                oid,
                relation,
            } => ChangeRef::Addition {
                location,
                relation,
                entry_mode,
                id: oid,
            },
            Deletion {
                entry_mode,
                oid,
                relation,
            } => ChangeRef::Deletion {
                entry_mode,
                location,
                relation,
                id: oid,
            },
            Modification {
                previous_entry_mode,
                previous_oid,
                entry_mode,
                oid,
            } => ChangeRef::Modification {
                location,
                previous_entry_mode,
                entry_mode,
                previous_id: previous_oid,
                id: oid,
            },
        };
        match visit(change) {
            Ok(std::ops::ControlFlow::Break(())) => std::ops::ControlFlow::Break(()),
            Ok(std::ops::ControlFlow::Continue(())) => std::ops::ControlFlow::Continue(()),
            Err(err) => {
                *stored_err = Some(err);
                std::ops::ControlFlow::Break(())
            }
        }
    }

    fn process_tracked_changes(
        &mut self,
        diff_cache: &mut crate::blob::Platform,
    ) -> Result<Option<rewrites::Outcome>, Error> {
        use crate::rewrites::tracker::Change as _;
        let tracked = match self.tracked.as_mut() {
            Some(t) => t,
            None => return Ok(None),
        };

        let outcome = tracked.emit(
            |dest, source| match source {
                Some(source) => {
                    let (oid, mode) = dest.change.oid_and_entry_mode();
                    let change = ChangeRef::Rewrite {
                        source_location: source.location,
                        source_entry_mode: source.entry_mode,
                        source_id: source.id,
                        source_relation: source.change.relation(),
                        entry_mode: mode,
                        id: oid.to_owned(),
                        relation: dest.change.relation(),
                        diff: source.diff,
                        location: dest.location,
                        copy: match source.kind {
                            tracker::visit::SourceKind::Rename => false,
                            tracker::visit::SourceKind::Copy => true,
                        },
                    };
                    match (self.visit)(change) {
                        Ok(std::ops::ControlFlow::Break(())) => std::ops::ControlFlow::Break(()),
                        Ok(std::ops::ControlFlow::Continue(())) => std::ops::ControlFlow::Continue(()),
                        Err(err) => {
                            self.err = Some(err);
                            std::ops::ControlFlow::Break(())
                        }
                    }
                }
                None => Self::emit_change(dest.change, dest.location, &mut self.visit, &mut self.err),
            },
            diff_cache,
            self.objects,
            |push| {
                let mut delegate = tree_to_changes::Delegate::new(push, self.location);
                let state = gix_traverse::tree::breadthfirst::State::default();
                gix_traverse::tree::breadthfirst(self.src_tree, state, self.objects, &mut delegate)
            },
        )?;
        Ok(Some(outcome))
    }
}

impl<VisitFn, E, Objects> crate::tree::Visit for Delegate<'_, '_, VisitFn, E, Objects>
where
    Objects: gix_object::FindObjectOrHeader,
    VisitFn: for<'delegate> FnMut(ChangeRef<'_>) -> Result<Action, E>,
    E: Into<Box<dyn std::error::Error + Sync + Send + 'static>>,
{
    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.recorder.pop_front_tracked_path_and_set_current();
    }

    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        self.recorder.push_back_tracked_path_component(component);
    }

    fn push_path_component(&mut self, component: &BStr) {
        self.recorder.push_path_component(component);
    }

    fn pop_path_component(&mut self) {
        self.recorder.pop_path_component();
    }

    fn visit(&mut self, change: crate::tree::visit::Change) -> crate::tree::visit::Action {
        match self.tracked.as_mut() {
            Some(tracked) => tracked
                .try_push_change(change, self.recorder.path())
                .map_or(std::ops::ControlFlow::Continue(()), |change| {
                    Self::emit_change(change, self.recorder.path(), &mut self.visit, &mut self.err)
                }),
            None => Self::emit_change(change, self.recorder.path(), &mut self.visit, &mut self.err),
        }
    }
}

mod tree_to_changes {
    use bstr::BStr;
    use gix_object::tree::EntryRef;

    use crate::tree::visit::Change;

    pub struct Delegate<'a> {
        push: &'a mut dyn FnMut(Change, &BStr),
        recorder: gix_traverse::tree::Recorder,
    }

    impl<'a> Delegate<'a> {
        pub fn new(push: &'a mut dyn FnMut(Change, &BStr), location: Option<crate::tree::recorder::Location>) -> Self {
            let location = location.map(|t| match t {
                crate::tree::recorder::Location::FileName => gix_traverse::tree::recorder::Location::FileName,
                crate::tree::recorder::Location::Path => gix_traverse::tree::recorder::Location::Path,
            });
            Self {
                push,
                recorder: gix_traverse::tree::Recorder::default().track_location(location),
            }
        }
    }

    impl gix_traverse::tree::Visit for Delegate<'_> {
        fn pop_back_tracked_path_and_set_current(&mut self) {
            self.recorder.pop_back_tracked_path_and_set_current();
        }

        fn pop_front_tracked_path_and_set_current(&mut self) {
            self.recorder.pop_front_tracked_path_and_set_current();
        }

        fn push_back_tracked_path_component(&mut self, component: &BStr) {
            self.recorder.push_back_tracked_path_component(component);
        }

        fn push_path_component(&mut self, component: &BStr) {
            self.recorder.push_path_component(component);
        }

        fn pop_path_component(&mut self) {
            self.recorder.pop_path_component();
        }

        fn visit_tree(&mut self, _entry: &EntryRef<'_>) -> gix_traverse::tree::visit::Action {
            std::ops::ControlFlow::Continue(true)
        }

        fn visit_nontree(&mut self, entry: &EntryRef<'_>) -> gix_traverse::tree::visit::Action {
            if entry.mode.is_blob() {
                (self.push)(
                    Change::Modification {
                        previous_entry_mode: entry.mode,
                        previous_oid: gix_hash::ObjectId::null(entry.oid.kind()),
                        entry_mode: entry.mode,
                        oid: entry.oid.to_owned(),
                    },
                    self.recorder.path(),
                );
            }
            std::ops::ControlFlow::Continue(true)
        }
    }
}
