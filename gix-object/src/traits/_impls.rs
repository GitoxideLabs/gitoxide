use std::{io::Read, ops::Deref, rc::Rc, sync::Arc};

use gix_hash::ObjectId;

use crate::{Kind, WriteTo};

impl<T> crate::Write for &T
where
    T: crate::Write,
{
    fn write(&self, object: &dyn WriteTo) -> Result<ObjectId, gix_error::Exn> {
        (*self).write(object)
    }

    fn write_buf(&self, object: Kind, from: &[u8]) -> Result<ObjectId, gix_error::Exn> {
        (*self).write_buf(object, from)
    }

    fn write_buf_with_known_id(&self, object: Kind, from: &[u8], id: ObjectId) -> Result<ObjectId, gix_error::Exn> {
        (*self).write_buf_with_known_id(object, from, id)
    }

    fn write_stream(&self, kind: Kind, size: u64, from: &mut dyn Read) -> Result<ObjectId, gix_error::Exn> {
        (*self).write_stream(kind, size, from)
    }

    fn write_stream_with_known_id(
        &self,
        kind: Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> Result<ObjectId, gix_error::Exn> {
        (*self).write_stream_with_known_id(kind, size, from, id)
    }
}

impl<T> crate::Write for Arc<T>
where
    T: crate::Write,
{
    fn write(&self, object: &dyn WriteTo) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write(object)
    }

    fn write_buf(&self, object: Kind, from: &[u8]) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_buf(object, from)
    }

    fn write_buf_with_known_id(&self, object: Kind, from: &[u8], id: ObjectId) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_buf_with_known_id(object, from, id)
    }

    fn write_stream(&self, kind: Kind, size: u64, from: &mut dyn Read) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_stream(kind, size, from)
    }

    fn write_stream_with_known_id(
        &self,
        kind: Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_stream_with_known_id(kind, size, from, id)
    }
}

impl<T> crate::Write for Rc<T>
where
    T: crate::Write,
{
    fn write(&self, object: &dyn WriteTo) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write(object)
    }

    fn write_buf(&self, object: Kind, from: &[u8]) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_buf(object, from)
    }

    fn write_buf_with_known_id(&self, object: Kind, from: &[u8], id: ObjectId) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_buf_with_known_id(object, from, id)
    }

    fn write_stream(&self, kind: Kind, size: u64, from: &mut dyn Read) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_stream(kind, size, from)
    }

    fn write_stream_with_known_id(
        &self,
        kind: Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> Result<ObjectId, gix_error::Exn> {
        self.deref().write_stream_with_known_id(kind, size, from, id)
    }
}

impl<T> WriteTo for &T
where
    T: WriteTo,
{
    fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
        <T as WriteTo>::write_to(self, out)
    }

    fn kind(&self) -> Kind {
        <T as WriteTo>::kind(self)
    }

    fn size(&self) -> u64 {
        <T as WriteTo>::size(self)
    }
}
