use gix_error::Result;
use gix_hash::ObjectId;

use crate::Negotiator;

pub(crate) struct Noop;

impl Negotiator for Noop {
    fn known_common(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> Result<()> {
        Ok(())
    }

    fn add_tip(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> Result<()> {
        Ok(())
    }

    fn next_have(&mut self, _graph: &mut crate::Graph<'_, '_>) -> Option<Result<ObjectId>> {
        None
    }

    fn in_common_with_remote(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> Result<bool> {
        Ok(false)
    }
}
