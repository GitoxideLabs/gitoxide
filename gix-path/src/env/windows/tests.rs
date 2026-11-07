mod system_prefix {
    use super::super::system_prefix_from_git_launcher;
    use super::super::{system_prefix_from_core_dir, system_prefix_from_exepath_var};
    use crate::env::tests::CurrentDir;
    use serial_test::serial;
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
    };

    #[test]
    fn core_dir_keeps_the_runtime_prefix() {
        for (core_dir, prefix) in [
            ("C:/Git/mingw64/libexec/git-core", "C:/Git/mingw64"),
            ("C:/Git/ucrt64/libexec/git-core", "C:/Git/ucrt64"),
            ("C:/Git/mingw32/libexec/git-core", "C:/Git/mingw32"),
            ("C:/Git/clangarm64/libexec/git-core", "C:/Git/clangarm64"),
            ("C:/msys64/usr/lib/git-core", "C:/msys64/usr"),
            ("C:/libexec/Git/ucrt64/libexec/git-core", "C:/libexec/Git/ucrt64"),
        ] {
            assert_eq!(
                system_prefix_from_core_dir(|| Some(Path::new(core_dir))),
                Some(PathBuf::from(prefix)),
                "the runtime prefix is the directory immediately above Git's libexec directory"
            );
        }
    }

    #[test]
    fn core_dir_without_a_runtime_prefix() {
        assert_eq!(system_prefix_from_core_dir(|| None), None);
        for core_dir in ["C:/Git/ucrt64/bin", "libexec/git-core"] {
            assert_eq!(
                system_prefix_from_core_dir(|| Some(Path::new(core_dir))),
                None,
                "a recognizable, nonempty prefix is required"
            );
        }
    }

    #[test]
    fn config_is_next_to_the_runtime_prefix() {
        for runtime in ["mingw64", "ucrt64", "mingw32", "clangarm64", "usr"] {
            assert_eq!(
                super::super::config_path_from_system_prefix(&Path::new("C:/Git").join(runtime)),
                PathBuf::from("C:/Git/etc/gitconfig"),
                "Git for Windows builds ETC_GITCONFIG as ../etc/gitconfig"
            );
        }
    }

    fn if_exepath(key: &str, value: impl Into<OsString>) -> Option<OsString> {
        match key {
            "EXEPATH" => Some(value.into()),
            _ => None,
        }
    }

    struct ExePath {
        _tempdir: tempfile::TempDir,
        path: PathBuf,
    }

    impl ExePath {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("can create new temporary directory");

            // This is just `tempdir.path()` unless it is relative, in which case it is resolved.
            let path = std::env::current_dir()
                .expect("can get current directory")
                .join(tempdir.path());

            Self {
                _tempdir: tempdir,
                path,
            }
        }

        fn create_subdir(&self, name: &str) -> PathBuf {
            let child = self.path.join(name);
            std::fs::create_dir(&child).expect("can create subdirectory");
            child
        }

        fn create_separate_subdirs(&self, names: &[&str]) {
            for name in names {
                self.create_subdir(name);
            }
        }

        fn create_separate_regular_files(&self, names: &[&str]) {
            for name in names {
                std::fs::File::create_new(self.path.join(name)).expect("can create new file");
            }
        }

        fn var_os_func(&self, key: &str) -> Option<OsString> {
            if_exepath(key, self.path.as_os_str())
        }
    }

    #[test]
    fn exepath_unset() {
        let outcome = system_prefix_from_exepath_var(|_| None);
        assert_eq!(outcome, None);
    }

    #[test]
    #[serial]
    fn exepath_no_relevant_subdir() {
        for names in [&[][..], &["clang64"][..]] {
            let exepath = ExePath::new();
            exepath.create_separate_subdirs(names);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(outcome, None);
        }
    }

    #[test]
    #[serial]
    fn exepath_unambiguous_subdir() {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let exepath = ExePath::new();
            let subdir = exepath.create_subdir(name);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(outcome, Some(subdir), "{name} is an unambiguous Git for Windows prefix");
        }
    }

    #[test]
    #[serial]
    fn exepath_unambiguous_subdir_beside_strange_files() {
        for (dirname, filenames) in [
            ("clangarm64", ["ucrt64", "mingw64", "mingw32"]),
            ("ucrt64", ["clangarm64", "mingw64", "mingw32"]),
            ("mingw64", ["clangarm64", "ucrt64", "mingw32"]),
            ("mingw32", ["clangarm64", "ucrt64", "mingw64"]),
        ] {
            let exepath = ExePath::new();
            let subdir = exepath.create_subdir(dirname);
            exepath.create_separate_regular_files(&filenames);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(
                outcome,
                Some(subdir),
                "only directories can be Git for Windows prefixes"
            );
        }
    }

    #[test]
    #[serial]
    fn exepath_ambiguous_subdir() {
        for names in [
            &["ucrt64", "mingw64"][..],
            &["ucrt64", "mingw32"][..],
            &["clangarm64", "ucrt64"][..],
            &["mingw32", "mingw64"][..],
            &["mingw32", "clangarm64"][..],
            &["mingw64", "clangarm64"][..],
            &["mingw32", "mingw64", "clangarm64"][..],
            &["clangarm64", "ucrt64", "mingw64", "mingw32"][..],
        ] {
            let exepath = ExePath::new();
            exepath.create_separate_subdirs(names);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(
                outcome, None,
                "multiple prefixes require querying Git to disambiguate {names:?}"
            );
        }
    }

    #[test]
    #[serial]
    fn git_launcher_in_cmd_with_unambiguous_subdir() {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let root = ExePath::new();
            let subdir = root.create_subdir(name);
            let cmd = root.create_subdir("cmd");
            std::fs::File::create_new(cmd.join("git.exe")).expect("can create the sibling launcher");
            for cmd in ["cmd", "CMD", "bin", "BIN"] {
                let outcome = system_prefix_from_git_launcher(&root.path.join(cmd).join("git.exe"));
                assert_eq!(
                    outcome.as_ref(),
                    Some(&subdir),
                    "{cmd}/git.exe belongs to the {name} prefix"
                );
            }
        }
    }

    #[test]
    #[serial]
    fn git_launcher_elsewhere_is_not_recognized() {
        let root = ExePath::new();
        root.create_subdir("mingw64");
        for parent in ["bin", "shims", "mingw64/bin", "usr/bin"] {
            let outcome = system_prefix_from_git_launcher(&root.path.join(parent).join("git.exe"));
            assert_eq!(outcome, None, "{parent} isn't Git for Windows' launcher directory");
        }
    }

    #[test]
    #[serial]
    fn git_launcher_with_ambiguous_or_no_subdir() {
        for names in [
            &[][..],
            &["clang64"][..],
            &["ucrt64", "mingw64"][..],
            &["mingw32", "mingw64"][..],
        ] {
            let root = ExePath::new();
            root.create_separate_subdirs(names);
            let outcome = system_prefix_from_git_launcher(&root.path.join("cmd/git.exe"));
            assert_eq!(outcome, None, "{names:?} needs querying Git");
        }
    }

    #[test]
    fn git_launcher_must_be_absolute() {
        for executable in ["git.exe", "cmd/git.exe", "./cmd/git.exe"] {
            assert_eq!(
                system_prefix_from_git_launcher(Path::new(executable)),
                None,
                "{executable} doesn't say where the installation is"
            );
        }
    }

    #[test]
    fn runtime_binary_identifies_its_prefix_lexically() {
        for runtime in ["clangarm64", "ucrt64", "mingw64", "mingw32", "UCRT64"] {
            let prefix = Path::new("C:/Git").join(runtime);
            assert_eq!(
                super::super::system_prefix_from_git_executable(&prefix.join("bin/git.exe")),
                Some(prefix),
                "the selected runtime binary identifies its own prefix"
            );
        }
        for executable in [
            "ucrt64/bin/git.exe",
            "C:/Git/unknown/bin/git.exe",
            "C:/Git/ucrt64/shims/git.exe",
        ] {
            assert_eq!(
                super::super::system_prefix_from_git_executable(Path::new(executable)),
                None,
                "relative paths and unrecognized layouts cannot identify a runtime"
            );
        }
    }

    #[test]
    #[serial]
    fn exepath_empty_string() {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let exepath = ExePath::new();
            exepath.create_subdir(name);
            let _cwd = CurrentDir::set(&exepath.path).expect("can change to test dir");
            let outcome = system_prefix_from_exepath_var(|key| if_exepath(key, ""));
            assert_eq!(outcome, None);
        }
    }

    #[test]
    #[serial]
    fn exepath_nonempty_relative() -> gix_testtools::TestResult {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let grandparent = tempfile::tempdir()?;
            let parent = grandparent.path().canonicalize()?.join("dir");
            std::fs::create_dir_all(parent.join(name))?;
            let _cwd = CurrentDir::set(grandparent.path())?;
            let outcome = system_prefix_from_exepath_var(|key| if_exepath(key, "dir"));
            assert_eq!(outcome, None);
        }
        Ok(())
    }
}

