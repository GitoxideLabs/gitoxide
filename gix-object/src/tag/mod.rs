use crate::TagRef;
use crate::parse::parse_signature;

mod decode;

///
pub mod write;

///
pub mod ref_iter;

impl<'a> TagRef<'a> {
    /// Deserialize a tag from `data`.
    pub fn from_bytes(mut data: &'a [u8], hash_kind: gix_hash::Kind) -> Result<TagRef<'a>, crate::decode::Error> {
        let input = &mut data;
        match decode::git_tag(input, hash_kind) {
            Ok(tag) => Ok(tag),
            Err(err) => Err(err),
        }
    }
    /// The object this tag points to as `Id`.
    pub fn target(&self) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_hex(self.target).expect("prior validation")
    }

    /// Return the tagger, if present.
    pub fn tagger(&self) -> Result<Option<gix_actor::SignatureRef<'a>>, crate::decode::Error> {
        Ok(self
            .tagger
            .map(parse_signature)
            .transpose()?
            .map(|signature| signature.trim()))
    }

    /// Copy all data into a fully-owned instance.
    pub fn into_owned(self) -> Result<crate::Tag, crate::decode::Error> {
        self.try_into()
    }
}
