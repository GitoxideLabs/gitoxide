use std::path::Path;

use gix_object::bstr::{BStr, BString, ByteSlice, ByteVec};
use gix_path::relative_path::RelativePath;

use crate::{FullName, FullNameRef, Namespace, PartialNameRef};

impl Namespace {
    /// Dissolve ourselves into the interior representation
    pub fn into_bstring(self) -> BString {
        self.0
    }
    /// Return ourselves as
    pub fn as_bstr(&self) -> &BStr {
        self.0.as_ref()
    }
    /// Return ourselves as a path for use within the filesystem.
    pub fn to_path(&self) -> &Path {
        gix_path::from_byte_slice(&self.0)
    }
    /// Append the given `prefix` to this namespace so it becomes usable for prefixed iteration.
    ///
    /// The prefix is a relative path with slash-separated path components.
    // TODO: use `RelativePath` type instead (see #1921), or a trait that helps convert into it.
    pub fn into_namespaced_prefix<'a>(mut self, prefix: &'a RelativePath) -> BString {
        self.0.push_str(prefix);
        gix_path::to_unix_separators_on_windows(self.0).into_owned()
    }
    pub(crate) fn into_namespaced_name(mut self, name: &FullNameRef) -> FullName {
        self.0.push_str(name.as_bstr());
        FullName(self.0)
    }
}

/// Given a `namespace` 'foo we output 'refs/namespaces/foo', and given 'foo/bar' we output 'refs/namespaces/foo/refs/namespaces/bar'.
///
/// For more information, consult the [git namespace documentation](https://git-scm.com/docs/gitnamespaces).
pub fn expand<'a, Name, E>(namespace: Name) -> Result<Namespace, gix_validate::reference::name::Error>
where
    Name: TryInto<&'a PartialNameRef, Error = E>,
    gix_validate::reference::name::Error: From<E>,
{
    let namespace = &namespace.try_into()?.0;
    let mut out = BString::default();
    for component in namespace.split_str(b"/") {
        out.push_str("refs/namespaces/");
        out.push_str(component);
        out.push_str(b"/");
    }
    Ok(Namespace(out))
}