mod locations {
    use std::{
        ffi::{OsStr, OsString, c_void},
        io::ErrorKind,
        os::windows::ffi::OsStringExt,
        path::{Path, PathBuf},
    };

    use windows::{
        Win32::{
            System::{
                Com::CoTaskMemFree,
                Threading::{GetCurrentProcess, IsWow64Process},
            },
            UI::Shell::{
                FOLDERID_LocalAppData, FOLDERID_ProgramFiles, FOLDERID_ProgramFilesX86, FOLDERID_UserProgramFiles,
                KF_FLAG_DEFAULT, KF_FLAG_DONT_VERIFY, KNOWN_FOLDER_FLAG, SHGetKnownFolderPath,
            },
        },
        core::{BOOL, GUID, PWSTR, Result as WindowsResult},
    };
    use winreg::{
        RegKey,
        enums::{HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE},
    };

    macro_rules! var_os_stub {
        { $($name:expr => $value:expr),* $(,)? } => {
            |key| {
                match key {
                    $(
                        $name => Some(OsString::from($value)),
                    )*
                    _ => None,
                }
            }
        }
    }

    macro_rules! locations_from {
        ($($name:expr => $value:expr),* $(,)?) => {
            super::super::locations_under_program_files(var_os_stub! {
                $(
                    $name => $value,
                )*
            })
        }
    }

    macro_rules! pathbuf_vec {
        [$($path:expr),* $(,)?] => {
            vec![$(
                PathBuf::from($path),
            )*]
        }
    }

    #[test]
    fn locations_under_program_files_no_vars() {
        assert_eq!(locations_from!(), Vec::<PathBuf>::new());
    }

    #[test]
    fn locations_under_program_files_global_only_ordinary_values_current_var_only() {
        assert_eq!(
            locations_from!(
                "ProgramFiles" => r"C:\Program Files",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"C:\Program Files\Git\clangarm64\bin",
                    r"C:\Program Files\Git\ucrt64\bin",
                    r"C:\Program Files\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![r"C:\Program Files\Git\mingw32\bin"]
            },
        );
    }

