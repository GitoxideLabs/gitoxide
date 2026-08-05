use std::{
    borrow::Cow,
    io::{self, Read},
    path::{Path, PathBuf},
};

pub use error::Error;

use crate::{
    BStr, BString, FullNameRef, PartialName, PartialNameRef, Reference, file,
    name::is_pseudo_ref,
    store_impl::{file::loose, packed},
};

/// ### Finding References - notes about precomposed unicode.
///
/// Generally, ref names and the target of symbolic refs are stored as-is if [`Self::precompose_unicode`] is `false`.
/// If `true`, refs are stored as precomposed unicode in `packed-refs`, but stored as is on disk as it is then assumed
/// to be indifferent, i.e. `"a\u{308}"` is the same as `"ä"`.
///
/// This also means that when refs are packed for transmission to another machine, both their names and the target of
/// symbolic references need to be precomposed.
///
/// Namespaces are left as is as they never get past the particular repository that uses them.
impl file::Store {
    /// Find a single reference by the given `path` which is required to be a valid reference name.
    ///
    /// Returns `Ok(None)` if no such ref exists.
    ///
    /// ### Note
    ///
    /// * The lookup algorithm follows the one in [the git documentation][git-lookup-docs].
    /// * The packed buffer is checked for modifications each time the method is called. See [`file::Store::try_find_packed()`]
    ///   for a version with more control.
    ///
    /// [git-lookup-docs]: https://github.com/git/git/blob/5d5b1473453400224ebb126bf3947e0a3276bdf5/Documentation/revisions.txt#L34-L46
    pub fn try_find<'a, Name, E>(&self, partial: Name) -> Result<Option<Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        let packed = self.assure_packed_refs_uptodate()?;
        self.find_one_with_verified_input(partial.try_into()?, packed.as_ref().map(|b| &***b))
    }

    /// Similar to [`file::Store::find()`] but a non-existing ref is treated as error.
    ///
    /// Find only loose references, that is references that aren't in the packed-refs buffer.
    /// All symbolic references are loose references.
    /// `HEAD` is always a loose reference.
    pub fn try_find_loose<'a, Name, E>(&self, partial: Name) -> Result<Option<loose::Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        self.find_one_with_verified_input(partial.try_into()?, None)
            .map(|r| r.map(Into::into))
    }

    /// Similar to [`file::Store::find()`], but allows to pass a snapshotted packed buffer instead.
    pub fn try_find_packed<'a, Name, E>(
        &self,
        partial: Name,
        packed: Option<&packed::Buffer>,
    ) -> Result<Option<Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        self.find_one_with_verified_input(partial.try_into()?, packed)
    }

    pub(crate) fn find_one_with_verified_input(
        &self,
        partial_name: &PartialNameRef,
        packed: Option<&packed::Buffer>,
    ) -> Result<Option<Reference>, Error> {
        fn decompose_if(mut r: Reference, input_changed_to_precomposed: bool) -> Reference {
            if input_changed_to_precomposed {
                use gix_object::bstr::ByteSlice;
                let decomposed = r
                    .name
                    .0
                    .to_str()
                    .ok()
                    .map(|name| gix_utils::str::decompose(name.into()));
                if let Some(Cow::Owned(decomposed)) = decomposed {
                    r.name.0 = decomposed.into();
                }
            }
            r
        }
        let mut buf = BString::default();
        let mut precomposed_partial_name_storage = packed.filter(|_| self.precompose_unicode).and_then(|_| {
            use gix_object::bstr::ByteSlice;
            let precomposed = partial_name.0.to_str().ok()?;
            let precomposed = gix_utils::str::precompose(precomposed.into());
            match precomposed {
                Cow::Owned(precomposed) => Some(PartialName(precomposed.into())),
                Cow::Borrowed(_) => None,
            }
        });
        let precomposed_partial_name = precomposed_partial_name_storage
            .as_ref()
            .map(std::convert::AsRef::as_ref);
        for consider_pseudo_ref in [true, false] {
            if !consider_pseudo_ref && !is_pseudo_ref(partial_name.as_bstr()) {
                break;
            }
            'try_directories: for inbetween in &["", "tags", "heads", "remotes"] {
                match self.find_inner(
                    inbetween,
                    partial_name,
                    precomposed_partial_name,
                    packed,
                    &mut buf,
                    consider_pseudo_ref,
                ) {
                    Ok(Some(r)) => return Ok(Some(decompose_if(r, precomposed_partial_name.is_some()))),
                    Ok(None) => {
                        if consider_pseudo_ref && is_pseudo_ref(partial_name.as_bstr()) {
                            break 'try_directories;
                        }
                        continue;
                    }
                    Err(err) => return Err(err),
                }
            }
        }
        if partial_name.as_bstr() != "HEAD" {
            if let Some(mut precomposed) = precomposed_partial_name_storage {
                precomposed = precomposed.join("HEAD".into()).expect("HEAD is valid name");
                precomposed_partial_name_storage = Some(precomposed);
            }
            self.find_inner(
                "remotes",
                partial_name
                    .to_owned()
                    .join("HEAD".into())
                    .expect("HEAD is valid name")
                    .as_ref(),
                precomposed_partial_name_storage
                    .as_ref()
                    .map(std::convert::AsRef::as_ref),
                None,
                &mut buf,
                true, /* consider-pseudo-ref */
            )
            .map(|res| res.map(|r| decompose_if(r, precomposed_partial_name_storage.is_some())))
        } else {
            Ok(None)
        }
    }

    fn find_inner(
        &self,
        inbetween: &str,
        partial_name: &PartialNameRef,
        precomposed_partial_name: Option<&PartialNameRef>,
        packed: Option<&packed::Buffer>,
        path_buf: &mut BString,
        consider_pseudo_ref: bool,
    ) -> Result<Option<Reference>, Error> {
        let full_name = precomposed_partial_name
            .unwrap_or(partial_name)
            .construct_full_name_ref(inbetween, path_buf, consider_pseudo_ref);
        let content_buf = match self.ref_contents(full_name) {
            Ok(content_buf) => content_buf,
            Err(err) if err.kind() == io::ErrorKind::NotADirectory => return Ok(None),
            Err(err) => {
                return Err(Error::ReadFileContents {
                    source: err,
                    path: self.reference_path(full_name),
                });
            }
        };

        match content_buf {
            None => {
                if let Some(packed) = packed {
                    if let Some(full_name) = packed::find::transform_full_name_for_lookup(full_name) {
                        let full_name_backing;
                        let full_name = match &self.namespace {
                            Some(namespace) => {
                                full_name_backing = namespace.to_owned().into_namespaced_name(full_name);
                                full_name_backing.as_ref()
                            }
                            None => full_name,
                        };
                        if let Some(packed_ref) = packed.try_find_full_name(full_name)? {
                            let mut res: Reference = packed_ref.into();
                            if let Some(namespace) = &self.namespace {
                                res.strip_namespace(namespace);
                            }
                            return Ok(Some(res));
                        }
                    }
                }
                Ok(None)
            }
            Some(content) => Ok(Some(
                loose::Reference::try_from_path(full_name.to_owned(), &content, self.object_hash)
                    .map(Into::into)
                    .map(|mut r: Reference| {
                        if let Some(namespace) = &self.namespace {
                            r.strip_namespace(namespace);
                        }
                        r
                    })
                    .map_err(|err| Error::ReferenceCreation {
                        source: err,
                        relative_path: full_name.to_path().to_owned(),
                    })?,
            )),
        }
    }
}

