use std::{borrow::Borrow, path::Path};

use gix_object::bstr::{BStr, BString, ByteSlice};

use crate::{bstr::ByteVec, name::is_pseudo_ref, Category, FullName, FullNameRef, Namespace, PartialNameRef};

impl TryFrom<&str> for FullName {
    type Error = gix_validate::reference::name::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(FullName(
            gix_validate::reference::name(value.as_bytes().as_bstr())?.into(),
        ))
    }
}

impl TryFrom<String> for FullName {
    type Error = gix_validate::reference::name::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        gix_validate::reference::name(value.as_bytes().as_bstr())?;
        Ok(FullName(value.into()))
    }
}

impl TryFrom<&BStr> for FullName {
    type Error = gix_validate::reference::name::Error;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        Ok(FullName(gix_validate::reference::name(value)?.into()))
    }
}

impl TryFrom<BString> for FullName {
    type Error = gix_validate::reference::name::Error;

    fn try_from(value: BString) -> Result<Self, Self::Error> {
        gix_validate::reference::name(value.as_ref())?;
        Ok(FullName(value))
    }
}

impl TryFrom<&BString> for FullName {
    type Error = gix_validate::reference::name::Error;

    fn try_from(value: &BString) -> Result<Self, Self::Error> {
        gix_validate::reference::name(value.as_ref())?;
        Ok(FullName(value.clone()))
    }
}

impl From<FullName> for BString {
    fn from(name: FullName) -> Self {
        name.0
    }
}

impl<'a> From<&'a FullNameRef> for &'a BStr {
    fn from(name: &'a FullNameRef) -> Self {
        &name.0
    }
}

impl<'a> From<&'a FullNameRef> for FullName {
    fn from(value: &'a FullNameRef) -> Self {
        FullName(value.as_bstr().into())
    }
}

impl std::fmt::Display for FullName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl std::fmt::Display for FullNameRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

impl FullNameRef {
    /// Interpret this fully qualified reference name as partial name.
    pub fn as_partial_name(&self) -> &PartialNameRef {
        PartialNameRef::new_unchecked(self.0.as_bstr())
    }

    /// Convert this name into the relative path identifying the reference location.
    pub fn to_path(&self) -> &Path {
        gix_path::from_byte_slice(&self.0)
    }

    /// Return ourselves as byte string which is a valid refname
    pub fn as_bstr(&self) -> &BStr {
        &self.0
    }

    /// Strip well-known prefixes from the name and return it.
    ///
    /// If there is no such prefix, the original name is returned.
    pub fn shorten(&self) -> &BStr {
        self.category_and_short_name()
            .map_or_else(|| self.0.as_bstr(), |(_, short)| short)
    }

    /// Classify this name, or return `None` if it's unclassified.
    pub fn category(&self) -> Option<Category<'_>> {
        self.category_and_short_name().map(|(cat, _)| cat)
    }

    /// Classify this name, or return `None` if it's unclassified. If `Some`,
    /// the shortened name is returned as well.
    pub fn category_and_short_name(&self) -> Option<(Category<'_>, &BStr)> {
        let name = self.0.as_bstr();
        for category in &[Category::Tag, Category::LocalBranch, Category::RemoteBranch] {
            if let Some(shortened) = name.strip_prefix(category.prefix().as_bytes()) {
                return Some((*category, shortened.as_bstr()));
            }
        }

        for category in &[
            Category::Note,
            Category::Bisect,
            Category::WorktreePrivate,
            Category::Rewritten,
        ] {
            if name.starts_with(category.prefix().as_ref()) {
                return Some((
                    *category,
                    name.strip_prefix(b"refs/")
                        .expect("we checked for refs/* above")
                        .as_bstr(),
                ));
            }
        }

        if is_pseudo_ref(name) {
            Some((Category::PseudoRef, name))
        } else if let Some(shortened) = name.strip_prefix(Category::MainPseudoRef.prefix().as_bytes()) {
            if shortened.starts_with_str("refs/") {
                (Category::MainRef, shortened.as_bstr()).into()
            } else {
                is_pseudo_ref(shortened.into()).then(|| (Category::MainPseudoRef, shortened.as_bstr()))
            }
        } else if let Some(shortened_with_worktree_name) =
            name.strip_prefix(Category::LinkedPseudoRef { name: "".into() }.prefix().as_bytes())
        {
            let (name, shortened) = shortened_with_worktree_name.find_byte(b'/').map(|pos| {
                (
                    shortened_with_worktree_name[..pos].as_bstr(),
                    shortened_with_worktree_name[pos + 1..].as_bstr(),
                )
            })?;
            if shortened.starts_with_str("refs/") {
                (Category::LinkedRef { name }, shortened.as_bstr()).into()
            } else {
                is_pseudo_ref(shortened).then(|| (Category::LinkedPseudoRef { name }, shortened.as_bstr()))
            }
        } else {
            None
        }
    }
}

