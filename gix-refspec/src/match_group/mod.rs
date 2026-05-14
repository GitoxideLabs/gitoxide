use std::collections::BTreeSet;

use crate::{MatchGroup, RefSpecRef, parse::Operation, types::Mode};

pub(crate) mod types;
pub use types::{Item, Mapping, Source, SourceRef, match_lhs, match_rhs};

///
pub mod validate;

/// Initialization
impl<'a> MatchGroup<'a> {
    /// Take all the fetch ref specs from `specs` get a match group ready.
    pub fn from_fetch_specs(specs: impl IntoIterator<Item = RefSpecRef<'a>>) -> Self {
        MatchGroup {
            specs: specs.into_iter().filter(|s| s.op == Operation::Fetch).collect(),
        }
    }

    /// Take all the push ref specs from `specs` get a match group ready.
    pub fn from_push_specs(specs: impl IntoIterator<Item = RefSpecRef<'a>>) -> Self {
        MatchGroup {
            specs: specs.into_iter().filter(|s| s.op == Operation::Push).collect(),
        }
    }
}

/// Matching
impl<'spec> MatchGroup<'spec> {
    /// Match all `items` against all *fetch* specs present in this group, returning deduplicated mappings from source to destination.
    /// `items` are expected to be references on the remote, which will be matched and mapped to obtain their local counterparts,
    /// i.e. *left side of refspecs is mapped to their right side*.
    /// *Note that this method is correct only for fetch-specs*, even though it also *works for push-specs*.
    ///
    /// Object names are never mapped and always returned as match.
    ///
    /// Note that negative matches are not part of the return value, so they are not observable but will be used to remove mappings.
    // TODO: figure out how to deal with push-specs, probably when push is being implemented.
    pub fn match_lhs<'item>(
        self,
        mut items: impl Iterator<Item = Item<'item>> + Clone,
    ) -> match_lhs::Outcome<'spec, 'item> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::default();
        let mut push_unique = |mapping| {
            if seen.insert(calculate_hash(&mapping)) {
                out.push(mapping);
            }
        };
        let mut matchers: Vec<Option<Matcher<'_>>> = self
            .specs
            .iter()
            .copied()
            .map(Matcher::from)
            .enumerate()
            .map(|(idx, m)| match m.lhs {
                Some(Needle::Object(id)) => {
                    push_unique(Mapping {
                        item_index: None,
                        lhs: SourceRef::ObjectId(id),
                        rhs: m.rhs.map(Needle::to_bstr),
                        spec_index: idx,
                    });
                    None
                }
                _ => Some(m),
            })
            .collect();

        let mut has_negation = false;
        for (spec_index, (spec, matcher)) in self.specs.iter().zip(matchers.iter_mut()).enumerate() {
            if spec.mode == Mode::Negative {
                has_negation = true;
                continue;
            }
            for (item_index, item) in items.clone().enumerate() {
                let Some(matcher) = matcher else { continue };
                let (matched, rhs) = matcher.matches_lhs(item);
                if matched {
                    push_unique(Mapping {
                        item_index: Some(item_index),
                        lhs: SourceRef::FullName(item.full_ref_name.into()),
                        rhs,
                        spec_index,
                    });
                }
            }
        }

        if let Some(hash_kind) = has_negation.then(|| items.next().map(|i| i.target.kind())).flatten() {
            let null_id = hash_kind.null();
            for matcher in matchers
                .into_iter()
                .zip(self.specs.iter())
                .filter_map(|(m, spec)| m.and_then(|m| (spec.mode == Mode::Negative).then_some(m)))
            {
                out.retain(|m| match &m.lhs {
                    SourceRef::ObjectId(_) => true,
                    SourceRef::FullName(name) => {
                        !matcher
                            .matches_lhs(Item {
                                full_ref_name: name.as_ref(),
                                target: &null_id,
                                object: None,
                            })
                            .0
                    }
                });
            }
        }
        match_lhs::Outcome {
            group: self,
            mappings: out,
        }
    }

    /// Match all `items` against all *fetch* specs present in this group, returning deduplicated mappings from destination to source.
    /// `items` are expected to be tracking references in the local clone, which will be matched and reverse-mapped to obtain their remote counterparts,
    /// i.e. *right side of refspecs is mapped to their left side*.
    /// *Note that this method is correct only for fetch-specs*, even though it also *works for push-specs*.
    ///
    /// Note that negative matches are not part of the return value, so they are not observable but will be used to remove mappings.
    // Reverse-mapping is implemented here: https://github.com/git/git/blob/76cf4f61c87855ebf0784b88aaf737d6b09f504b/branch.c#L252
    pub fn match_rhs<'item>(
        self,
        mut items: impl Iterator<Item = Item<'item>> + Clone,
    ) -> match_rhs::Outcome<'spec, 'item> {
        let mut out = Vec::<Mapping<'spec, 'item>>::new();
        let mut seen = BTreeSet::default();
        let mut push_unique = |mapping| {
            if seen.insert(calculate_hash(&mapping)) {
                out.push(mapping);
            }
        };
        let mut matchers: Vec<Matcher<'_>> = self.specs.iter().copied().map(Matcher::from).collect();

        let mut has_negation = false;
        for (spec_index, (spec, matcher)) in self.specs.iter().zip(matchers.iter_mut()).enumerate() {
            if spec.mode == Mode::Negative {
                has_negation = true;
                continue;
            }
            for (item_index, item) in items.clone().enumerate() {
                let (matched, lhs) = matcher.matches_rhs(item);
                if let Some(lhs) = lhs.filter(|_| matched) {
                    push_unique(Mapping {
                        item_index: Some(item_index),
                        lhs: SourceRef::FullName(lhs),
                        rhs: Some(item.full_ref_name.into()),
                        spec_index,
                    });
                }
            }
        }

        if let Some(hash_kind) = has_negation.then(|| items.next().map(|i| i.target.kind())).flatten() {
            let null_id = hash_kind.null();
            for matcher in matchers
                .into_iter()
                .zip(self.specs.iter())
                .filter_map(|(m, spec)| (spec.mode == Mode::Negative).then_some(m))
            {
                out.retain(|m| match &m.lhs {
                    SourceRef::ObjectId(_) => true,
                    SourceRef::FullName(name) => {
                        !matcher
                            .matches_rhs(Item {
                                full_ref_name: name.as_ref(),
                                target: &null_id,
                                object: None,
                            })
                            .0
                    }
                });
            }
        }
        match_rhs::Outcome {
            group: self,
            mappings: out,
        }
    }
}

fn calculate_hash<T: std::hash::Hash>(t: &T) -> u64 {
    use std::hash::Hasher;
    let mut s = std::collections::hash_map::DefaultHasher::new();
    t.hash(&mut s);
    s.finish()
}

mod util;
use util::{Matcher, Needle};
