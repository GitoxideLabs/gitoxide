use super::Error;
use gix_error::Exn;
use gix_hash::ObjectId;

use crate::{Repository, bstr, bstr::BString, ext::ObjectIdExt};

/// Additional information about candidates that caused ambiguity.
#[derive(Debug)]
pub enum CandidateInfo {
    /// An error occurred when looking up or decoding the object.
    FindError {
        /// The reported error.
        source: crate::Error,
    },
    /// The candidate is an object of the given `kind`.
    Object {
        /// The kind of the object.
        kind: gix_object::Kind,
    },
    /// The candidate is a tag.
    Tag {
        /// The name of the tag.
        name: BString,
    },
    /// The candidate is a commit.
    Commit {
        /// The date of the commit.
        date: String,
        /// The subject line.
        title: BString,
    },
}

impl std::fmt::Display for CandidateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateInfo::FindError { source } => write!(f, "lookup error: {source}"),
            CandidateInfo::Tag { name } => write!(f, "tag {name:?}"),
            CandidateInfo::Object { kind } => std::fmt::Display::fmt(kind, f),
            CandidateInfo::Commit { date, title } => {
                write!(
                    f,
                    "commit {} {title:?}",
                    gix_date::parse_header(date)
                        .unwrap_or_default()
                        .format_or_unix(gix_date::time::format::SHORT)
                )
            }
        }
    }
}

/// Attach a parser-owned recovery signal without confusing missing objects with missing references.
pub(crate) fn with_missing_reference(err: Exn) -> Exn {
    match err.downcast_any_ref::<gix_ref::file::find::NotFound>() {
        Some(not_found) => {
            let context = Error::MissingReference {
                name: not_found.name.clone(),
            };
            err.raise(context).erased()
        }
        None => err,
    }
}

pub(crate) fn ambiguous(candidates: Vec<ObjectId>, prefix: gix_hash::Prefix, repo: &Repository) -> Error {
    Error::AmbiguousPrefix {
        prefix,
        candidates: candidate_info(candidates, repo),
    }
}

pub(crate) fn ambiguous_ref_and_object(
    candidates: Vec<ObjectId>,
    prefix: gix_hash::Prefix,
    reference: gix_ref::FullName,
    repo: &Repository,
) -> Error {
    Error::AmbiguousRefAndObject {
        prefix,
        reference,
        candidates: candidate_info(candidates, repo),
    }
}

fn candidate_info(candidates: Vec<ObjectId>, repo: &Repository) -> Vec<(gix_hash::Prefix, CandidateInfo)> {
    #[derive(PartialOrd, Ord, Eq, PartialEq, Copy, Clone)]
    enum Order {
        Tag,
        Commit,
        Tree,
        Blob,
        Invalid,
    }
    let candidates = {
        let mut c: Vec<_> = candidates
            .into_iter()
            .map(|object_id| {
                let obj = repo.find_object(object_id);
                let order = match &obj {
                    Err(_) => Order::Invalid,
                    Ok(obj) => match obj.kind {
                        gix_object::Kind::Tag => Order::Tag,
                        gix_object::Kind::Commit => Order::Commit,
                        gix_object::Kind::Tree => Order::Tree,
                        gix_object::Kind::Blob => Order::Blob,
                    },
                };
                (object_id, obj, order)
            })
            .collect();
        c.sort_by(|lhs, rhs| lhs.2.cmp(&rhs.2).then_with(|| lhs.0.cmp(&rhs.0)));
        c
    };
    candidates
        .into_iter()
        .map(|(object_id, find_result, _)| {
            let info = find_result
                .and_then(|obj| {
                    Ok(match obj.kind {
                        gix_object::Kind::Tree | gix_object::Kind::Blob => CandidateInfo::Object { kind: obj.kind },
                        gix_object::Kind::Tag => {
                            let tag = obj.try_to_tag_ref()?;
                            CandidateInfo::Tag { name: tag.name.into() }
                        }
                        gix_object::Kind::Commit => {
                            use bstr::ByteSlice;
                            let commit = obj.try_to_commit_ref()?;
                            let date = match commit.committer() {
                                Ok(signature) => signature.time.trim().to_owned(),
                                Err(_) => {
                                    let committer = commit.committer;
                                    let manually_parsed_best_effort = committer
                                        .rfind_byte(b'>')
                                        .map(|pos| committer[pos + 1..].trim().as_bstr().to_string());
                                    manually_parsed_best_effort.unwrap_or_default()
                                }
                            };
                            CandidateInfo::Commit {
                                date,
                                title: commit.message().title.trim().into(),
                            }
                        }
                    })
                })
                .unwrap_or_else(|source| CandidateInfo::FindError { source });
            (
                object_id.attach(repo).shorten().unwrap_or_else(|_| object_id.into()),
                info,
            )
        })
        .collect()
}
