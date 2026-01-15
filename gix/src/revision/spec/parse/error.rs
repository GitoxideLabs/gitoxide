use gix_error::message;
use gix_hash::ObjectId;

use crate::{bstr, bstr::BString, ext::ObjectIdExt, Repository};

/// Additional information about candidates that caused ambiguity.
#[derive(Debug)]
pub enum CandidateInfo {
    /// An error occurred when looking up the object in the database.
    FindError {
        /// The reported error.
        source: crate::object::find::existing::Error,
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

pub(crate) fn ambiguous(candidates: Vec<ObjectId>, prefix: gix_hash::Prefix, repo: &Repository) -> gix_error::Message {
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
            .map(|oid| {
                let obj = repo.find_object(oid);
                let order = match &obj {
                    Err(_) => Order::Invalid,
                    Ok(obj) => match obj.kind {
                        gix_object::Kind::Tag => Order::Tag,
                        gix_object::Kind::Commit => Order::Commit,
                        gix_object::Kind::Tree => Order::Tree,
                        gix_object::Kind::Blob => Order::Blob,
                    },
                };
                (oid, obj, order)
            })
            .collect();
        c.sort_by(|lhs, rhs| lhs.2.cmp(&rhs.2).then_with(|| lhs.0.cmp(&rhs.0)));
        c
    };
    let info: Vec<_> = candidates
        .into_iter()
        .map(|(oid, find_result, _)| {
            let info = match find_result {
                Ok(obj) => match obj.kind {
                    gix_object::Kind::Tree | gix_object::Kind::Blob => CandidateInfo::Object { kind: obj.kind },
                    gix_object::Kind::Tag => {
                        let tag = obj.to_tag_ref();
                        CandidateInfo::Tag { name: tag.name.into() }
                    }
                    gix_object::Kind::Commit => {
                        use bstr::ByteSlice;
                        let commit = obj.to_commit_ref();
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
                },
                Err(err) => CandidateInfo::FindError { source: err },
            };
            (oid.attach(repo).shorten().unwrap_or_else(|_| oid.into()), info)
        })
        .collect();
    message!(
        "Short id {prefix} is ambiguous. Candidates are:\n{info}",
        prefix = prefix,
        info = info
            .iter()
            .map(|(oid, info)| format!("\t{oid} {info}"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}
