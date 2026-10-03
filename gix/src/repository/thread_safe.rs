mod access {
    impl crate::ThreadSafeRepository {
        /// Add thread-local state to an easy-to-use thread-local repository for the most convenient API.
        pub fn to_thread_local(&self) -> crate::Repository {
            self.into()
        }
    }
}

mod location {

    impl crate::ThreadSafeRepository {
        /// The path to the `.git` directory itself, or equivalent if this is a bare repository.
        pub fn path(&self) -> &std::path::Path {
            self.git_dir()
        }

        /// Return the path to the repository itself, containing objects, references, configuration, and more.
        ///
        /// Synonymous to [`path()`][crate::ThreadSafeRepository::path()].
        pub fn git_dir(&self) -> &std::path::Path {
            self.refs.git_dir()
        }

        /// Return the path to the working directory if this is not a bare repository.
        pub fn work_dir(&self) -> Option<&std::path::Path> {
            self.work_tree.as_deref()
        }

        /// Return the path to the directory containing all objects.
        pub fn objects_dir(&self) -> &std::path::Path {
            self.objects.path()
        }
    }
}

mod impls {
    impl std::fmt::Debug for crate::ThreadSafeRepository {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Repository(git = \"{}\", working_tree: {:?}",
                self.git_dir().display(),
                self.work_tree
            )
        }
    }

    impl PartialEq<crate::ThreadSafeRepository> for crate::ThreadSafeRepository {
        fn eq(&self, other: &crate::ThreadSafeRepository) -> bool {
            self.git_dir() == other.git_dir() && self.work_tree == other.work_tree
        }
    }
}