impl file::Store {
    pub(crate) fn to_base_dir_and_relative_name<'a>(
        &self,
        name: &'a FullNameRef,
        is_reflog: bool,
    ) -> (Cow<'_, Path>, &'a FullNameRef) {
        let commondir = self.common_dir_resolved();
        let linked_git_dir =
            |worktree_name: &BStr| commondir.join("worktrees").join(gix_path::from_bstr(worktree_name));
        name.category_and_short_name()
            .map(|(c, sn)| {
                use crate::Category::*;
                let sn = FullNameRef::new_unchecked(sn);
                match c {
                    LinkedPseudoRef { name: worktree_name } => {
                        if is_reflog {
                            (linked_git_dir(worktree_name).into(), sn)
                        } else {
                            (commondir.into(), name)
                        }
                    }
                    Tag | LocalBranch | RemoteBranch | Note => (commondir.into(), name),
                    MainRef | MainPseudoRef => (commondir.into(), sn),
                    LinkedRef { name: worktree_name } => {
                        if sn.category().is_some_and(|cat| cat.is_worktree_private()) {
                            if is_reflog {
                                (linked_git_dir(worktree_name).into(), sn)
                            } else {
                                (commondir.into(), name)
                            }
                        } else {
                            (commondir.into(), sn)
                        }
                    }
                    PseudoRef | Bisect | Rewritten | WorktreePrivate => (self.git_dir.as_path().into(), name),
                }
            })
            .unwrap_or((commondir.into(), name))
    }

    /// Implements the logic required to transform a fully qualified refname into a filesystem path
    pub(crate) fn reference_path_with_base<'b>(&self, name: &'b FullNameRef) -> (Cow<'_, Path>, Cow<'b, Path>) {
        let (base, name) = self.to_base_dir_and_relative_name(name, false);
        (
            base,
            match &self.namespace {
                None => gix_path::to_native_path_on_windows(name.as_bstr()),
                Some(namespace) => {
                    gix_path::to_native_path_on_windows(namespace.to_owned().into_namespaced_name(name).into_inner())
                }
            },
        )
    }

    /// Implements the logic required to transform a fully qualified refname into a filesystem path
    pub(crate) fn reference_path(&self, name: &FullNameRef) -> PathBuf {
        let (base, relative_path) = self.reference_path_with_base(name);
        base.join(relative_path)
    }

    /// If `prohibit_windows_device_names` is set, check that `name` does not
    /// contain a path component that matches a reserved Windows device name.
    pub(crate) fn check_windows_device_name(&self, name: &FullNameRef) -> io::Result<()> {
        if !self.prohibit_windows_device_names {
            return Ok(());
        }
        let (_, relative_path) = self.reference_path_with_base(name);
        if relative_path
            .components()
            .filter_map(|c| gix_path::try_os_str_into_bstr(c.as_os_str().into()).ok())
            .any(|c| gix_validate::path::component_is_windows_device(c.as_ref()))
        {
            Err(std::io::Error::other(format!(
                "Illegal use of reserved Windows device name in \"{}\"",
                name.as_bstr()
            )))
        } else {
            Ok(())
        }
    }

    /// Read the file contents with a verified full reference path and return it in the given vector if possible.
    pub(crate) fn ref_contents(&self, name: &FullNameRef) -> io::Result<Option<Vec<u8>>> {
        self.check_windows_device_name(name)?;
        let (base, relative_path) = self.reference_path_with_base(name);
        let ref_path = base.join(&relative_path);
        match std::fs::File::open(&ref_path) {
            Ok(mut file) => {
                let mut buf = Vec::with_capacity(128);
                if let Err(err) = file.read_to_end(&mut buf) {
                    return if ref_path.is_dir() { Ok(None) } else { Err(err) };
                }
                Ok(buf.into())
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                #[cfg(windows)]
                if path_has_file_prefix(base.as_ref(), relative_path.as_ref()) {
                    return Err(io::Error::new(io::ErrorKind::NotADirectory, err));
                }
                Ok(None)
            }
            #[cfg(windows)]
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => {
                if path_has_file_prefix(base.as_ref(), relative_path.as_ref()) {
                    Err(io::Error::new(io::ErrorKind::NotADirectory, err))
                } else {
                    Ok(None)
                }
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(windows)]
fn path_has_file_prefix(base: &Path, relative_path: &Path) -> bool {
    let mut path = base.to_owned();
    let mut components = relative_path.components().peekable();
    while let Some(component) = components.next() {
        if components.peek().is_none() {
            break;
        }
        path.push(component.as_os_str());
        match std::fs::metadata(&path) {
            Ok(metadata) if metadata.is_file() => return true,
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => return false,
            Err(_) => {}
        }
    }
    false
}

///
pub mod existing {
    pub use error::Error;

    use crate::{
        PartialNameRef, Reference,
        file::{self},
        store_impl::{
            file::{find, loose},
            packed,
        },
    };

    impl file::Store {
        /// Similar to [`file::Store::try_find()`] but a non-existing ref is treated as error.
        pub fn find<'a, Name, E>(&self, partial: Name) -> Result<Reference, Error>
        where
            Name: TryInto<&'a PartialNameRef, Error = E>,
            crate::name::Error: From<E>,
        {
            let packed = self.assure_packed_refs_uptodate().map_err(find::Error::PackedOpen)?;
            self.find_existing_inner(partial, packed.as_ref().map(|b| &***b))
        }

        /// Similar to [`file::Store::find()`], but supports a stable packed buffer.
        pub fn find_packed<'a, Name, E>(
            &self,
            partial: Name,
            packed: Option<&packed::Buffer>,
        ) -> Result<Reference, Error>
        where
            Name: TryInto<&'a PartialNameRef, Error = E>,
            crate::name::Error: From<E>,
        {
            self.find_existing_inner(partial, packed)
        }

        /// Similar to [`file::Store::find()`] won't handle packed-refs.
        pub fn find_loose<'a, Name, E>(&self, partial: Name) -> Result<loose::Reference, Error>
        where
            Name: TryInto<&'a PartialNameRef, Error = E>,
            crate::name::Error: From<E>,
        {
            self.find_existing_inner(partial, None).map(Into::into)
        }

        /// Similar to [`file::Store::find()`] but a non-existing ref is treated as error.
        pub(crate) fn find_existing_inner<'a, Name, E>(
            &self,
            partial: Name,
            packed: Option<&packed::Buffer>,
        ) -> Result<Reference, Error>
        where
            Name: TryInto<&'a PartialNameRef, Error = E>,
            crate::name::Error: From<E>,
        {
            let path = partial
                .try_into()
                .map_err(|err| Error::Find(find::Error::RefnameValidation(err.into())))?;
            match self.find_one_with_verified_input(path, packed) {
                Ok(Some(r)) => Ok(r),
                Ok(None) => Err(Error::NotFound {
                    name: path.to_partial_path().to_owned(),
                }),
                Err(err) => Err(err.into()),
            }
        }
    }

    mod error {
        use std::path::PathBuf;

        use crate::store_impl::file::find;

        /// The error returned by [file::Store::find_existing()][crate::file::Store::find()].
        #[derive(Debug)]
        #[expect(missing_docs)]
        pub enum Error {
            Find(find::Error),
            NotFound { name: PathBuf },
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Find(_) => f.write_str("An error occurred while trying to find a reference"),
                    #[allow(clippy::unnecessary_debug_formatting)]
                    // `{:?}` of a `Path` is what `thiserror` generated; keep the rendered text identical.
                    Error::NotFound { name } => write!(f, "The ref partially named {name:?} could not be found"),
                }
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Error::Find(err) => Some(err),
                    Error::NotFound { .. } => None,
                }
            }
        }

        impl From<find::Error> for Error {
            fn from(err: find::Error) -> Self {
                Error::Find(err)
            }
        }
    }
}

