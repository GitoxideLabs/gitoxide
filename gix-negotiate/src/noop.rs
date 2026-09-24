use gix_error::ExnResult;
use gix_hash::ObjectId;

use crate::Negotiator;

pub(crate) struct Noop;

impl Negotiator for Noop {
    fn known_common(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> ExnResult {
        Ok(())
    }

    fn add_tip(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> ExnResult {
        Ok(())
    }

    fn next_have(&mut self, _graph: &mut crate::Graph<'_, '_>) -> Option<ExnResult<ObjectId>> {
        None
    }

    fn in_common_with_remote(&mut self, _id: ObjectId, _graph: &mut crate::Graph<'_, '_>) -> ExnResult<bool> {
        Ok(false)
    }
}
