use crate::fetch::response::ShallowUpdate;
use crate::handshake::{refs::parse::Error, Ref};
use bstr::{BStr, BString, ByteSlice};

impl From<InternalRef> for Ref {
    fn from(v: InternalRef) -> Self {
        match v {
            InternalRef::Symbolic {
                path,
                target: Some(target),
                tag,
                object,
            } => Ref::Symbolic {
                full_ref_name: path,
                target,
                tag,
                object,
            },
            InternalRef::Symbolic {
                path,
                target: None,
                tag: None,
                object,
            } => Ref::Direct {
                full_ref_name: path,
                object,
            },
            InternalRef::Symbolic {
                path,
                target: None,
                tag: Some(tag),
                object,
            } => Ref::Peeled {
                full_ref_name: path,
                tag,
                object,
            },
            InternalRef::Peeled { path, tag, object } => Ref::Peeled {
                full_ref_name: path,
                tag,
                object,
            },
            InternalRef::Direct { path, object } => Ref::Direct {
                full_ref_name: path,
                object,
            },
            InternalRef::SymbolicForLookup { .. } => {
                unreachable!("this case should have been removed during processing")
            }
        }
    }
}

#[cfg_attr(test, derive(PartialEq, Eq, Debug, Clone))]
pub(crate) enum InternalRef {
    /// A ref pointing to a `tag` object, which in turns points to an `object`, usually a commit
    Peeled {
        path: BString,
        tag: gix_hash::ObjectId,
        object: gix_hash::ObjectId,
    },
    /// A ref pointing to a commit object
    Direct { path: BString, object: gix_hash::ObjectId },
    /// A symbolic ref pointing to `target` ref, which in turn points to an `object`
    Symbolic {
        path: BString,
        /// It is `None` if the target is unreachable as it points to another namespace than the one is currently set
        /// on the server (i.e. based on the repository at hand or the user performing the operation).
        ///
        /// The latter is more of an edge case, please [this issue][#205] for details.
        target: Option<BString>,
        tag: Option<gix_hash::ObjectId>,
        object: gix_hash::ObjectId,
    },
    /// extracted from V1 capabilities, which contain some important symbolic refs along with their targets
    /// These don't contain the Id
    SymbolicForLookup { path: BString, target: Option<BString> },
}

impl InternalRef {
    fn unpack_direct(self) -> Option<(BString, gix_hash::ObjectId)> {
        match self {
            InternalRef::Direct { path, object } => Some((path, object)),
            _ => None,
        }
    }
    fn lookup_symbol_has_path(&self, predicate_path: &BStr) -> bool {
        matches!(self, InternalRef::SymbolicForLookup { path, .. } if path == predicate_path)
    }
}

pub(crate) fn from_capabilities<'a>(
    capabilities: impl Iterator<Item = gix_transport::client::capabilities::Capability<'a>>,
) -> Result<Vec<InternalRef>, Error> {
    let mut out_refs = Vec::new();
    let symref_values = capabilities.filter_map(|c| {
        if c.name() == b"symref".as_bstr() {
            c.value().map(ToOwned::to_owned)
        } else {
            None
        }
    });
    for symref in symref_values {
        let (left, right) = symref.split_at(symref.find_byte(b':').ok_or_else(|| Error::MalformedSymref {
            symref: symref.to_owned(),
        })?);
        if left.is_empty() || right.is_empty() {
            return Err(Error::MalformedSymref {
                symref: symref.to_owned(),
            });
        }
        out_refs.push(InternalRef::SymbolicForLookup {
            path: left.into(),
            target: match &right[1..] {
                b"(null)" => None,
                name => Some(name.into()),
            },
        });
    }
    Ok(out_refs)
}

pub(in crate::handshake::refs) fn parse_v1(
    num_initial_out_refs: usize,
    out_refs: &mut Vec<InternalRef>,
    out_shallow: &mut Vec<ShallowUpdate>,
    line: &BStr,
) -> Result<(), Error> {
    let trimmed = line.trim_end();
    let (hex_hash, path) = trimmed.split_at(
        trimmed
            .find(b" ")
            .ok_or_else(|| Error::MalformedV1RefLine(trimmed.to_owned().into()))?,
    );
    let path = &path[1..];
    if path.is_empty() {
        return Err(Error::MalformedV1RefLine(trimmed.to_owned().into()));
    }
    match path.strip_suffix(b"^{}") {
        Some(stripped) => {
            if hex_hash.iter().all(|b| *b == b'0') && stripped == b"capabilities" {
                // this is a special dummy-ref just for the sake of getting capabilities across in a repo that is empty.
                return Ok(());
            }
            let (previous_path, tag) =
                out_refs
                    .pop()
                    .and_then(InternalRef::unpack_direct)
                    .ok_or(Error::InvariantViolation {
                        message: "Expecting peeled refs to be preceded by direct refs",
                    })?;
            if previous_path != stripped {
                return Err(Error::InvariantViolation {
                    message: "Expecting peeled refs to have the same base path as the previous, unpeeled one",
                });
            }
            out_refs.push(InternalRef::Peeled {
                path: previous_path,
                tag,
                object: gix_hash::ObjectId::from_hex(hex_hash.as_bytes())?,
            });
        }
        None => {
            let object = match gix_hash::ObjectId::from_hex(hex_hash.as_bytes()) {
                Ok(id) => id,
                Err(_) if hex_hash.as_bstr() == "shallow" => {
                    let id = gix_hash::ObjectId::from_hex(path)?;
                    out_shallow.push(ShallowUpdate::Shallow(id));
                    return Ok(());
                }
                Err(err) => return Err(err.into()),
            };
            match out_refs
                .iter()
                .take(num_initial_out_refs)
                .position(|r| r.lookup_symbol_has_path(path.into()))
            {
                Some(position) => match out_refs.swap_remove(position) {
                    InternalRef::SymbolicForLookup { path: _, target } => out_refs.push(InternalRef::Symbolic {
                        path: path.into(),
                        tag: None, // TODO: figure out how annotated tags work here.
                        object,
                        target,
                    }),
                    _ => unreachable!("Bug in lookup_symbol_has_path - must return lookup symbols"),
                },
                None => out_refs.push(InternalRef::Direct {
                    object,
                    path: path.into(),
                }),
            }
        }
    }
    Ok(())
}

