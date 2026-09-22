use crate::parse::Comment;
use gix_error::ExnMessageResult;

impl Comment {
    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> ExnMessageResult<Comment> {
        Ok(Comment {
            tag: self.tag,
            text: self.text.copy_to_backing_in(source, target)?,
        })
    }

    pub(crate) fn write_to_in(&self, backing: &[u8], mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(&[self.tag])?;
        out.write_all(self.text.as_slice_in(backing))
    }
}