/// Conversion
impl Category<'_> {
    /// As the inverse of [`FullNameRef::category_and_short_name()`], use the prefix of this category alongside
    /// the `short_name` to create a valid fully qualified [reference name](FullName).
    pub fn to_full_name<'a>(&self, short_name: impl Into<&'a BStr>) -> Result<FullName, crate::name::Error> {
        let mut out: BString = self.prefix().into();
        let short_name = short_name.into();
        let partial_name = match self {
            Category::Note => short_name.strip_prefix("notes/".as_bytes()).unwrap_or(short_name),
            Category::MainRef => short_name.strip_prefix("refs/".as_bytes()).unwrap_or(short_name),
            Category::LinkedPseudoRef { name } | Category::LinkedRef { name } => {
                out.extend_from_slice(name);
                out.push(b'/');
                short_name
            }
            Category::Bisect => short_name.strip_prefix("bisect/".as_bytes()).unwrap_or(short_name),
            Category::Rewritten => short_name.strip_prefix("rewritten/".as_bytes()).unwrap_or(short_name),
            Category::WorktreePrivate => short_name.strip_prefix("worktree/".as_bytes()).unwrap_or(short_name),
            Category::Tag
            | Category::LocalBranch
            | Category::RemoteBranch
            | Category::PseudoRef
            | Category::MainPseudoRef => short_name,
        };
        out.extend_from_slice(partial_name);
        FullName::try_from(out)
    }
}

impl FullName {
    /// Convert this name into the relative path, lossily, identifying the reference location relative to a repository
    pub fn to_path(&self) -> &Path {
        gix_path::from_byte_slice(&self.0)
    }

    /// Dissolve this instance and return the buffer.
    pub fn into_inner(self) -> BString {
        self.0
    }

    /// Return ourselves as byte string which is a valid refname
    pub fn as_bstr(&self) -> &BStr {
        self.0.as_bstr()
    }

    /// Modify ourself so that we use `namespace` as prefix, if it is not yet in the `namespace`
    pub fn prefix_namespace(&mut self, namespace: &Namespace) -> &mut Self {
        if !self.0.starts_with_str(&namespace.0) {
            self.0.insert_str(0, &namespace.0);
        }
        self
    }

    /// Strip the given `namespace` off the beginning of this name, if it is in this namespace.
    pub fn strip_namespace(&mut self, namespace: &Namespace) -> &mut Self {
        if self.0.starts_with_str(&namespace.0) {
            let prev_len = self.0.len();
            self.0.copy_within(namespace.0.len().., 0);
            self.0.resize(prev_len - namespace.0.len(), 0);
        }
        self
    }

    /// Strip well-known prefixes from the name and return it.
    ///
    /// If there is no such prefix, the original name is returned.
    pub fn shorten(&self) -> &BStr {
        self.as_ref().shorten()
    }

    /// Classify this name, or return `None` if it's unclassified.
    pub fn category(&self) -> Option<crate::Category<'_>> {
        self.as_ref().category()
    }

    /// Classify this name, or return `None` if it's unclassified. If `Some`,
    /// the shortened name is returned as well.
    pub fn category_and_short_name(&self) -> Option<(crate::Category<'_>, &BStr)> {
        self.as_ref().category_and_short_name()
    }
}

impl FullNameRef {
    /// Return the file name portion of a full name, for instance `main` if the
    /// full name was `refs/heads/main`.
    pub fn file_name(&self) -> &BStr {
        self.0.rsplitn(2, |b| *b == b'/').next().expect("valid ref").as_bstr()
    }
}

impl Borrow<FullNameRef> for FullName {
    #[inline]
    fn borrow(&self) -> &FullNameRef {
        FullNameRef::new_unchecked(self.0.as_bstr())
    }
}

impl AsRef<FullNameRef> for FullName {
    fn as_ref(&self) -> &FullNameRef {
        self.borrow()
    }
}

impl ToOwned for FullNameRef {
    type Owned = FullName;

    fn to_owned(&self) -> Self::Owned {
        FullName(self.0.to_owned())
    }
}
