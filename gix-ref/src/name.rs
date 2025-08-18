use std::{convert, convert::Infallible, ffi::OsStr, path::Path};

use gix_object::bstr::{BStr, BString, ByteSlice, ByteVec};

use crate::{Category, FullName, FullNameRef, PartialName, PartialNameRef};

/// The error used in the [`PartialNameRef`]`::try_from`(…) implementations.
pub type Error = gix_validate::reference::name::Error;

impl Category<'_> {
    /// Return the prefix that would contain all references of our kind, or an empty string if the reference would
    /// be directly inside of the [`git_dir()`][crate::file::Store::git_dir()].
    pub fn prefix(&self) -> &BStr {
        match self {
            Category::Tag => b"refs/tags/".as_bstr(),
            Category::LocalBranch => b"refs/heads/".as_bstr(),
            Category::RemoteBranch => b"refs/remotes/".as_bstr(),
            Category::Note => b"refs/notes/".as_bstr(),
            Category::MainPseudoRef => b"main-worktree/".as_bstr(),
            Category::MainRef => b"main-worktree/refs/".as_bstr(),
            Category::PseudoRef => b"".as_bstr(),
            Category::LinkedPseudoRef { .. } => b"worktrees/".as_bstr(),
            Category::LinkedRef { .. } => b"worktrees/".as_bstr(),
            Category::Bisect => b"refs/bisect/".as_bstr(),
            Category::Rewritten => b"refs/rewritten/".as_bstr(),
            Category::WorktreePrivate => b"refs/worktree/".as_bstr(),
        }
    }

    /// Returns true if the category is private to their worktrees, and never shared with other worktrees.
    pub fn is_worktree_private(&self) -> bool {
        matches!(
            self,
            Category::MainPseudoRef
                | Category::PseudoRef
                | Category::LinkedPseudoRef { .. }
                | Category::WorktreePrivate
                | Category::Rewritten
                | Category::Bisect
        )
    }
}

impl FullNameRef {
    pub(crate) fn new_unchecked(v: &BStr) -> &Self {
        // SAFETY: FullNameRef is transparent and equivalent to a &BStr if provided as reference
        #[allow(unsafe_code)]
        unsafe {
            std::mem::transmute(v)
        }
    }
}

impl PartialNameRef {
    pub(crate) fn new_unchecked(v: &BStr) -> &Self {
        // SAFETY: PartialNameRef is transparent and equivalent to a &BStr if provided as reference
        #[allow(unsafe_code)]
        unsafe {
            std::mem::transmute(v)
        }
    }
}

impl PartialNameRef {
    pub(crate) fn looks_like_full_name(&self, consider_pseudo_ref: bool) -> bool {
        let name = self.0.as_bstr();
        name.starts_with_str("refs/")
            || name.starts_with(Category::MainPseudoRef.prefix())
            || name.starts_with(Category::LinkedPseudoRef { name: "".into() }.prefix())
            || (consider_pseudo_ref && is_pseudo_ref(name))
    }
    pub(crate) fn construct_full_name_ref<'buf>(
        &self,
        inbetween: &str,
        buf: &'buf mut BString,
        consider_pseudo_ref: bool,
    ) -> &'buf FullNameRef {
        buf.clear();
        if !self.looks_like_full_name(consider_pseudo_ref) {
            buf.push_str("refs/");
        }
        if !inbetween.is_empty() {
            buf.push_str(inbetween);
            buf.push_byte(b'/');
        }
        buf.extend_from_slice(&self.0);
        FullNameRef::new_unchecked(buf.as_bstr())
    }
}

impl PartialNameRef {
    /// Convert this name into the relative path possibly identifying the reference location.
    /// Note that it may be only a partial path though.
    pub fn to_partial_path(&self) -> &Path {
        gix_path::from_byte_slice(self.0.as_bstr())
    }

    /// Provide the name as binary string which is known to be a valid partial ref name.
    pub fn as_bstr(&self) -> &BStr {
        &self.0
    }
}

impl PartialName {
    /// Append the `component` to ourselves and validate the newly created partial path.
    pub fn join(self, component: &BStr) -> Result<Self, Error> {
        let mut b = self.0;
        b.push_byte(b'/');
        b.extend(component.as_bytes());
        gix_validate::reference::name_partial(b.as_ref())?;
        Ok(PartialName(b))
    }
}

impl<'a> convert::TryFrom<&'a BStr> for &'a FullNameRef {
    type Error = Error;

    fn try_from(v: &'a BStr) -> Result<Self, Self::Error> {
        Ok(FullNameRef::new_unchecked(gix_validate::reference::name(v)?))
    }
}