    #[test]
    fn locations_under_program_files_global_only_ordinary_values_all_vars() {
        assert_eq!(
            locations_from!(
                "ProgramFiles" => {
                    if cfg!(target_pointer_width = "64") {
                        r"C:\Program Files"
                    } else {
                        r"C:\Program Files (x86)"
                    }
                },
                "ProgramFiles(x86)" => r"C:\Program Files (x86)",
                "ProgramW6432" => r"C:\Program Files",
            ),
            pathbuf_vec![
                r"C:\Program Files\Git\clangarm64\bin",
                r"C:\Program Files\Git\ucrt64\bin",
                r"C:\Program Files\Git\mingw64\bin",
                r"C:\Program Files (x86)\Git\mingw32\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_global_only_strange_values_all_vars_distinct() {
        assert_eq!(
            locations_from!(
                "ProgramFiles" => r"X:\cur\rent",
                "ProgramFiles(x86)" => r"Y:\nar\row",
                "ProgramW6432" => r"Z:\wi\de",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\clangarm64\bin",
                    r"X:\cur\rent\Git\ucrt64\bin",
                    r"X:\cur\rent\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\mingw32\bin",
                ]
            },
        );
    }

    #[test]
    fn locations_under_program_files_global_only_strange_values_64bit_var_only() {
        assert_eq!(
            locations_from!(
                "ProgramW6432" => r"Z:\wi\de",
            ),
            pathbuf_vec![
                r"Z:\wi\de\Git\clangarm64\bin",
                r"Z:\wi\de\Git\ucrt64\bin",
                r"Z:\wi\de\Git\mingw64\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_global_only_strange_values_all_vars_path_cruft() {
        assert_eq!(
            locations_from!(
                "ProgramFiles" => r"Z:/wi//de/",
                "ProgramFiles(x86)" => r"Y:/\nar\/row",
                "ProgramW6432" => r"Z:\wi\.\de",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"Z:\wi\de\Git\mingw32\bin",
                ]
            },
        );
    }

    #[test]
    fn locations_under_program_files_global_only_strange_values_some_relative() {
        assert_eq!(
            locations_from!(
                "ProgramFiles" => r"foo\bar",
                "ProgramFiles(x86)" => r"\\host\share\subdir",
                "ProgramW6432" => r"",
            ),
            pathbuf_vec![r"\\host\share\subdir\Git\mingw32\bin"],
        );
    }

    #[test]
    fn locations_under_program_files_local_only_ordinary_value() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"C:\Users\alice\AppData\Local",
            ),
            pathbuf_vec![
                r"C:\Users\alice\AppData\Local\Programs\Git\clangarm64\bin",
                r"C:\Users\alice\AppData\Local\Programs\Git\ucrt64\bin",
                r"C:\Users\alice\AppData\Local\Programs\Git\mingw64\bin",
                r"C:\Users\alice\AppData\Local\Programs\Git\mingw32\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_local_only_strange_value_path_cruft() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"\\.\Q:\Documents and Settings/bob\/weird\.\sub//dir",
            ),
            pathbuf_vec![
                r"\\.\Q:\Documents and Settings\bob\weird\sub\dir\Programs\Git\clangarm64\bin",
                r"\\.\Q:\Documents and Settings\bob\weird\sub\dir\Programs\Git\ucrt64\bin",
                r"\\.\Q:\Documents and Settings\bob\weird\sub\dir\Programs\Git\mingw64\bin",
                r"\\.\Q:\Documents and Settings\bob\weird\sub\dir\Programs\Git\mingw32\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_local_only_strange_value_empty() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => "",
            ),
            Vec::<PathBuf>::new(),
        );
    }

    #[test]
    fn locations_under_program_files_local_only_strange_value_relative_nonempty() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"AppData\Local",
            ),
            Vec::<PathBuf>::new(),
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_ordinary_values_limited_vars() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"C:\Users\alice\AppData\Local",
                "ProgramFiles" => r"C:\Program Files",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"C:\Users\alice\AppData\Local\Programs\Git\clangarm64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\ucrt64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw32\bin",
                    r"C:\Program Files\Git\clangarm64\bin",
                    r"C:\Program Files\Git\ucrt64\bin",
                    r"C:\Program Files\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"C:\Users\alice\AppData\Local\Programs\Git\clangarm64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\ucrt64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw32\bin",
                    r"C:\Program Files\Git\mingw32\bin",
                ]
            },
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_ordinary_values_all_vars() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"C:\Users\bob\AppData\Local",
                "ProgramFiles" => {
                    if cfg!(target_pointer_width = "64") {
                        r"C:\Program Files"
                    } else {
                        r"C:\Program Files (x86)"
                    }
                },
                "ProgramFiles(x86)" => r"C:\Program Files (x86)",
                "ProgramW6432" => r"C:\Program Files",
            ),
            pathbuf_vec![
                r"C:\Users\bob\AppData\Local\Programs\Git\clangarm64\bin",
                r"C:\Users\bob\AppData\Local\Programs\Git\ucrt64\bin",
                r"C:\Users\bob\AppData\Local\Programs\Git\mingw64\bin",
                r"C:\Users\bob\AppData\Local\Programs\Git\mingw32\bin",
                r"C:\Program Files\Git\clangarm64\bin",
                r"C:\Program Files\Git\ucrt64\bin",
                r"C:\Program Files\Git\mingw64\bin",
                r"C:\Program Files (x86)\Git\mingw32\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_strange_values_all_vars_distinct() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"W:\us\er",
                "ProgramFiles" => r"X:\cur\rent",
                "ProgramFiles(x86)" => r"Y:\nar\row",
                "ProgramW6432" => r"Z:\wi\de",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"W:\us\er\Programs\Git\clangarm64\bin",
                    r"W:\us\er\Programs\Git\ucrt64\bin",
                    r"W:\us\er\Programs\Git\mingw64\bin",
                    r"W:\us\er\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\clangarm64\bin",
                    r"X:\cur\rent\Git\ucrt64\bin",
                    r"X:\cur\rent\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"W:\us\er\Programs\Git\clangarm64\bin",
                    r"W:\us\er\Programs\Git\ucrt64\bin",
                    r"W:\us\er\Programs\Git\mingw64\bin",
                    r"W:\us\er\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\mingw32\bin",
                ]
            },
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_strange_values_limited_64bit_var() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"W:\us\er",
                "ProgramW6432" => r"Z:\wi\de",
            ),
            pathbuf_vec![
                r"W:\us\er\Programs\Git\clangarm64\bin",
                r"W:\us\er\Programs\Git\ucrt64\bin",
                r"W:\us\er\Programs\Git\mingw64\bin",
                r"W:\us\er\Programs\Git\mingw32\bin",
                r"Z:\wi\de\Git\clangarm64\bin",
                r"Z:\wi\de\Git\ucrt64\bin",
                r"Z:\wi\de\Git\mingw64\bin",
            ],
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_strange_values_crufty_cross_clash() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => r"Y:\nar\row",
                "ProgramFiles" => r"Z:/wi//de/",
                "ProgramFiles(x86)" => r"Y:/\nar/row\Programs",
                "ProgramW6432" => r"Z:\wi\.\de",
            ),
            if cfg!(target_pointer_width = "64") {
                pathbuf_vec![
                    r"Y:\nar\row\Programs\Git\clangarm64\bin",
                    r"Y:\nar\row\Programs\Git\ucrt64\bin",
                    r"Y:\nar\row\Programs\Git\mingw64\bin",
                    r"Y:\nar\row\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Y:\nar\row\Programs\Git\clangarm64\bin",
                    r"Y:\nar\row\Programs\Git\ucrt64\bin",
                    r"Y:\nar\row\Programs\Git\mingw64\bin",
                    r"Y:\nar\row\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\ucrt64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Z:\wi\de\Git\mingw32\bin",
                ]
            },
        );
    }

    #[test]
    fn locations_under_program_files_local_and_global_strange_values_some_relative() {
        assert_eq!(
            locations_from!(
                "LocalAppData" => "dir",
                "ProgramFiles" => r"foo\bar",
                "ProgramFiles(x86)" => r"\\host\share\subdir",
                "ProgramW6432" => r"",
            ),
            pathbuf_vec![r"\\host\share\subdir\Git\mingw32\bin"],
        );
    }

    /// Owner of a null-terminated `PWSTR` that must be freed with `CoTaskMemFree`.
    struct CoStr {
        pwstr: PWSTR,
    }

    impl CoStr {
        /// SAFETY: The caller must ensure `pwstr` is a non-null pointer to the beginning of a
        /// null-terminated (zero-codepoint-terminated) wide string releasable by `CoTaskMemFree`.
        unsafe fn new(pwstr: PWSTR) -> Self {
            Self { pwstr }
        }

        fn to_os_string(&self) -> OsString {
            // SAFETY: We know `pwstr` is derefrenceable and the string is null-terminated.
            let wide = unsafe { self.pwstr.as_wide() };
            OsString::from_wide(wide)
        }
    }

    impl Drop for CoStr {
        fn drop(&mut self) {
            // SAFETY: `pwstr` is allowed to be passed to `CoTaskMemFree`. (We happen to know it's
            // non-null, but `CoTaskMemFree` permits null as well, so the cast is doubly safe.)
            unsafe { CoTaskMemFree(Some(self.pwstr.as_ptr().cast::<c_void>())) };
        }
    }

    fn get_known_folder_path_with_flag(id: GUID, flag: KNOWN_FOLDER_FLAG) -> WindowsResult<PathBuf> {
        // SAFETY: `SHGetKnownFolderPath` in the `windows` crate wraps the API function and returns
        // a non-null pointer to a null-terminated wide string, or an error, not a null pointer.
        // As in the wrapped API function, the pointer it returns can be passed to `CoTaskMemFree`.
        let costr = unsafe { CoStr::new(SHGetKnownFolderPath(&id, flag, None)?) };
        Ok(PathBuf::from(costr.to_os_string()))
    }

    #[derive(Clone, Copy, Debug)]
    enum PlatformBitness {
        Is32on32,
        Is32on64,
        Is64on64,
    }

    impl PlatformBitness {
        fn current() -> WindowsResult<Self> {
            // Ordinarily, we would check the target pointer width first to avoid doing extra work,
            // because if this is a 64-bit executable then the operating system is 64-bit. But this
            // is for the test suite, and doing it this way allows problems to be caught earlier if
            // a change made on a 64-bit development machine breaks the IsWow64Process() call.
            let mut wow64process = BOOL::default();
            unsafe {
                // SAFETY: `GetCurrentProcess` always succeeds, and the handle it returns is a
                // valid process handle to pass to `IsWow64Process`.
                IsWow64Process(GetCurrentProcess(), &mut wow64process)?;
            }

            let platform_bitness = if wow64process.as_bool() {
                Self::Is32on64
            } else if cfg!(target_pointer_width = "32") {
                Self::Is32on32
            } else {
                Self::Is64on64
            };
            Ok(platform_bitness)
        }
    }

    fn ends_with_case_insensitive(full_text: &OsStr, literal_pattern: &str) -> Option<bool> {
        let folded_text = full_text.to_str()?.to_lowercase();
        let folded_pattern = literal_pattern.to_lowercase();
        Some(folded_text.ends_with(&folded_pattern))
    }

    /// The most common program files paths, as they are available in this environment.
    ///
    /// This omits the global 32-bit ARM program files directory, because Git for Windows is never
    /// installed there.
    #[derive(Clone, Debug)]
    struct ProgramFilesPaths {
        /// The program files directory used for whatever architecture this program was built for.
        current: PathBuf,

        /// The 32-bit x86 program files directory regardless of the architecture of the program.
        ///
        /// If Rust gains Windows targets like ARMv7 where this is unavailable, this could fail.
        x86: PathBuf,

        /// The 64-bit program files directory if there is one.
        ///
        /// This is present on x64 (AMD64) and also ARM64 systems. On an ARM64 system, ARM64 and
        /// AMD64 programs use the same program files directory while 32-bit x86 and 32-bit ARM
        /// programs use two others. Only a 32-bit system has no 64-bit program files directory.
        maybe_64bit: Option<PathBuf>,

        /// The per-user `Programs` subdirectory of the user's local application data directory.
        user: PathBuf,
    }

    impl ProgramFilesPaths {
        /// Get the four most common kinds of program files paths without environment variables.
        ///
        /// The idea here is to obtain this information, which the `alternative_locations()` unit
        /// test uses to learn the expected alternative locations, without duplicating *any* of the
        /// approach used for `locations_under_program_files`, so it can be used to test that. The approach
        /// here is also more reliable than using environment variables, but it is a bit more
        /// complex, and it requires either additional dependencies or the use of unsafe code.
        ///
        /// This gets `pf_user`, `pf_current`, and `pf_x86` by the [known folders][known-folders]
        /// system. But it gets `maybe_pf_64bit` from the registry, as the corresponding known
        /// folder is not available to 32-bit processes. See the [`KNOWNFOLDDERID`][knownfolderid]
        /// documentation.
        ///
        /// If in the future the implementation of `locations_under_program_files` uses these techniques,
        /// then this function can be changed to use environment variables and renamed accordingly.
        ///
        /// [known-folders]: https://learn.microsoft.com/en-us/windows/win32/shell/known-folders
        /// [knownfolderid]: https://learn.microsoft.com/en-us/windows/win32/shell/knownfolderid#remarks
        fn obtain_envlessly() -> Self {
            let pf_current = get_known_folder_path_with_flag(FOLDERID_ProgramFiles, KF_FLAG_DEFAULT)
                .expect("The process architecture specific program files folder is always available");

            let pf_x86 = get_known_folder_path_with_flag(FOLDERID_ProgramFilesX86, KF_FLAG_DEFAULT)
                .expect("The x86 program files folder will in practice always be available");

            let maybe_pf_64bit = RegKey::predef(HKEY_LOCAL_MACHINE)
                .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows\CurrentVersion", KEY_QUERY_VALUE)
                .expect("The `CurrentVersion` registry key exists and allows reading")
                .get_value::<OsString, _>("ProgramW6432Dir")
                .map(PathBuf::from)
                .inspect_err(|error| {
                    assert_eq!(error.kind(), ErrorKind::NotFound);
                })
                .ok();

            let pf_user = get_known_folder_path_with_flag(FOLDERID_UserProgramFiles, KF_FLAG_DONT_VERIFY)
                .expect("The path where the user's local Programs folder is, or would be created, is known");

            Self {
                current: pf_current,
                x86: pf_x86,
                maybe_64bit: maybe_pf_64bit,
                user: pf_user,
            }
        }

        /// Check that the paths we got for testing are reasonable.
        ///
        /// This checks that `obtain_envlessly()` returned paths that are likely to be correct and
        /// that satisfy the most important properties based on the current system and process.
        fn validated(self) -> Self {
            self.validate_global_folders();
            self.validate_user_folder();
            self
        }

        fn validate_global_folders(&self) {
            match PlatformBitness::current().expect("Process and system 'bitness' should be available") {
                PlatformBitness::Is32on32 => {
                    assert_eq!(
                        self.current.as_os_str(),
                        self.x86.as_os_str(),
                        "Our program files path is exactly identical to the 32-bit one.",
                    );
                    for trailing_arch in [" (x86)", " (Arm)"] {
                        let is_adorned = ends_with_case_insensitive(self.current.as_os_str(), trailing_arch)
                            .expect("Assume the test system's important directories are valid Unicode");
                        assert!(
                            !is_adorned,
                            "The 32-bit program files directory name on a 32-bit system mentions no architecture.",
                        );
                    }
                    assert_eq!(
                        self.maybe_64bit, None,
                        "A 32-bit system has no 64-bit program files directory.",
                    );
                }
                PlatformBitness::Is32on64 => {
                    assert_eq!(
                        self.current.as_os_str(),
                        self.x86.as_os_str(),
                        "Our program files path is exactly identical to the 32-bit one.",
                    );
                    let pf_64bit = self
                        .maybe_64bit
                        .as_ref()
                        .expect("The 64-bit program files directory exists");
                    assert_ne!(
                        &self.x86, pf_64bit,
                        "The 32-bit and 64-bit program files directories have different locations.",
                    );
                }
                PlatformBitness::Is64on64 => {
                    let pf_64bit = self
                        .maybe_64bit
                        .as_ref()
                        .expect("The 64-bit program files directory exists");
                    assert_eq!(
                        self.current.as_os_str(),
                        pf_64bit.as_os_str(),
                        "Our program files path is exactly identical to the 64-bit one.",
                    );
                    assert_ne!(
                        &self.x86, pf_64bit,
                        "The 32-bit and 64-bit program files directories have different locations.",
                    );
                }
            }
        }

        fn validate_user_folder(&self) {
            let expected = get_known_folder_path_with_flag(FOLDERID_LocalAppData, KF_FLAG_DEFAULT)
                .expect("The user's local application data directory is available")
                .join("Programs");
            assert_eq!(
                self.user, expected,
                "The user program files directory is Programs in the local application data directory.",
            );
        }
    }

    /// Architecture-specific Git for Windows paths relative to the user program files directory.
    #[derive(Clone, Debug)]
    struct RelativeUserGitBinPaths<'a> {
        x86: &'a Path,
        x64: &'a Path,
        x64_legacy: &'a Path,
        arm64: &'a Path,
    }

    impl<'a> RelativeUserGitBinPaths<'a> {
        /// Assert that `locations` leads with the given user path prefix, and extract the suffixes.
        fn assert_from(pf_user: &'a Path, locations: &'a [PathBuf]) -> Self {
            match locations {
                [path1, path2, path3, path4, ..] => {
                    let suffix_user_arm64 = path1
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user 64-bit ARM64 path and lists it first");
                    let suffix_user_x64 = path2
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user UCRT64 path and lists it second");
                    let suffix_user_x64_legacy = path3
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user legacy 64-bit x86 path and lists it third");
                    let suffix_user_x86 = path4
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user 32-bit x86 path and lists it fourth");
                    Self {
                        x86: suffix_user_x86,
                        x64: suffix_user_x64,
                        x64_legacy: suffix_user_x64_legacy,
                        arm64: suffix_user_arm64,
                    }
                }
                other => panic!(
                    "{:?} has length {}, so some expected leading user program files paths are absent",
                    other,
                    other.len()
                ),
            }
        }

        /// Assert that suffixes are common Git install locations relative to a program files directory.
        fn assert_architectures(&self) {
            assert_eq!(self.x86, Path::new("Git/mingw32/bin"));
            assert_eq!(self.x64, Path::new("Git/ucrt64/bin"), "UCRT64 precedes legacy x86_64");
            assert_eq!(
                self.x64_legacy,
                Path::new("Git/mingw64/bin"),
                "legacy x86_64 remains available"
            );
            assert_eq!(self.arm64, Path::new("Git/clangarm64/bin"));
        }
    }

    /// Architecture-specific Git for Windows paths relative to global program files directories.
    #[derive(Clone, Debug)]
    struct RelativeGlobalGitBinPaths<'a> {
        x86: &'a Path,
        maybe_x64: Option<&'a Path>,
        maybe_x64_legacy: Option<&'a Path>,
        maybe_arm64: Option<&'a Path>,
    }

    impl<'a> RelativeGlobalGitBinPaths<'a> {
        /// Assert that `locations` trails with the given global path prefixes, and extract the suffixes.
        fn assert_from(pf: &'a ProgramFilesPaths, locations: &'a [PathBuf]) -> Self {
            match locations {
                [_, _, _, _, path5, path6, path7, path8] => {
                    let prefix_64bit = pf
                        .maybe_64bit
                        .as_ref()
                        .expect("It gives 8 paths only if some global paths can be 64-bit");
                    let suffix_global_arm64 = path5
                        .strip_prefix(prefix_64bit)
                        .expect("It gives a global 64-bit ARM64 path and lists it fifth");
                    let suffix_global_x64 = path6
                        .strip_prefix(prefix_64bit)
                        .expect("It gives a global UCRT64 path and lists it sixth");
                    let suffix_global_x64_legacy = path7
                        .strip_prefix(prefix_64bit)
                        .expect("It gives a global legacy 64-bit x86 path and lists it seventh");
                    let suffix_global_x86 = path8
                        .strip_prefix(&pf.x86)
                        .expect("It gives a global 32-bit path and lists it eighth");
                    Self {
                        x86: suffix_global_x86,
                        maybe_x64: Some(suffix_global_x64),
                        maybe_x64_legacy: Some(suffix_global_x64_legacy),
                        maybe_arm64: Some(suffix_global_arm64),
                    }
                }
                [_, _, _, _, path5] => {
                    assert_eq!(
                        pf.maybe_64bit, None,
                        "It gives 5 paths only if no global paths can be 64-bit.",
                    );
                    let suffix_global_x86 = path5
                        .strip_prefix(&pf.x86)
                        .expect("It gives a global 32-bit path and lists it fifth");
                    Self {
                        x86: suffix_global_x86,
                        maybe_x64: None,
                        maybe_x64_legacy: None,
                        maybe_arm64: None,
                    }
                }
                other => panic!("{:?} has length {}, expected 5 or 8.", other, other.len()),
            }
        }

        /// Assert that suffixes are common Git install locations relative to a program files directory.
        fn assert_architectures(&self) {
            assert_eq!(self.x86, Path::new("Git/mingw32/bin"));

            if let Some(suffix_x64) = self.maybe_x64 {
                assert_eq!(suffix_x64, Path::new("Git/ucrt64/bin"), "UCRT64 precedes legacy x86_64");
            }
            if let Some(suffix_x64_legacy) = self.maybe_x64_legacy {
                assert_eq!(
                    suffix_x64_legacy,
                    Path::new("Git/mingw64/bin"),
                    "legacy x86_64 remains available"
                );
            }
            if let Some(suffix_arm64) = self.maybe_arm64 {
                assert_eq!(suffix_arm64, Path::new("Git/clangarm64/bin"));
            }
        }
    }

    /// Architecture-specific Git for Windows paths relative to particular program files directories.
    #[derive(Clone, Debug)]
    struct RelativeGitBinPaths<'a> {
        global: RelativeGlobalGitBinPaths<'a>,
        user: RelativeUserGitBinPaths<'a>,
    }

    impl<'a> RelativeGitBinPaths<'a> {
        /// Assert that `locations` has the given path prefixes, and extract the suffixes.
        fn assert_from(pf: &'a ProgramFilesPaths, locations: &'a [PathBuf]) -> Self {
            let user = RelativeUserGitBinPaths::assert_from(&pf.user, locations);
            let global = RelativeGlobalGitBinPaths::assert_from(pf, locations);
            Self { global, user }
        }

        /// Assert that global and user suffixes (relative subdirectories) are common Git install locations.
        fn assert_architectures(&self) {
            self.global.assert_architectures();
            self.user.assert_architectures();
        }
    }

    #[test]
    fn path_hits_do_not_inspect_fallbacks() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _cwd = gix_testtools::set_current_dir(&fixture)?;
        let bin = fixture.join("path");
        for path in [bin.as_os_str(), OsStr::new("path")] {
            let executable = super::super::find_executable(Some(path), || {
                panic!("a PATH hit must not inspect alternative locations")
            })
            .expect("the fixture has Git on PATH");
            assert_eq!(
                executable.path,
                bin.join("git.exe"),
                "retain an absolute path for companion discovery, including relative PATH entries"
            );
            assert_eq!(
                executable.invocation(),
                Path::new("git.exe"),
                "PATH hits use a short invocation"
            );
        }

        let missing = fixture.join("missing");
        for path in [None, Some(missing.as_os_str())] {
            let fallback = super::super::find_executable(path, || Some(bin.join("git.exe")))
                .expect("an absent or unsuccessful PATH reaches the fallback");
            assert_eq!(
                fallback.invocation(),
                bin.join("git.exe"),
                "fallback invocations are absolute"
            );
        }
        Ok(())
    }

    #[test]
    fn alternative_sources_are_lazy() {
        use std::cell::Cell;

        let lookups = Cell::new(0);
        let mut locations = super::super::alternative_locations(
            |key| {
                lookups.set(lookups.get() + 1);
                match key {
                    "ProgramFiles" => Some("C:/early".into()),
                    "LocalAppData" | "ProgramW6432" | "ProgramFiles(x86)" => None,
                    _ => panic!("package roots must not be inspected before Program Files is exhausted"),
                }
            },
            || -> Vec<PathBuf> { panic!("registry discovery must wait for earlier sources") },
            || -> Vec<PathBuf> { panic!("MSYS2 registrations must remain untouched") },
        );
        assert_eq!(
            lookups.get(),
            0,
            "constructing the iterator does not inspect any source"
        );
        assert!(
            locations
                .next()
                .expect("Program Files has candidates")
                .starts_with("C:/early"),
            "Program Files remains the first fallback"
        );

        let mut locations = super::super::alternative_locations(
            |key| (key == "CONDA_PREFIX").then(|| "C:/conda".into()),
            || -> Vec<PathBuf> { panic!("fixed package paths must precede registry queries") },
            || -> Vec<PathBuf> { panic!("fixed package paths must precede MSYS2 enumeration") },
        );
        assert_eq!(
            locations.next(),
            Some("C:/conda/Library/bin".into()),
            "even less common fixed paths precede registry queries"
        );

        let mut locations = super::super::alternative_locations(
            |_| None,
            || [PathBuf::from("C:/registered/cmd")],
            || -> Vec<PathBuf> { panic!("native Git must not enumerate MSYS2 registrations") },
        );
        assert_eq!(
            locations.next(),
            Some("C:/registered/cmd".into()),
            "direct installer registrations precede broader enumeration"
        );

        let registry_calls = Cell::new(0);
        let mut locations = super::super::alternative_locations(
            |_| None,
            std::iter::empty::<PathBuf>,
            || {
                registry_calls.set(registry_calls.get() + 1);
                [PathBuf::from("C:/registered-msys/usr/bin")]
            },
        );
        assert_eq!(
            registry_calls.get(),
            0,
            "registry discovery is deferred until iteration"
        );
        assert_eq!(
            locations.next(),
            Some("C:/registered-msys/usr/bin".into()),
            "MSYS2 registry discovery remains the last fallback"
        );
        assert_eq!(locations.next(), None, "all sources are exhausted");
        assert_eq!(registry_calls.get(), 1, "the registry provider runs only once");
    }

    #[test]
    #[serial_test::serial]
    fn scoop_locations_are_lazy() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        let locations = super::super::locations_under_scoop(|key| match key {
            "SCOOP" => Some(fixture.join("custom").into_os_string()),
            _ => panic!("global and default Scoop roots must remain untouched"),
        });
        assert_eq!(
            locations.into_iter().next(),
            Some(fixture.join("custom/apps/git/current/cmd")),
            "a configured user installation precedes global Scoop roots"
        );
        assert!(
            super::super::locations_under_scoop_without_junctions(|key| {
                (key == "SCOOP").then(|| fixture.join("custom").into_os_string())
            })
            .next()
            .is_none(),
            "a usable current installation excludes retained versions from the scan fallback"
        );
        Ok(())
    }

    #[test]
    fn explicit_installation_precedes_package_managers() {
        let mut locations = super::super::locations_under_package_managers(|key| match key {
            "GIT_INSTALL_ROOT" => Some(r"D:\portable\git".into()),
            _ => panic!("package managers must not be inspected before the explicit root is exhausted"),
        });
        assert_eq!(
            locations.next(),
            Some(PathBuf::from(r"D:\portable\git\cmd")),
            "an explicit Git root precedes package-manager fallbacks"
        );
    }

    #[test]
    fn native_paths_do_not_probe_msys() {
        for path in [
            r"C:\Git\etc\gitconfig",
            "C:/Git/ucrt64/libexec/git-core",
            r"\\server\git\etc",
            "relative",
        ] {
            assert_eq!(
                super::super::native_path_with(Path::new(path), || {
                    panic!("native and relative paths must not trigger executable discovery")
                }),
                Some(PathBuf::from(path)),
                "only rooted Unix paths require an MSYS runtime"
            );
        }
    }

    #[test]
    fn scoop_locations_respect_configured_roots() {
        let locations: Vec<_> = super::super::locations_under_scoop(var_os_stub! {
            "SCOOP" => r"D:\custom",
            "SCOOP_GLOBAL" => r"E:\shared",
            "USERPROFILE" => r"C:\Users\alice",
            "ProgramData" => r"C:\ProgramData",
        })
        .collect();
        assert_eq!(
            locations,
            pathbuf_vec![r"D:\custom\apps\git\current\cmd", r"E:\shared\apps\git\current\cmd",],
            "explicit installation roots take precedence over Scoop's default roots"
        );
    }

    #[test]
    fn scoop_locations_reject_relative_roots() {
        for invalid in ["", "relative", r"C:relative", r"\rooted"] {
            let variables = |_: &str| Some(OsString::from(invalid));
            let mut locations = super::super::locations_under_scoop(variables)
                .chain(super::super::locations_under_scoop_without_junctions(variables));
            assert!(
                locations.next().is_none(),
                "{invalid:?} must not cause discovery relative to the working directory"
            );
        }
    }

    #[test]
    fn alternative_locations() {
        // Obtain program files directory paths by other means and check that they seem correct.
        let pf = ProgramFilesPaths::obtain_envlessly().validated();

        // Check that the first fallback corresponds to them, with the correct subdirectories.
        let locations = super::super::locations_under_program_files(|key| std::env::var_os(key));
        RelativeGitBinPaths::assert_from(&pf, &locations).assert_architectures();
    }

    #[test]
    fn package_manager_locations_reject_relative_roots() {
        for invalid in ["", "relative", r"C:relative", r"\rooted"] {
            let mut locations = super::super::locations_under_package_managers(|_| Some(invalid.into()));
            assert!(
                locations.next().is_none(),
                "{invalid:?} must not enable discovery relative to the working directory"
            );
        }
    }

    #[test]
    #[serial_test::serial]
    fn package_fallbacks_use_isolated_roots() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        let missing = fixture.join("missing");
        for (variable, root, expected) in [
            ("SCOOP", "no-junction", "no-junction/apps/git/2.50.0/cmd/git.exe"),
            ("ChocolateyToolsLocation", "chocolatey", "chocolatey/git/cmd/git.exe"),
            ("CONDA_PREFIX", "conda", "conda/Library/bin/git.exe"),
            ("LocalAppData", "winget", "winget/Microsoft/WinGet/Links/git.exe"),
            ("ProgramW6432", "machine", "machine/WinGet/Links/git.exe"),
            ("GIT_INSTALL_ROOT", "msys", "msys/usr/bin/git.exe"),
            ("USERPROFILE", "user", "user/scoop/apps/git/current/cmd/git.exe"),
            ("SCOOP", "custom", "custom/apps/git/current/cmd/git.exe"),
            ("ProgramData", "global", "global/scoop/apps/git/current/cmd/git.exe"),
            ("SCOOP_GLOBAL", "custom", "custom/apps/git/current/cmd/git.exe"),
            ("GIT_INSTALL_ROOT", "explicit", "explicit/cmd/git.exe"),
        ] {
            for path in [None, Some(missing.as_os_str())] {
                let executable = super::super::find_executable(path, || {
                    super::super::alternative_locations(
                        |key| (key == variable).then(|| fixture.join(root).into_os_string()),
                        std::iter::empty::<PathBuf>,
                        std::iter::empty::<PathBuf>,
                    )
                    .map(|bin| bin.join("git.exe"))
                    .find(|path| path.is_file())
                })
                .expect("the fixture contains the package's Git executable");
                assert_eq!(
                    executable.invocation(),
                    fixture.join(expected),
                    "{variable} discovers its installation when PATH is absent or has no Git"
                );
            }
        }
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn fallback_candidates_follow_cost_then_installation_priority() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        let variables = |key: &str| {
            let root = match key {
                "LocalAppData" => "winget",
                "ProgramW6432" => "machine",
                "ProgramFiles" => "other-machine",
                "ProgramFiles(x86)" => "x86",
                "GIT_INSTALL_ROOT" => "explicit",
                "ChocolateyToolsLocation" => "chocolatey",
                "SCOOP" => "no-junction",
                "SCOOP_GLOBAL" => "global/scoop",
                "CONDA_PREFIX" => "conda",
                _ => return None,
            };
            Some(fixture.join(root).into_os_string())
        };
        let locations: Vec<_> = super::super::alternative_locations(
            variables,
            || [fixture.join("registered/cmd")],
            || [fixture.join("registered-msys/usr/bin")],
        )
        .collect();
        let mut expected = super::super::locations_under_program_files(variables);
        expected.extend(
            [
                "explicit/cmd",
                "winget/Microsoft/WinGet/Links",
                "machine/WinGet/Links",
                "other-machine/WinGet/Links",
                "chocolatey/git/cmd",
                "no-junction/apps/git/current/cmd",
                "global/scoop/apps/git/current/cmd",
                "conda/Library/bin",
                "explicit/usr/bin",
                "registered/cmd",
                "no-junction/apps/git/2.50.0/cmd",
                "registered-msys/usr/bin",
            ]
            .map(|bin| fixture.join(bin)),
        );
        assert_eq!(
            locations, expected,
            "fixed paths use installer priority, followed by direct registry queries, Scoop scans and MSYS2 enumeration"
        );
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn fixed_path_winners_avoid_registry_queries_and_version_scans() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        for (explicit, winget, global_scoop, expected) in [
            (Some("explicit"), Some("winget"), None, "explicit/cmd/git.exe"),
            (None, Some("winget"), None, "winget/Microsoft/WinGet/Links/git.exe"),
            (None, None, Some("custom"), "custom/apps/git/current/cmd/git.exe"),
            (None, None, None, "conda/Library/bin/git.exe"),
        ] {
            let executable = super::super::find_executable(None, || {
                super::super::alternative_locations(
                    |key| {
                        let root = match key {
                            "LocalAppData" => winget?,
                            "GIT_INSTALL_ROOT" => explicit?,
                            "SCOOP" => "no-junction",
                            "SCOOP_GLOBAL" => global_scoop?,
                            "CONDA_PREFIX" => "conda",
                            _ => return None,
                        };
                        Some(fixture.join(root).into_os_string())
                    },
                    || -> Vec<PathBuf> { panic!("a fixed-path winner avoids native registry queries") },
                    || -> Vec<PathBuf> { panic!("a fixed-path winner avoids MSYS2 enumeration") },
                )
                .map(|bin| bin.join("git.exe"))
                .find(|path| path.is_file())
            })
            .expect("several fixture installations contain Git");
            assert_eq!(
                executable.invocation(),
                fixture.join(expected),
                "fixed paths precede the installed Scoop version that requires a directory scan"
            );
        }
        Ok(())
    }

    #[test]
    #[serial_test::serial]
    fn direct_registration_wins_before_version_scans() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        let executable = super::super::find_executable(None, || {
            super::super::alternative_locations(
                |key| (key == "SCOOP").then(|| fixture.join("no-junction").into_os_string()),
                || [fixture.join("path")],
                || -> Vec<PathBuf> { panic!("a direct registration avoids MSYS2 enumeration") },
            )
            .map(|bin| bin.join("git.exe"))
            .find(|path| path.is_file())
        })
        .expect("the fixture contains registered and no-junction Git installations");
        assert_eq!(
            executable.invocation(),
            fixture.join("path/git.exe"),
            "direct registry keys are cheaper than searching retained Scoop versions"
        );
        Ok(())
    }

    #[test]
    fn msys_paths_and_companions_use_the_selected_runtime() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let bin = fixture.join("msys/usr/bin");
        let executable = bin.join("git.exe");
        let _env = gix_testtools::Env::new().set("PATH", bin.to_str().expect("UTF-8 fixture path"));
        assert_eq!(
            super::super::msys_bin_at(&executable),
            Some(bin.as_path()),
            "identify the selected MSYS runtime"
        );
        let mixed_case_bin = fixture.join("msys/UsR/BiN");
        assert_eq!(
            super::super::msys_bin_at(&mixed_case_bin.join("git.exe")),
            Some(mixed_case_bin.as_path()),
            "Windows PATH spelling does not change the selected MSYS runtime"
        );
        assert_eq!(
            super::super::msys_bin_at(Path::new("usr/bin/git.exe")),
            None,
            "relative roots are not trusted"
        );
        assert_eq!(
            super::super::msys_bin_at(&fixture.join("path/git.exe")),
            None,
            "ordinary Git is not an MSYS runtime"
        );
        for path in ["/usr/lib/git-core", "/etc/gitconfig", "/a directory with spaces "] {
            let actual = super::super::native_path(Path::new(path)).expect("the bundled cygpath runs");
            assert_eq!(
                actual,
                fixture.join("msys").join(&path[1..]),
                "Unix paths are mapped by the same runtime, preserving spaces"
            );
        }
        for path in ["relative", r"C:\custom\git-core"] {
            assert_eq!(
                super::super::native_path(Path::new(path)).as_deref(),
                Some(Path::new(path)),
                "non-Unix overrides are preserved"
            );
        }
        for stem in ["bash", "sh"] {
            assert_eq!(
                crate::env::installation_program(stem),
                Some(bin.join(format!("{stem}.exe"))),
                "companions come from the selected runtime"
            );
        }
        assert!(
            !crate::env::shell().to_string_lossy().contains('\\'),
            "the shell path uses forward slashes"
        );
        assert_eq!(
            crate::env::shell_command().get_args().count(),
            0,
            "MSYS sh is not a Git for Windows shim"
        );
        Ok(())
    }
}
