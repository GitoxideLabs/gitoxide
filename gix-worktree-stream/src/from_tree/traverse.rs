use std::{collections::VecDeque, io::Write};

use gix_filter::{driver::apply::MaybeDelayed, pipeline::convert::ToWorktreeOutcome};
use gix_object::{
    bstr::{BStr, BString, ByteSlice, ByteVec},
    tree, FindExt,
};
use gix_traverse::tree::{visit::Action, Visit};

use crate::{entry::Error, protocol, SharedErrorSlot};

pub struct Delegate<'a, AttributesFn, Find>
where
    Find: gix_object::Find,
{
    pub(crate) out: &'a mut gix_features::io::pipe::Writer,
    pub(crate) err: SharedErrorSlot,
    pub(crate) path_deque: VecDeque<BString>,
    pub(crate) path: BString,
    pub(crate) pipeline: gix_filter::Pipeline,
    pub(crate) attrs: gix_attributes::search::Outcome,
    pub(crate) fetch_attributes: AttributesFn,
    pub(crate) objects: Find,
    pub(crate) buf: Vec<u8>,
}

impl<AttributesFn, Find> Delegate<'_, AttributesFn, Find>
where
    Find: gix_object::Find,
    AttributesFn:
        FnMut(&BStr, gix_object::tree::EntryMode, &mut gix_attributes::search::Outcome) -> Result<(), Error> + 'static,
{
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
    /// Return the state of the `export-ignore` attribute.
    fn ignore_state(&self) -> gix_attributes::StateRef<'_> {
        self.attrs
            .iter_selected()
            .next()
            .expect("initialized with one attr")
            .assignment
            .state
    }

    fn handle_entry(&mut self, entry: &tree::EntryRef<'_>) -> Result<Action, Error> {
        if !entry.mode.is_blob_or_symlink() {
            return Ok(std::ops::ControlFlow::Continue(true));
        }
        (self.fetch_attributes)(self.path.as_ref(), entry.mode, &mut self.attrs)?;
        if self.ignore_state().is_set() {
            return Ok(std::ops::ControlFlow::Continue(true));
        }
        self.objects.find(entry.oid, &mut self.buf)?;

        self.pipeline.driver_context_mut().blob = Some(entry.oid.into());
        let converted = self.pipeline.convert_to_worktree(
            &self.buf,
            self.path.as_ref(),
            &mut |a, b| {
                (self.fetch_attributes)(a, entry.mode, b).ok();
            },
            gix_filter::driver::apply::Delay::Forbid,
        )?;

        // Our pipe writer always writes the whole amount.
        #[allow(clippy::unused_io_amount)]
        match converted {
            ToWorktreeOutcome::Unchanged(buf) | ToWorktreeOutcome::Buffer(buf) => {
                protocol::write_entry_header_and_path(
                    self.path.as_ref(),
                    entry.oid,
                    entry.mode,
                    Some(buf.len()),
                    self.out,
                )?;
                self.out.write(buf)?;
            }
            ToWorktreeOutcome::Process(MaybeDelayed::Immediate(read)) => {
                protocol::write_entry_header_and_path(self.path.as_ref(), entry.oid, entry.mode, None, self.out)?;
                protocol::write_stream(&mut self.buf, read, self.out)?;
            }
            ToWorktreeOutcome::Process(MaybeDelayed::Delayed(_)) => {
                unreachable!("we forbade it")
            }
        }
        Ok(std::ops::ControlFlow::Continue(true))
    }
}

impl<AttributesFn, Find> Visit for Delegate<'_, AttributesFn, Find>
where
    Find: gix_object::Find,
    AttributesFn:
        FnMut(&BStr, gix_object::tree::EntryMode, &mut gix_attributes::search::Outcome) -> Result<(), Error> + 'static,
{
    fn pop_back_tracked_path_and_set_current(&mut self) {
        self.path = self.path_deque.pop_back().unwrap_or_default();
    }

    fn pop_front_tracked_path_and_set_current(&mut self) {
        self.path = self
            .path_deque
            .pop_front()
            .expect("every call is matched with push_tracked_path_component");
    }

    fn push_back_tracked_path_component(&mut self, component: &BStr) {
        self.push_element(component);
        self.path_deque.push_back(self.path.clone());
    }

    fn push_path_component(&mut self, component: &BStr) {
        self.push_element(component);
    }

    fn pop_path_component(&mut self) {
        self.pop_element();
    }

    fn visit_tree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        if let Err(err) = (self.fetch_attributes)(self.path.as_ref(), entry.mode, &mut self.attrs) {
            *self.err.lock() = Some(err);
            std::ops::ControlFlow::Break(())
        } else if self.ignore_state().is_set() {
            std::ops::ControlFlow::Continue(false)
        } else {
            std::ops::ControlFlow::Continue(true)
        }
    }

    fn visit_nontree(&mut self, entry: &tree::EntryRef<'_>) -> Action {
        match self.handle_entry(entry) {
            Ok(action) => action,
            Err(err) => {
                *self.err.lock() = Some(err);
                std::ops::ControlFlow::Break(())
            }
        }
    }
}
