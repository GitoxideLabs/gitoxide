use bstr::{BString, ByteSlice};
use gix_glob::Pattern;
use kstring::{KString, KStringRef};

use crate::{
    search::{
        refmap::RefMapKey, Assignments, AttributeId, Attributes, MatchKind, Metadata, MetadataCollection, Outcome,
        TrackedAssignment, Value,
    },
    AssignmentRef, NameRef, StateRef,
};

/// Initialization
impl Outcome {
    /// Initialize this instance to collect outcomes for all names in `collection`, which represents all possible attributes
    /// or macros we may visit, and [`reset`][Self::reset()] it unconditionally.
    ///
    /// This must be called after each time `collection` changes.
    pub fn initialize(&mut self, collection: &MetadataCollection) {
        if self.matches_by_id.len() != collection.name_to_meta.len() {
            let global_num_attrs = collection.name_to_meta.len();

            self.matches_by_id.resize(global_num_attrs, Default::default());

            // NOTE: This works only under the assumption that macros remain defined.
            for (order, macro_attributes) in collection.iter().filter_map(|(_, meta)| {
                (!meta.macro_attributes.is_empty()).then_some((meta.id.0, &meta.macro_attributes))
            }) {
                self.matches_by_id[order].macro_attributes.clone_from(macro_attributes);
            }

            for (name, id) in self.selected.iter_mut().filter(|(_, id)| id.is_none()) {
                *id = collection.name_to_meta.get(name.as_str()).map(|meta| meta.id);
            }
        }
        self.reset();
    }

    /// Like [`initialize()`][Self::initialize()], but limits the set of attributes to look for and fill in
    /// to `attribute_names`.
    /// Users of this instance should prefer to limit their search as this would allow it to finish earlier.
    ///
    /// Note that `attribute_names` aren't validated to be valid names here, as invalid names definitely will always be unspecified.
    pub fn initialize_with_selection<'a>(
        &mut self,
        collection: &MetadataCollection,
        attribute_names: impl IntoIterator<Item = impl Into<KStringRef<'a>>>,
    ) {
        self.initialize_with_selection_inner(collection, &mut attribute_names.into_iter().map(Into::into));
    }

    fn initialize_with_selection_inner(
        &mut self,
        collection: &MetadataCollection,
        attribute_names: &mut dyn Iterator<Item = KStringRef<'_>>,
    ) {
        self.selected.clear();
        self.selected.extend(attribute_names.map(|name| {
            (
                name.to_owned(),
                collection.name_to_meta.get(name.as_str()).map(|meta| meta.id),
            )
        }));

        self.initialize(collection);
        self.reset_remaining();
    }

    /// Prepare for a new search over the known set of attributes by resetting our state.
    pub fn reset(&mut self) {
        self.matches_by_id.iter_mut().for_each(|item| item.r#match = None);
        self.attrs_stack.clear();
        self.reset_remaining();
    }

    fn reset_remaining(&mut self) {
        self.remaining = Some(if self.selected.is_empty() {
            self.matches_by_id.len()
        } else {
            self.selected.iter().filter(|(_name, id)| id.is_some()).count()
        });
    }

    /// A performance optimization which allows results from this instance to be efficiently copied over to `dest`.
    /// For this to work, `collection` must be the one used to initialize our state, and `dest` should not have been initialized
    /// with any meaningful collection initially, i.e. be empty the first time this method is called.
    ///
    /// Note that it's safe to call it multiple times, so that it can be called after this instance was used to store a search result.
    pub fn copy_into(&self, collection: &MetadataCollection, dest: &mut Self) {
        dest.initialize(collection);
        dest.matches_by_id.clone_from(&self.matches_by_id);
        if dest.patterns.len() != self.patterns.len() {
            dest.patterns = self.patterns.clone();
        }
        if dest.assignments.len() != self.assignments.len() {
            dest.assignments = self.assignments.clone();
        }
        if dest.source_paths.len() != self.source_paths.len() {
            dest.source_paths = self.source_paths.clone();
        }
        dest.remaining = self.remaining;
    }
}

/// Access
impl Outcome {
    /// Return an iterator over all filled attributes we were initialized with.
    ///
    /// ### Note
    ///
    /// If [`initialize_with_selection`][Self::initialize_with_selection()] was used,
    /// use [`iter_selected()`][Self::iter_selected()] instead.
    ///
    /// ### Deviation
    ///
    /// It's possible that the order in which the attribute are returned (if not limited to a set of attributes) isn't exactly
    /// the same as what `git` provides.
    /// Ours is in order of declaration, whereas `git` seems to list macros first somehow. Since the values are the same, this
    /// shouldn't be an issue.
    pub fn iter(&self) -> impl Iterator<Item = crate::search::Match<'_>> {
        self.matches_by_id
            .iter()
            .filter_map(|item| item.r#match.as_ref().map(|m| m.to_outer(self)))
    }

