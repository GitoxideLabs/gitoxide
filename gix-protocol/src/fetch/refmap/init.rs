use bstr::{BString, ByteSlice};
use gix_transport::client::Capabilities;

use crate::{
    fetch::{
        refmap::{Mapping, Source, SpecIndex},
        RefMap,
    },
    handshake::Ref,
};

/// The error returned by [`crate::Handshake::fetch_or_extract_refmap()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("The object format {format:?} as used by the remote is unsupported")]
    UnknownObjectFormat { format: BString },
    #[error(transparent)]
    MappingValidation(#[from] gix_refspec::match_group::validate::Error),
    #[error(transparent)]
    ListRefs(#[from] crate::ls_refs::Error),
}

/// For use in [`RefMap::from_refs()`].
#[derive(Debug, Clone)]
pub struct Context {
    /// All explicit refspecs to identify references on the remote that you are interested in.
    /// Note that these are copied to [`RefMap::refspecs`] for convenience, as `RefMap::mappings` refer to them by index.
    pub fetch_refspecs: Vec<gix_refspec::RefSpec>,
    /// A list of refspecs to use as implicit refspecs which won't be saved or otherwise be part of the remote in question.
    ///
    /// This is useful for handling `remote.<name>.tagOpt` for example.
    pub extra_refspecs: Vec<gix_refspec::RefSpec>,
}

impl Context {
    pub(crate) fn aggregate_refspecs(&self) -> Vec<gix_refspec::RefSpec> {
        let mut all_refspecs = self.fetch_refspecs.clone();
        all_refspecs.extend(self.extra_refspecs.iter().cloned());
        all_refspecs
    }
}

impl RefMap {
    /// Create a ref-map from already obtained `remote_refs`. Use `context` to pass in refspecs.
    /// `capabilities` are used to determine the object format.
    pub fn from_refs(remote_refs: Vec<Ref>, capabilities: &Capabilities, context: Context) -> Result<RefMap, Error> {
        let all_refspecs = context.aggregate_refspecs();
        let Context {
            fetch_refspecs,
            extra_refspecs,
        } = context;
        let num_explicit_specs = fetch_refspecs.len();
        let group = gix_refspec::MatchGroup::from_fetch_specs(all_refspecs.iter().map(gix_refspec::RefSpec::to_ref));
        let null = gix_hash::ObjectId::null(gix_hash::Kind::Sha1); // OK to hardcode Sha1, it's not supposed to match, ever.
        let (res, fixes) = group
            .match_lhs(remote_refs.iter().map(|r| {
                let (full_ref_name, target, object) = r.unpack();
                gix_refspec::match_group::Item {
                    full_ref_name,
                    target: target.unwrap_or(&null),
                    object,
                }
            }))
            .validated()?;

        let mappings = res.mappings;
        let mappings = mappings
            .into_iter()
            .map(|m| Mapping {
                remote: m.item_index.map_or_else(
                    || {
                        Source::ObjectId(match m.lhs {
                            gix_refspec::match_group::SourceRef::ObjectId(id) => id,
                            _ => unreachable!("no item index implies having an object id"),
                        })
                    },
                    |idx| Source::Ref(remote_refs[idx].clone()),
                ),
                local: m.rhs.map(std::borrow::Cow::into_owned),
                spec_index: if m.spec_index < num_explicit_specs {
                    SpecIndex::ExplicitInRemote(m.spec_index)
                } else {
                    SpecIndex::Implicit(m.spec_index - num_explicit_specs)
                },
            })
            .collect();

        // Assume sha1 if server says nothing, otherwise configure anything beyond sha1 in the local repo configuration
        let object_hash = if let Some(object_format) = capabilities.capability("object-format").and_then(|c| c.value())
        {
            let object_format = object_format.to_str().map_err(|_| Error::UnknownObjectFormat {
                format: object_format.into(),
            })?;
            match object_format {
                "sha1" => gix_hash::Kind::Sha1,
                unknown => return Err(Error::UnknownObjectFormat { format: unknown.into() }),
            }
        } else {
            gix_hash::Kind::Sha1
        };

        Ok(Self {
            mappings,
            refspecs: fetch_refspecs,
            extra_refspecs,
            fixes,
            remote_refs,
            object_hash,
        })
    }
}
