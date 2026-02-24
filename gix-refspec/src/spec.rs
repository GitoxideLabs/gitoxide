use bstr::{BStr, BString, ByteSlice};

use crate::{
    instruction::{Fetch, Push},
    parse::Operation,
    types::Mode,
    Instruction, RefSpec, RefSpecRef,
};

/// Conversion. Use the [`RefSpecRef`][RefSpec::to_ref()] type for more usage options.
impl RefSpec {
    /// Return ourselves as reference type.
    pub fn to_ref(&self) -> RefSpecRef<'_> {
        RefSpecRef {
            mode: self.mode,
            op: self.op,
            src: self.src.as_ref().map(AsRef::as_ref),
            dst: self.dst.as_ref().map(AsRef::as_ref),
        }
    }

    /// Return true if the spec stats with a `+` and thus forces setting the reference.
    pub fn allow_non_fast_forward(&self) -> bool {
        matches!(self.mode, Mode::Force)
    }
}

mod impls {
    use std::{
        cmp::Ordering,
        hash::{Hash, Hasher},
    };

    use crate::{RefSpec, RefSpecRef};

    impl From<RefSpecRef<'_>> for RefSpec {
        fn from(v: RefSpecRef<'_>) -> Self {
            v.to_owned()
        }
    }

    impl Hash for RefSpec {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.to_ref().hash(state);
        }
    }

    impl Hash for RefSpecRef<'_> {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.instruction().hash(state);
        }
    }

    impl PartialEq for RefSpec {
        fn eq(&self, other: &Self) -> bool {
            self.to_ref().eq(&other.to_ref())
        }
    }

    impl PartialEq for RefSpecRef<'_> {
        fn eq(&self, other: &Self) -> bool {
            self.instruction().eq(&other.instruction())
        }
    }

    impl PartialOrd for RefSpecRef<'_> {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    #[allow(clippy::non_canonical_partial_ord_impl)]
    impl PartialOrd for RefSpec {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.to_ref().cmp(&other.to_ref()))
        }
    }

    impl Ord for RefSpecRef<'_> {
        fn cmp(&self, other: &Self) -> Ordering {
            self.instruction().cmp(&other.instruction())
        }
    }

    impl Ord for RefSpec {
        fn cmp(&self, other: &Self) -> Ordering {
            self.to_ref().cmp(&other.to_ref())
        }
    }
}

/// Access
impl<'a> RefSpecRef<'a> {
    /// Return the left-hand side of the spec, typically the source.
    /// It takes many different forms so don't rely on this being a ref name.
    ///
    /// It's not present in case of deletions.
    pub fn source(&self) -> Option<&BStr> {
        self.src
    }

    /// Return the right-hand side of the spec, typically the destination.
    /// It takes many different forms so don't rely on this being a ref name.
    ///
    /// It's not present in case of source-only specs.
    pub fn destination(&self) -> Option<&BStr> {
        self.dst
    }

    /// Always returns the remote side, whose actual side in the refspec depends on how it was parsed.
    pub fn remote(&self) -> Option<&BStr> {
        match self.op {
            Operation::Push => self.dst,
            Operation::Fetch => self.src,
        }
    }

    /// Always returns the local side, whose actual side in the refspec depends on how it was parsed.
    pub fn local(&self) -> Option<&BStr> {
        match self.op {
            Operation::Push => self.src,
            Operation::Fetch => self.dst,
        }
    }

    /// Derive the prefix from the [`source`][Self::source()] side of this spec if this is a fetch spec,
    /// or the [`destination`][Self::destination()] side if it is a push spec, if it is possible to do so without ambiguity.
    ///
    /// This means it starts with `refs/`. Note that it won't contain more than two components, like `refs/heads/`
    pub fn prefix(&self) -> Option<&BStr> {
        if self.mode == Mode::Negative {
            return None;
        }
        let source = match self.op {
            Operation::Fetch => self.source(),
            Operation::Push => self.destination(),
        }?;
        if source == "HEAD" {
            return source.into();
        }

        let sans_refs_prefix = source.strip_prefix(b"refs/")?;
        if let Some(star_pos) = sans_refs_prefix.find_byte(b'*') {
            // Disallow `*` glob star components after `refs/`
            if star_pos == 0 {
                return None;
            }
            let prefix = &source[.."refs/".len() + star_pos];
            return (!prefix.is_empty()).then_some(prefix.as_bstr());
        }
        Some(source)
    }

