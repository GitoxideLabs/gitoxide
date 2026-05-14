/// What to do when having to pick a side to resolve a conflict.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ResolveWith {
    /// Chose the ancestor to resolve a conflict.
    Ancestor,
    /// Chose our side to resolve a conflict.
    Ours,
    /// Chose their side to resolve a conflict.
    Theirs,
}

/// Tell the caller of [`merge()`](function::merge) which side was picked.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Pick {
    /// Chose the ancestor.
    Ancestor,
    /// Chose our side.
    Ours,
    /// Chose their side.
    Theirs,
}

pub(super) mod function {
    use crate::blob::{
        Resolution,
        builtin_driver::binary::{Pick, ResolveWith},
    };

    /// As this algorithm doesn't look at the actual data, it returns a choice solely based on logic.
    /// This also means that the caller has to assure this only gets called if the input *doesn't* match.
    ///
    /// It always results in a conflict with `current` being picked unless `on_conflict` is not `None`,
    /// which is when we always return [`Resolution::CompleteWithAutoResolvedConflict`].
    pub fn merge(on_conflict: Option<ResolveWith>) -> (Pick, Resolution) {
        match on_conflict {
            None => (Pick::Ours, Resolution::Conflict),
            Some(resolve) => (
                match resolve {
                    ResolveWith::Ours => Pick::Ours,
                    ResolveWith::Theirs => Pick::Theirs,
                    ResolveWith::Ancestor => Pick::Ancestor,
                },
                Resolution::CompleteWithAutoResolvedConflict,
            ),
        }
    }
}