    /// Iterate over all matches of the attribute selection in their original order.
    ///
    /// This only yields values if this instance was initialized with [`Outcome::initialize_with_selection()`].
    pub fn iter_selected(&self) -> impl Iterator<Item = crate::search::Match<'_>> {
        static DUMMY: Pattern = Pattern {
            text: BString::new(Vec::new()),
            mode: gix_glob::pattern::Mode::empty(),
            first_wildcard_pos: None,
        };
        self.selected.iter().map(|(name, id)| {
            id.and_then(|id| self.matches_by_id[id.0].r#match.as_ref().map(|m| m.to_outer(self)))
                .unwrap_or_else(|| crate::search::Match {
                    pattern: &DUMMY,
                    assignment: AssignmentRef {
                        name: NameRef::try_from(name.as_bytes().as_bstr())
                            .unwrap_or_else(|_| NameRef("invalid".into())),
                        state: StateRef::Unspecified,
                    },
                    kind: MatchKind::Attribute { macro_id: None },
                    location: crate::search::MatchLocation {
                        source: None,
                        sequence_number: 0,
                    },
                })
        })
    }

    /// Obtain a match by the order of its attribute, if the order exists in our initialized attribute list and there was a match.
    pub fn match_by_id(&self, id: AttributeId) -> Option<crate::search::Match<'_>> {
        self.matches_by_id
            .get(id.0)
            .and_then(|m| m.r#match.as_ref().map(|m| m.to_outer(self)))
    }

    /// Return `true` if there is nothing more to be done as all attributes were filled.
    pub fn is_done(&self) -> bool {
        self.remaining() == 0
    }
}

/// Mutation
impl Outcome {
    /// Fill all `attrs` and resolve them recursively if they are macros. Return `true` if there is no attribute left to be resolved and
    /// we are totally done.
    /// `pattern` is what matched a patch and is passed for contextual information,
    /// providing `sequence_number` and `source` as well.
    pub(crate) fn fill_attributes<'a>(
        &mut self,
        attrs: impl Iterator<Item = &'a TrackedAssignment>,
        pattern: &gix_glob::Pattern,
        source: Option<&std::path::PathBuf>,
        sequence_number: usize,
    ) -> bool {
        self.attrs_stack.extend(
            attrs
                .filter(|attr| self.matches_by_id[attr.id.0].r#match.is_none())
                .map(|attr| (attr.id, attr.inner.clone(), None)),
        );
        while let Some((id, assignment, parent_order)) = self.attrs_stack.pop() {
            let slot = &mut self.matches_by_id[id.0];
            if slot.r#match.is_some() {
                continue;
            }
            // Let's be explicit - this is only non-empty for macros.
            let is_macro = !slot.macro_attributes.is_empty();

            slot.r#match = Some(Match {
                pattern: self.patterns.insert(pattern),
                assignment: self.assignments.insert_owned(assignment),
                kind: if is_macro {
                    MatchKind::Macro {
                        parent_macro_id: parent_order,
                    }
                } else {
                    MatchKind::Attribute { macro_id: parent_order }
                },
                location: MatchLocation {
                    source: source.map(|path| self.source_paths.insert(path)),
                    sequence_number,
                },
            });
            if self.reduce_and_check_if_done(id) {
                return true;
            }

            if is_macro {
                // TODO(borrowchk): one fine day we should be able to re-borrow `slot` without having to redo the array access.
                let slot = &self.matches_by_id[id.0];
                self.attrs_stack.extend(
                    slot.macro_attributes
                        .iter()
                        .filter(|attr| self.matches_by_id[attr.id.0].r#match.is_none())
                        .map(|attr| (attr.id, attr.inner.clone(), Some(id))),
                );
            }
        }
        false
    }
}

impl Outcome {
    /// Given a list of `attrs` by order, return true if at least one of them is not set
    pub(crate) fn has_unspecified_attributes(&self, mut attrs: impl Iterator<Item = AttributeId>) -> bool {
        attrs.any(|order| self.matches_by_id[order.0].r#match.is_none())
    }
    /// Return the amount of attributes haven't yet been found.
    ///
    /// If this number reaches 0, then the search can be stopped as there is nothing more to fill in.
    pub(crate) fn remaining(&self) -> usize {
        self.remaining
            .expect("BUG: instance must be initialized for each search set")
    }

    fn reduce_and_check_if_done(&mut self, attr: AttributeId) -> bool {
        if self.selected.is_empty() || self.selected.iter().any(|(_name, id)| *id == Some(attr)) {
            *self.remaining.as_mut().expect("initialized") -= 1;
        }
        self.is_done()
    }
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct AsDisplay<'a>(&'a dyn std::fmt::Display);
        impl std::fmt::Debug for AsDisplay<'_> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        let mut dbg = f.debug_tuple("Outcome");
        if self.selected.is_empty() {
            for match_ in self.iter() {
                dbg.field(&AsDisplay(&match_.assignment));
            }
        } else {
            for match_ in self.iter_selected() {
                dbg.field(&AsDisplay(&match_.assignment));
            }
        }
        dbg.finish()
    }
}