impl<'a> From<&'a FullNameRef> for &'a PartialNameRef {
    fn from(v: &'a FullNameRef) -> Self {
        PartialNameRef::new_unchecked(v.0.as_bstr())
    }
}

impl<'a> convert::TryFrom<&'a OsStr> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a OsStr) -> Result<Self, Self::Error> {
        let v = gix_path::os_str_into_bstr(v).map_err(|_| {
            Error::Tag(gix_validate::tag::name::Error::InvalidByte {
                byte: "<unknown encoding>".into(),
            })
        })?;
        Ok(PartialNameRef::new_unchecked(gix_validate::reference::name_partial(
            v.as_bstr(),
        )?))
    }
}

mod impls {
    use std::borrow::Borrow;

    use crate::{bstr::ByteSlice, PartialName, PartialNameRef};

    impl Borrow<PartialNameRef> for PartialName {
        #[inline]
        fn borrow(&self) -> &PartialNameRef {
            PartialNameRef::new_unchecked(self.0.as_bstr())
        }
    }

    impl AsRef<PartialNameRef> for PartialName {
        fn as_ref(&self) -> &PartialNameRef {
            self.borrow()
        }
    }

    impl ToOwned for PartialNameRef {
        type Owned = PartialName;

        fn to_owned(&self) -> Self::Owned {
            PartialName(self.0.to_owned())
        }
    }
}

impl<'a> convert::TryFrom<&'a BString> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a BString) -> Result<Self, Self::Error> {
        Ok(PartialNameRef::new_unchecked(gix_validate::reference::name_partial(
            v.as_ref(),
        )?))
    }
}

impl<'a> convert::TryFrom<&'a BStr> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a BStr) -> Result<Self, Self::Error> {
        Ok(PartialNameRef::new_unchecked(gix_validate::reference::name_partial(v)?))
    }
}

impl<'a> convert::TryFrom<&'a PartialName> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a PartialName) -> Result<Self, Self::Error> {
        Ok(PartialNameRef::new_unchecked(v.0.as_bstr()))
    }
}

impl<'a> convert::TryFrom<&'a str> for &'a FullNameRef {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<Self, Self::Error> {
        let v = v.as_bytes().as_bstr();
        Ok(FullNameRef::new_unchecked(gix_validate::reference::name(v)?))
    }
}

impl<'a> convert::TryFrom<&'a str> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<Self, Self::Error> {
        let v = v.as_bytes().as_bstr();
        Ok(PartialNameRef::new_unchecked(gix_validate::reference::name_partial(v)?))
    }
}

impl<'a> convert::TryFrom<&'a str> for PartialName {
    type Error = Error;

    fn try_from(v: &'a str) -> Result<Self, Self::Error> {
        let v = v.as_bytes().as_bstr();
        Ok(PartialName(gix_validate::reference::name_partial(v)?.to_owned()))
    }
}

#[allow(clippy::infallible_try_from)]
impl<'a> convert::TryFrom<&'a FullName> for &'a PartialNameRef {
    type Error = Infallible;

    fn try_from(v: &'a FullName) -> Result<Self, Self::Error> {
        Ok(v.as_ref().as_partial_name())
    }
}

impl<'a> convert::TryFrom<&'a String> for &'a FullNameRef {
    type Error = Error;

    fn try_from(v: &'a String) -> Result<Self, Self::Error> {
        let v = v.as_bytes().as_bstr();
        Ok(FullNameRef::new_unchecked(gix_validate::reference::name(v)?))
    }
}

impl<'a> convert::TryFrom<&'a String> for &'a PartialNameRef {
    type Error = Error;

    fn try_from(v: &'a String) -> Result<Self, Self::Error> {
        let v = v.as_bytes().as_bstr();
        Ok(PartialNameRef::new_unchecked(gix_validate::reference::name_partial(v)?))
    }
}

impl convert::TryFrom<String> for PartialName {
    type Error = Error;

    fn try_from(v: String) -> Result<Self, Self::Error> {
        gix_validate::reference::name_partial(v.as_bytes().as_bstr())?;
        Ok(PartialName(v.into()))
    }
}

impl convert::TryFrom<BString> for PartialName {
    type Error = Error;

    fn try_from(v: BString) -> Result<Self, Self::Error> {
        gix_validate::reference::name_partial(v.as_ref())?;
        Ok(PartialName(v))
    }
}

/// Note that this method is disagreeing with `gix_validate` as it allows dashes '-' for some reason.
/// Since partial names cannot be created with dashes inside we adjusted this as it's probably unintended or git creates pseudo-refs
/// which wouldn't pass its safety checks.
pub(crate) fn is_pseudo_ref(name: &BStr) -> bool {
    name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')
}
