use winnow::prelude::*;

use crate::TagRef;

mod decode;

///
pub mod write;

///
pub mod ref_iter;

impl<'a> TagRef<'a> {
    /// Deserialize a tag from `data`.
    pub fn from_bytes(mut data: &'a [u8]) -> Result<TagRef<'a>, crate::decode::Error> {
        let input = &mut data;
        match decode::git_tag.parse_next(input) {
            Ok(tag) => Ok(tag),
            Err(err) => Err(crate::decode::Error::with_err(err, input)),
        }
    }
    /// The object this tag points to as `Id`.
    pub fn target(&self) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_hex(self.target).expect("prior validation")
    }

    /// Return the tagger, if present.
    pub fn tagger(&self) -> Result<Option<gix_actor::SignatureRef<'a>>, crate::decode::Error> {
        self.tagger
            .map(|raw| {
                gix_actor::SignatureRef::from_bytes::<crate::decode::ParseError>(raw.as_ref())
                    .map_err(|err| crate::decode::Error::with_err(err, raw.as_ref()))
                    .map(|signature| signature.trim())
            })
            .transpose()
    }

    /// Copy all data into a fully-owned instance.
    pub fn into_owned(self) -> Result<crate::Tag, crate::decode::Error> {
        crate::Tag::try_from(self)
    }
}