/// Mutation
impl MetadataCollection {
    /// Assign order ids to each attribute either in macros (along with macros themselves) or attributes of patterns, and store
    /// them in this collection.
    ///
    /// Must be called before querying matches.
    pub fn update_from_list(&mut self, list: &mut gix_glob::search::pattern::List<Attributes>) {
        for pattern in &mut list.patterns {
            match &mut pattern.value {
                Value::MacroAssignments { id: order, assignments } => {
                    *order = self.id_for_macro(
                        pattern
                            .pattern
                            .text
                            .to_str()
                            .expect("valid macro names are always UTF8 and this was verified"),
                        assignments,
                    );
                }
                Value::Assignments(assignments) => {
                    self.assign_order_to_attributes(assignments);
                }
            }
        }
    }
}

/// Access
impl MetadataCollection {
    /// Return an iterator over the contents of the map in an easy-to-consume form.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Metadata)> {
        self.name_to_meta.iter().map(|(k, v)| (k.as_str(), v))
    }
}

impl MetadataCollection {
    pub(crate) fn id_for_macro(&mut self, name: &str, attrs: &mut Assignments) -> AttributeId {
        let order = match self.name_to_meta.get_mut(name) {
            Some(meta) => meta.id,
            None => {
                let order = AttributeId(self.name_to_meta.len());
                self.name_to_meta.insert(
                    KString::from_ref(name),
                    Metadata {
                        id: order,
                        macro_attributes: Default::default(),
                    },
                );
                order
            }
        };

        self.assign_order_to_attributes(attrs);
        self.name_to_meta
            .get_mut(name)
            .expect("just added")
            .macro_attributes
            .clone_from(attrs);

        order
    }
    pub(crate) fn id_for_attribute(&mut self, name: &str) -> AttributeId {
        match self.name_to_meta.get(name) {
            Some(meta) => meta.id,
            None => {
                let order = AttributeId(self.name_to_meta.len());
                self.name_to_meta.insert(KString::from_ref(name), order.into());
                order
            }
        }
    }
    pub(crate) fn assign_order_to_attributes(&mut self, attributes: &mut [TrackedAssignment]) {
        for TrackedAssignment {
            id: order,
            inner: crate::Assignment { name, .. },
        } in attributes
        {
            *order = self.id_for_attribute(&name.0);
        }
    }
}

impl From<AttributeId> for Metadata {
    fn from(order: AttributeId) -> Self {
        Metadata {
            id: order,
            macro_attributes: Default::default(),
        }
    }
}

impl MatchKind {
    /// return the id of the macro that resolved us, or `None` if that didn't happen.
    pub fn source_id(&self) -> Option<AttributeId> {
        match self {
            MatchKind::Attribute { macro_id: id } | MatchKind::Macro { parent_macro_id: id } => *id,
        }
    }
}

/// A version of `Match` without references.
#[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct Match {
    /// The glob pattern itself, like `/target/*`.
    pub pattern: RefMapKey,
    /// The key=value pair of the attribute that matched at the pattern. There can be multiple matches per pattern.
    pub assignment: RefMapKey,
    /// Additional information about the kind of match.
    pub kind: MatchKind,
    /// Information about the location of the match.
    pub location: MatchLocation,
}

impl Match {
    fn to_outer<'a>(&self, out: &'a Outcome) -> crate::search::Match<'a> {
        crate::search::Match {
            pattern: out.patterns.resolve(self.pattern).expect("pattern still present"),
            assignment: out
                .assignments
                .resolve(self.assignment)
                .expect("assignment present")
                .as_ref(),
            kind: self.kind,
            location: self.location.to_outer(out),
        }
    }
}

/// A version of `MatchLocation` without references.
#[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct MatchLocation {
    /// The path to the source from which the pattern was loaded, or `None` if it was specified by other means.
    pub source: Option<RefMapKey>,
    /// The line at which the pattern was found in its `source` file, or the occurrence in which it was provided.
    pub sequence_number: usize,
}

impl MatchLocation {
    fn to_outer<'a>(&self, out: &'a Outcome) -> crate::search::MatchLocation<'a> {
        crate::search::MatchLocation {
            source: self
                .source
                .and_then(|source| out.source_paths.resolve(source).map(AsRef::as_ref)),
            sequence_number: self.sequence_number,
        }
    }
}