pub(in crate::handshake::refs) fn parse_v2(line: &BStr) -> Result<Ref, Error> {
    let trimmed = line.trim_end();
    let mut tokens = trimmed.splitn(4, |b| *b == b' ');
    match (tokens.next(), tokens.next()) {
        (Some(hex_hash), Some(path)) => {
            let id = if hex_hash == b"unborn" {
                None
            } else {
                Some(gix_hash::ObjectId::from_hex(hex_hash.as_bytes())?)
            };
            if path.is_empty() {
                return Err(Error::MalformedV2RefLine(trimmed.to_owned().into()));
            }
            let mut symref_target = None;
            let mut peeled = None;
            for attribute in tokens.by_ref().take(2) {
                let mut tokens = attribute.splitn(2, |b| *b == b':');
                match (tokens.next(), tokens.next()) {
                    (Some(attribute), Some(value)) => {
                        if value.is_empty() {
                            return Err(Error::MalformedV2RefLine(trimmed.to_owned().into()));
                        }
                        match attribute {
                            b"peeled" => {
                                peeled = Some(gix_hash::ObjectId::from_hex(value.as_bytes())?);
                            }
                            b"symref-target" => {
                                symref_target = Some(value);
                            }
                            _ => {
                                return Err(Error::UnknownAttribute {
                                    attribute: attribute.to_owned().into(),
                                    line: trimmed.to_owned().into(),
                                })
                            }
                        }
                    }
                    _ => return Err(Error::MalformedV2RefLine(trimmed.to_owned().into())),
                }
            }
            if tokens.next().is_some() {
                return Err(Error::MalformedV2RefLine(trimmed.to_owned().into()));
            }
            Ok(match (symref_target, peeled) {
                (Some(target_name), peeled) => match target_name {
                    b"(null)" => match peeled {
                        None => Ref::Direct {
                            full_ref_name: path.into(),
                            object: id.ok_or(Error::InvariantViolation {
                                message: "got 'unborn' while (null) was a symref target",
                            })?,
                        },
                        Some(peeled) => Ref::Peeled {
                            full_ref_name: path.into(),
                            object: peeled,
                            tag: id.ok_or(Error::InvariantViolation {
                                message: "got 'unborn' while (null) was a symref target",
                            })?,
                        },
                    },
                    name => match id {
                        Some(id) => Ref::Symbolic {
                            full_ref_name: path.into(),
                            tag: peeled.map(|_| id),
                            object: peeled.unwrap_or(id),
                            target: name.into(),
                        },
                        None => Ref::Unborn {
                            full_ref_name: path.into(),
                            target: name.into(),
                        },
                    },
                },
                (None, Some(peeled)) => Ref::Peeled {
                    full_ref_name: path.into(),
                    object: peeled,
                    tag: id.ok_or(Error::InvariantViolation {
                        message: "got 'unborn' as tag target",
                    })?,
                },
                (None, None) => Ref::Direct {
                    object: id.ok_or(Error::InvariantViolation {
                        message: "got 'unborn' as object name of direct reference",
                    })?,
                    full_ref_name: path.into(),
                },
            })
        }
        _ => Err(Error::MalformedV2RefLine(trimmed.to_owned().into())),
    }
}

#[cfg(test)]
mod tests {
    use gix_transport::client;

    use crate::handshake::{refs, refs::shared::InternalRef};

    #[test]
    fn extract_symbolic_references_from_capabilities() -> Result<(), client::Error> {
        let caps = client::Capabilities::from_bytes(
            b"\0unrelated symref=HEAD:refs/heads/main symref=ANOTHER:refs/heads/foo symref=MISSING_NAMESPACE_TARGET:(null) agent=git/2.28.0",
        )?
            .0;
        let out = refs::shared::from_capabilities(caps.iter()).expect("a working example");

        assert_eq!(
            out,
            vec![
                InternalRef::SymbolicForLookup {
                    path: "HEAD".into(),
                    target: Some("refs/heads/main".into())
                },
                InternalRef::SymbolicForLookup {
                    path: "ANOTHER".into(),
                    target: Some("refs/heads/foo".into())
                },
                InternalRef::SymbolicForLookup {
                    path: "MISSING_NAMESPACE_TARGET".into(),
                    target: None
                }
            ]
        );
        Ok(())
    }
}