mod error {
    use std::{convert::Infallible, io, path::PathBuf};

    use crate::{file, store_impl::packed};

    /// The error returned by [file::Store::find()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        RefnameValidation(crate::name::Error),
        ReadFileContents {
            source: io::Error,
            path: PathBuf,
        },
        ReferenceCreation {
            source: file::loose::reference::decode::Error,
            relative_path: PathBuf,
        },
        PackedRef(packed::find::Error),
        PackedOpen(packed::buffer::open::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::RefnameValidation(_) => f.write_str("The ref name or path is not a valid ref name"),
                #[allow(clippy::unnecessary_debug_formatting)]
                // `{:?}` of a `Path` is what `thiserror` generated; keep the rendered text identical.
                Error::ReadFileContents { path, .. } => {
                    write!(f, "The ref file {path:?} could not be read in full")
                }
                Error::ReferenceCreation { relative_path, .. } => {
                    write!(
                        f,
                        "The reference at \"{}\" could not be instantiated",
                        relative_path.display()
                    )
                }
                Error::PackedRef(_) => f.write_str("A packed ref lookup failed"),
                Error::PackedOpen(_) => {
                    f.write_str("Could not open the packed refs buffer when trying to find references.")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::RefnameValidation(err) => Some(err),
                Error::ReadFileContents { source, .. } => Some(source),
                Error::ReferenceCreation { source, .. } => Some(source),
                Error::PackedRef(err) => Some(err),
                Error::PackedOpen(err) => Some(err),
            }
        }
    }

    impl From<crate::name::Error> for Error {
        fn from(err: crate::name::Error) -> Self {
            Error::RefnameValidation(err)
        }
    }

    impl From<packed::find::Error> for Error {
        fn from(err: packed::find::Error) -> Self {
            Error::PackedRef(err)
        }
    }

    impl From<packed::buffer::open::Error> for Error {
        fn from(err: packed::buffer::open::Error) -> Self {
            Error::PackedOpen(err)
        }
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!("this impl is needed to allow passing a known valid partial path as parameter")
        }
    }
}
