use gix_error::{ExnResult, ResultExt, message};

use crate::{PartialNameRef, Reference, store};

use crate::store::handle;

impl store::Handle {
    /// TODO: actually implement this with handling of the packed buffer.
    pub fn try_find<'a, Name, E>(&self, partial: Name) -> ExnResult<Option<Reference>>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Result<&'a PartialNameRef, E>: ResultExt<Success = &'a PartialNameRef>,
    {
        let _name = partial
            .try_into()
            .or_raise_erased(|| message("Invalid reference name"))?;
        match &self.state {
            handle::State::Loose { .. } => {
                todo!()
            }
        }
    }
}

impl store::Handle {
    /// Similar to [`crate::file::Store::find()`] but a non-existing ref is treated as error.
    pub fn find<'a, Name, E>(&self, _partial: Name) -> ExnResult<Reference>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Result<&'a PartialNameRef, E>: ResultExt<Success = &'a PartialNameRef>,
    {
        todo!()
    }
}
