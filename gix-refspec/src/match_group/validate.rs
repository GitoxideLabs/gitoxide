use std::collections::BTreeMap;

use bstr::BString;

use crate::{
    RefSpec,
    match_group::{Source, match_lhs},
};

/// The error returned [outcome validation](match_lhs::Outcome::validated()).
#[derive(Debug)]
pub struct Error {
    /// All issues discovered during validation.
    pub issues: Vec<Issue>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Found {} {} the refspec mapping to be used: \n\t{}",
            self.issues.len(),
            if self.issues.len() == 1 {
                "issue that prevents"
            } else {
                "issues that prevent"
            },
            self.issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("\n\t")
        )
    }
}

impl std::error::Error for Error {}

/// All possible issues found while validating matched mappings.
#[derive(Debug, PartialEq, Eq)]
pub enum Issue {
    /// Multiple sources try to write the same destination.
    ///
    /// Note that this issue doesn't take into consideration that these sources might contain the same object behind a reference.
    Conflict {
        /// The unenforced full name of the reference to be written.
        destination_full_ref_name: BString,
        /// The list of sources that map to this destination.
        sources: Vec<Source>,
        /// The list of specs that caused the mapping conflict, each matching the respective one in `sources` to allow both
        /// `sources` and `specs` to be zipped together.
        specs: Vec<BString>,
    },
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Issue::Conflict {
                destination_full_ref_name,
                sources,
                specs,
            } => {
                write!(
                    f,
                    "Conflicting destination {destination_full_ref_name:?} would be written by {}",
                    sources
                        .iter()
                        .zip(specs.iter())
                        .map(|(src, spec)| format!("{src} ({spec:?})"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

/// All possible fixes corrected while validating matched mappings.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Fix {
    /// Removed a mapping that contained a partial destination entirely.
    MappingWithPartialDestinationRemoved {
        /// The destination ref name that was ignored.
        name: BString,
        /// The spec that defined the mapping
        spec: RefSpec,
    },
}

impl match_lhs::Outcome<'_, '_> {
    /// Validate all mappings or dissolve them into an error stating the discovered issues.
    /// Return `(modified self, issues)` providing a fixed-up set of mappings in `self` with the fixed `issues`
    /// provided as part of it.
    /// Terminal issues are communicated using the [`Error`] type accordingly.
    pub fn validated(mut self) -> Result<(Self, Vec<Fix>), Error> {
        let mut sources_by_destinations = BTreeMap::new();
        for (dst, (spec_index, src)) in self
            .mappings
            .iter()
            .filter_map(|m| m.rhs.as_ref().map(|dst| (dst.as_ref(), (m.spec_index, &m.lhs))))
        {
            let sources = sources_by_destinations.entry(dst).or_insert_with(Vec::new);
            if !sources.iter().any(|(_, lhs)| lhs == &src) {
                sources.push((spec_index, src));
            }
        }
        let mut issues = Vec::new();
        for (dst, conflicting_sources) in sources_by_destinations.into_iter().filter(|(_, v)| v.len() > 1) {
            issues.push(Issue::Conflict {
                destination_full_ref_name: dst.to_owned(),
                specs: conflicting_sources
                    .iter()
                    .map(|(spec_idx, _)| self.group.specs[*spec_idx].to_bstring())
                    .collect(),
                sources: conflicting_sources
                    .into_iter()
                    .map(|(_, src)| src.clone().into_owned())
                    .collect(),
            });
        }
        if !issues.is_empty() {
            Err(Error { issues })
        } else {
            let mut fixed = Vec::new();
            let group = &self.group;
            self.mappings.retain(|m| match m.rhs.as_ref() {
                Some(dst) => {
                    if dst.starts_with(b"refs/") || dst.as_ref() == "HEAD" {
                        true
                    } else {
                        fixed.push(Fix::MappingWithPartialDestinationRemoved {
                            name: dst.as_ref().to_owned(),
                            spec: group.specs[m.spec_index].to_owned(),
                        });
                        false
                    }
                }
                None => true,
            });
            Ok((self, fixed))
        }
    }
}
