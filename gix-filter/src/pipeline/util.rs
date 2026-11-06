use gix_error::{ErrorExt, Result, message};
use std::path::Path;

use bstr::{BStr, BString};
use gix_attributes::StateRef;
use smallvec::SmallVec;

use crate::{
    Driver, driver, eol,
    eol::AttributesDigest,
    pipeline::{Context, CrlfRoundTripCheck},
};

pub(crate) struct Configuration<'a> {
    pub(crate) driver: Option<&'a Driver>,
    /// What attributes say about CRLF handling.
    pub(crate) _attr_digest: Option<eol::AttributesDigest>,
    /// The final digest that includes configuration values
    pub(crate) digest: eol::AttributesDigest,
    pub(crate) encoding: Option<gix_encoding::Encoding>,
    pub(crate) check_roundtrip_encoding: bool,
    /// Whether or not to apply the `ident` filter
    pub(crate) apply_ident_filter: bool,
}

impl<'driver> Configuration<'driver> {
    pub(crate) fn at_path(
        rela_path: &BStr,
        drivers: &'driver [Driver],
        attrs: &mut gix_attributes::search::Outcome,
        attributes: &mut dyn FnMut(&BStr, &mut gix_attributes::search::Outcome),
        config: eol::Configuration,
        ignore_unknown_encoding: bool,
        roundtrip_encodings: &[BString],
    ) -> Result<Configuration<'driver>> {
        fn extract_driver<'a>(drivers: &'a [Driver], attr: &gix_attributes::search::Match<'_>) -> Option<&'a Driver> {
            if let StateRef::Value(name) = attr.assignment.state {
                drivers.iter().find(|d| d.name == name.as_bstr())
            } else {
                None
            }
        }

        fn extract_encoding(
            attr: &gix_attributes::search::Match<'_>,
            ignore_unknown: bool,
        ) -> Result<Option<gix_encoding::Encoding>> {
            match attr.assignment.state {
                StateRef::Set | StateRef::Unset => {
                    Err(gix_error::validation("Encodings must be names, like UTF-16, and cannot be booleans.").raise())
                }
                StateRef::Value(name) => match gix_encoding::Encoding::for_label(name.as_bstr()) {
                    Some(encoding) => Ok({
                        // The working-tree-encoding is the encoding we have to expect in the working tree.
                        // If the specified one is the default encoding, there is nothing to do.
                        if encoding == gix_encoding::Encoding::Utf8 {
                            None
                        } else {
                            Some(encoding)
                        }
                    }),
                    None if ignore_unknown => {
                        gix_trace::warn!(encoding = %name.as_bstr(), "Ignoring unavailable worktree encoding");
                        Ok(None)
                    }
                    None => Err(message!("The encoding named '{}' isn't available", name.as_bstr()).validation_error()),
                },
                StateRef::Unspecified => Ok(None),
            }
        }

        /// This is based on `git_path_check_crlf` in the git codebase.
        fn extract_crlf(attr: &gix_attributes::search::Match<'_>) -> Option<eol::AttributesDigest> {
            match attr.assignment.state {
                StateRef::Unspecified => None,
                StateRef::Set => Some(eol::AttributesDigest::Text),
                StateRef::Unset => Some(eol::AttributesDigest::Binary),
                StateRef::Value(v) => {
                    if v.as_bstr() == "input" {
                        Some(eol::AttributesDigest::TextInput)
                    } else if v.as_bstr() == "auto" {
                        Some(eol::AttributesDigest::TextAuto)
                    } else {
                        None
                    }
                }
            }
        }

        fn extract_eol(attr: &gix_attributes::search::Match<'_>) -> Option<eol::Mode> {
            match attr.assignment.state {
                StateRef::Unspecified | StateRef::Unset | StateRef::Set => None,
                StateRef::Value(v) => {
                    if v.as_bstr() == "lf" {
                        Some(eol::Mode::Lf)
                    } else if v.as_bstr() == "crlf" {
                        Some(eol::Mode::CrLf)
                    } else {
                        None
                    }
                }
            }
        }

        attributes(rela_path, attrs);
        let attrs: SmallVec<[_; crate::pipeline::ATTRS.len()]> = attrs.iter_selected().collect();
        let apply_ident_filter = attrs[1].assignment.state.is_set();
        let driver = extract_driver(drivers, &attrs[2]);
        let encoding = extract_encoding(&attrs[5], ignore_unknown_encoding)?;
        let check_roundtrip_encoding = match attrs[5].assignment.state {
            StateRef::Value(name) => roundtrip_encodings
                .iter()
                .any(|label| name.as_bstr().eq_ignore_ascii_case(label)),
            _ => false,
        };

        let mut digest = extract_crlf(&attrs[4]);
        if digest.is_none() {
            digest = extract_crlf(&attrs[0]);
        }

        if digest != Some(AttributesDigest::Binary) {
            let eol = extract_eol(&attrs[3]);
            digest = match digest {
                Some(AttributesDigest::TextAuto) if eol == Some(eol::Mode::Lf) => Some(AttributesDigest::TextAutoInput),
                Some(AttributesDigest::TextAuto) if eol == Some(eol::Mode::CrLf) => {
                    Some(AttributesDigest::TextAutoCrlf)
                }
                _ => match eol {
                    Some(eol::Mode::CrLf) => Some(AttributesDigest::TextCrlf),
                    Some(eol::Mode::Lf) => Some(AttributesDigest::TextInput),
                    _ => digest,
                },
            };
        }

        let attr_digest = digest;
        digest = match digest {
            None => Some(config.auto_crlf.into()),
            Some(AttributesDigest::Text) => Some(config.to_eol().into()),
            _ => digest,
        };

        Ok(Configuration {
            driver,
            _attr_digest: attr_digest,
            digest: digest.expect("always set by now"),
            encoding,
            check_roundtrip_encoding,
            apply_ident_filter,
        })
    }
}

impl Context {
    pub(crate) fn with_path<'a>(&self, rela_path: &'a BStr) -> driver::apply::Context<'a, '_> {
        driver::apply::Context {
            rela_path,
            ref_name: self.ref_name.as_ref().map(AsRef::as_ref),
            treeish: self.treeish,
            blob: self.blob,
        }
    }
}

impl CrlfRoundTripCheck {
    pub(crate) fn to_eol_roundtrip_check(self, rela_path: &Path) -> Option<eol::convert_to_git::RoundTripCheck<'_>> {
        match self {
            CrlfRoundTripCheck::Fail => Some(eol::convert_to_git::RoundTripCheck::Fail { rela_path }),
            CrlfRoundTripCheck::Warn => Some(eol::convert_to_git::RoundTripCheck::Warn { rela_path }),
            CrlfRoundTripCheck::Skip => None,
        }
    }
}