    /// As opposed to [`prefix()`][Self::prefix], if the latter is `None` it will expand to all possible prefixes and place them in `out`.
    ///
    /// Note that only the `source` side is considered.
    pub fn expand_prefixes(&self, out: &mut Vec<BString>) {
        match self.prefix() {
            Some(prefix) => out.push(prefix.into()),
            None => {
                let source = match match self.op {
                    Operation::Fetch => self.source(),
                    Operation::Push => self.destination(),
                } {
                    Some(source) => source,
                    None => return,
                };
                if let Some(rest) = source.strip_prefix(b"refs/") {
                    if !rest.contains(&b'/') {
                        out.push(source.into());
                    }
                    return;
                } else if gix_hash::ObjectId::from_hex(source).is_ok() {
                    return;
                }
                expand_partial_name(source, |expanded| {
                    out.push(expanded.into());
                    None::<()>
                });
            }
        }
    }

    /// Transform the state of the refspec into an instruction making clear what to do with it.
    pub fn instruction(&self) -> Instruction<'a> {
        match self.op {
            Operation::Fetch => match (self.mode, self.src, self.dst) {
                (Mode::Normal | Mode::Force, Some(src), None) => Instruction::Fetch(Fetch::Only { src }),
                (Mode::Normal | Mode::Force, Some(src), Some(dst)) => Instruction::Fetch(Fetch::AndUpdate {
                    src,
                    dst,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Negative, Some(src), None) => Instruction::Fetch(Fetch::Exclude { src }),
                (mode, src, dest) => {
                    unreachable!(
                        "BUG: fetch instructions with {:?} {:?} {:?} are not possible",
                        mode, src, dest
                    )
                }
            },
            Operation::Push => match (self.mode, self.src, self.dst) {
                (Mode::Normal | Mode::Force, Some(src), None) => Instruction::Push(Push::Matching {
                    src,
                    dst: src,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Normal | Mode::Force, None, Some(dst)) => {
                    Instruction::Push(Push::Delete { ref_or_pattern: dst })
                }
                (Mode::Normal | Mode::Force, None, None) => Instruction::Push(Push::AllMatchingBranches {
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Normal | Mode::Force, Some(src), Some(dst)) => Instruction::Push(Push::Matching {
                    src,
                    dst,
                    allow_non_fast_forward: matches!(self.mode, Mode::Force),
                }),
                (Mode::Negative, Some(src), None) => Instruction::Push(Push::Exclude { src }),
                (mode, src, dest) => {
                    unreachable!(
                        "BUG: push instructions with {:?} {:?} {:?} are not possible",
                        mode, src, dest
                    )
                }
            },
        }
    }
}

/// Conversion
impl RefSpecRef<'_> {
    /// Convert this ref into a standalone, owned copy.
    pub fn to_owned(&self) -> RefSpec {
        RefSpec {
            mode: self.mode,
            op: self.op,
            src: self.src.map(ToOwned::to_owned),
            dst: self.dst.map(ToOwned::to_owned),
        }
    }
}

pub(crate) fn expand_partial_name<T>(name: &BStr, mut cb: impl FnMut(&BStr) -> Option<T>) -> Option<T> {
    use bstr::ByteVec;
    let mut buf = BString::from(Vec::with_capacity(128));
    for (base, append_head) in [
        ("", false),
        ("refs/", false),
        ("refs/tags/", false),
        ("refs/heads/", false),
        ("refs/remotes/", false),
        ("refs/remotes/", true),
    ] {
        buf.clear();
        buf.push_str(base);
        buf.push_str(name);
        if append_head {
            buf.push_str("/HEAD");
        }
        if let Some(res) = cb(buf.as_ref()) {
            return Some(res);
        }
    }
    None
}
