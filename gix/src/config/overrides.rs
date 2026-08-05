use crate::bstr::{BStr, BString, ByteSlice};

/// The error returned by [`SnapshotMut::apply_cli_overrides()`][crate::config::SnapshotMut::append_config()].
pub type Error = gix_error::Error;

pub(crate) fn append(
    config: &mut gix_config::File,
    values: impl IntoIterator<Item = impl gix_utils::AsBStr>,
    source: gix_config::Source,
    mut make_comment: impl FnMut(&BStr) -> Option<BString>,
) -> Result<(), Error> {
    let mut file = gix_config::File::new(gix_config::file::Metadata::from(source));
    for key_value in values {
        let key_value = key_value.as_bstr();
        let mut tokens = key_value.splitn(2, |b| *b == b'=').map(ByteSlice::trim);
        let key = tokens.next().expect("always one value").as_bstr();
        let value = tokens.next();
        let key = gix_config::KeyRef::parse_unvalidated(key).ok_or_else(|| {
            let input: BString = key.into();
            gix_error::Error::from_error(gix_error::message!(
                "{input:?} is not a valid configuration key. Examples are 'core.abbrev' or 'remote.origin.url'"
            ))
        })?;
        let mut section = file
            .section_mut_or_create_new(key.section_name, key.subsection_name)
            .map_err(gix_error::Error::from_error)?;
        let comment = make_comment(key_value);
        let value = value.map(ByteSlice::as_bstr);
        match comment {
            Some(comment) => section.push_with_comment(key.value_name, value, &**comment),
            None => section.push(key.value_name, value),
        }
        .map_err(gix_error::Error::from_error)?;
    }
    config.append(file).map_err(gix_error::Error::from_error)?;
    Ok(())
}
