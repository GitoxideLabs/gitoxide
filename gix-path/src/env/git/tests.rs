use std::path::Path;

#[cfg(windows)]
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
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\clangarm64\bin",
                    r"X:\cur\rent\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
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
            pathbuf_vec![r"Z:\wi\de\Git\clangarm64\bin", r"Z:\wi\de\Git\mingw64\bin"],
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
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Z:\wi\de\Git\clangarm64\bin",
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
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw64\bin",
                    r"C:\Users\alice\AppData\Local\Programs\Git\mingw32\bin",
                    r"C:\Program Files\Git\clangarm64\bin",
                    r"C:\Program Files\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"C:\Users\alice\AppData\Local\Programs\Git\clangarm64\bin",
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
                r"C:\Users\bob\AppData\Local\Programs\Git\mingw64\bin",
                r"C:\Users\bob\AppData\Local\Programs\Git\mingw32\bin",
                r"C:\Program Files\Git\clangarm64\bin",
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
                    r"W:\us\er\Programs\Git\mingw64\bin",
                    r"W:\us\er\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                    r"Y:\nar\row\Git\mingw32\bin",
                    r"X:\cur\rent\Git\clangarm64\bin",
                    r"X:\cur\rent\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"W:\us\er\Programs\Git\clangarm64\bin",
                    r"W:\us\er\Programs\Git\mingw64\bin",
                    r"W:\us\er\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
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
                r"W:\us\er\Programs\Git\mingw64\bin",
                r"W:\us\er\Programs\Git\mingw32\bin",
                r"Z:\wi\de\Git\clangarm64\bin",
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
                    r"Y:\nar\row\Programs\Git\mingw64\bin",
                    r"Y:\nar\row\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
                    r"Z:\wi\de\Git\mingw64\bin",
                ]
            } else {
                pathbuf_vec![
                    r"Y:\nar\row\Programs\Git\clangarm64\bin",
                    r"Y:\nar\row\Programs\Git\mingw64\bin",
                    r"Y:\nar\row\Programs\Git\mingw32\bin",
                    r"Z:\wi\de\Git\clangarm64\bin",
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
                assert!(cfg!(target_pointer_width = "64"));
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
        /// approach used for `ALTERNATIVE_LOCATIONS`, so it can be used to test that. The approach
        /// here is also more reliable than using environment variables, but it is a bit more
        /// complex, and it requires either additional dependencies or the use of unsafe code.
        ///
        /// This gets `pf_user`, `pf_current`, and `pf_x86` by the [known folders][known-folders]
        /// system. But it gets `maybe_pf_64bit` from the registry, as the corresponding known
        /// folder is not available to 32-bit processes. See the [`KNOWNFOLDDERID`][knownfolderid]
        /// documentation.
        ///
        /// If in the future the implementation of `ALTERNATIVE_LOCATIONS` uses these techniques,
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
                .map_err(|error| {
                    assert_eq!(error.kind(), ErrorKind::NotFound);
                    error
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
        arm64: &'a Path,
    }

    impl<'a> RelativeUserGitBinPaths<'a> {
        /// Assert that `locations` leads with the given user path prefix, and extract the suffixes.
        fn assert_from(pf_user: &'a Path, locations: &'static [PathBuf]) -> Self {
            match locations {
                [path1, path2, path3, ..] => {
                    let suffix_user_arm64 = path1
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user 64-bit ARM64 path and lists it first");
                    let suffix_user_x64 = path2
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user 64-bit x86 path and lists it second");
                    let suffix_user_x86 = path3
                        .strip_prefix(pf_user)
                        .expect("It gives a per-user 32-bit x86 path and lists it third");
                    Self {
                        x86: suffix_user_x86,
                        x64: suffix_user_x64,
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
            assert_eq!(self.x64, Path::new("Git/mingw64/bin"));
            assert_eq!(self.arm64, Path::new("Git/clangarm64/bin"));
        }
    }

    /// Architecture-specific Git for Windows paths relative to global program files directories.
    #[derive(Clone, Debug)]
    struct RelativeGlobalGitBinPaths<'a> {
        x86: &'a Path,
        maybe_x64: Option<&'a Path>,
        maybe_arm64: Option<&'a Path>,
    }

    impl<'a> RelativeGlobalGitBinPaths<'a> {
        /// Assert that `locations` trails with the given global path prefixes, and extract the suffixes.
        fn assert_from(pf: &'a ProgramFilesPaths, locations: &'static [PathBuf]) -> Self {
            match locations {
                [_, _, _, path4, path5, path6] => {
                    let prefix_64bit = pf
                        .maybe_64bit
                        .as_ref()
                        .expect("It gives 6 paths only if some global paths can be 64-bit");
                    let suffix_global_arm64 = path4
                        .strip_prefix(prefix_64bit)
                        .expect("It gives a global 64-bit ARM64 path and lists it fourth");
                    let suffix_global_x64 = path5
                        .strip_prefix(prefix_64bit)
                        .expect("It gives a global 64-bit x86 path and lists it fifth");
                    let suffix_global_x86 = path6
                        .strip_prefix(&pf.x86)
                        .expect("It gives a global 32-bit path and lists it sixth");
                    Self {
                        x86: suffix_global_x86,
                        maybe_x64: Some(suffix_global_x64),
                        maybe_arm64: Some(suffix_global_arm64),
                    }
                }
                [_, _, _, path4] => {
                    assert_eq!(
                        pf.maybe_64bit, None,
                        "It gives 4 paths only if no global paths can be 64-bit.",
                    );
                    let suffix_global_x86 = path4
                        .strip_prefix(&pf.x86)
                        .expect("It gives a global 32-bit path and lists it fourth");
                    Self {
                        x86: suffix_global_x86,
                        maybe_x64: None,
                        maybe_arm64: None,
                    }
                }
                other => panic!("{:?} has length {}, expected 4 or 6.", other, other.len()),
            }
        }

        /// Assert that suffixes are common Git install locations relative to a program files directory.
        fn assert_architectures(&self) {
            assert_eq!(self.x86, Path::new("Git/mingw32/bin"));

            if let Some(suffix_x64) = self.maybe_x64 {
                assert_eq!(suffix_x64, Path::new("Git/mingw64/bin"));
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
        fn assert_from(pf: &'a ProgramFilesPaths, locations: &'static [PathBuf]) -> Self {
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
    fn alternative_locations() {
        // Obtain program files directory paths by other means and check that they seem correct.
        let pf = ProgramFilesPaths::obtain_envlessly().validated();

        // Check that `ALTERNATIVE_LOCATIONS` correspond to them, with the correct subdirectories.
        let locations = super::super::ALTERNATIVE_LOCATIONS.as_slice();
        RelativeGitBinPaths::assert_from(&pf, locations).assert_architectures();
    }
}

#[cfg(not(windows))]
mod locations {
    #[test]
    fn alternative_locations() {
        assert!(super::super::ALTERNATIVE_LOCATIONS.is_empty());
    }
}

mod exe_info {
    use std::path::{Path, PathBuf};

    use gix_testtools::tempfile;
    use serial_test::serial;

    use crate::env::git::{NULL_DEVICE, exe_info};

    /// Wrapper for a valid path to a plausible location, kept from accidentally existing (until drop).
    #[derive(Debug)]
    struct NonexistentLocation {
        _empty: tempfile::TempDir,
        nonexistent: PathBuf,
    }

    impl NonexistentLocation {
        fn new() -> Self {
            let empty = tempfile::tempdir().expect("can create new temporary subdirectory");

            let nonexistent = empty
                .path()
                .canonicalize()
                .expect("path to the new directory works")
                .join("nonexistent");

            assert!(!nonexistent.exists(), "Test bug: Need nonexistent directory");

            Self {
                _empty: empty,
                nonexistent,
            }
        }

        fn path(&self) -> &Path {
            &self.nonexistent
        }
    }

    fn set_temp_env_vars<'a>(path: &Path) -> gix_testtools::Env<'a> {
        let path_str = path.to_str().expect("valid Unicode");

        let env = gix_testtools::Env::new()
            .set("TMPDIR", path_str) // Mainly for Unix.
            .set("TMP", path_str) // Mainly for Windows.
            .set("TEMP", path_str); // Mainly for Windows, too.

        assert_eq!(
            std::env::temp_dir(),
            path,
            "Possible test bug: Temp dir path may not have been customized successfully"
        );

        env
    }

    fn unset_windows_directory_vars<'a>() -> gix_testtools::Env<'a> {
        gix_testtools::Env::new().unset("windir").unset("SystemRoot")
    }

    fn check_exe_info() {
        let path = exe_info()
            .map(crate::from_bstring)
            .expect("It is present in the test environment (nonempty config)");

        assert!(
            path.is_absolute(),
            "It is absolute (unless overridden such as with GIT_CONFIG_SYSTEM)"
        );
        assert!(
            path.exists(),
            "It should exist on disk, since `git config` just found an entry there"
        );
    }

    #[test]
    #[serial]
    fn with_unmodified_environment() {
        check_exe_info();
    }

    #[test]
    #[serial]
    fn tolerates_broken_temp() {
        let non = NonexistentLocation::new();
        let _env = set_temp_env_vars(non.path());
        check_exe_info();
    }

    #[test]
    #[serial]
    fn tolerates_oversanitized_env() {
        // This test runs on all systems, but it is only checking for a Windows regression. Also, on
        // Windows, having both a broken temp dir and an over-sanitized environment is not supported.
        let _env = unset_windows_directory_vars();
        check_exe_info();
    }

    #[test]
    #[serial]
    fn tolerates_git_config_env_var() {
        let _env = gix_testtools::Env::new().set("GIT_CONFIG", NULL_DEVICE);
        check_exe_info();
    }

    #[test]
    #[serial]
    fn same_result_with_broken_temp() {
        let with_unmodified_temp = exe_info();

        let with_nonexistent_temp = {
            let non = NonexistentLocation::new();
            let _env = set_temp_env_vars(non.path());
            exe_info()
        };

        assert_eq!(with_unmodified_temp, with_nonexistent_temp);
    }

    #[test]
    #[serial]
    fn same_result_with_oversanitized_env() {
        let with_unmodified_env = exe_info();

        let with_oversanitized_env = {
            let _env = unset_windows_directory_vars();
            exe_info()
        };

        assert_eq!(with_unmodified_env, with_oversanitized_env);
    }

    #[test]
    #[serial]
    fn same_result_with_git_config_env_var() {
        let with_unmodified_env = exe_info();

        let with_git_config_env_var = {
            let _env = gix_testtools::Env::new().set("GIT_CONFIG", NULL_DEVICE);
            exe_info()
        };

        assert_eq!(with_unmodified_env, with_git_config_env_var);
    }

    #[test]
    #[serial]
    #[cfg(not(target_os = "macos"))] // Assumes no higher "unknown" scope. The `nosystem` case works.
    fn never_from_local_scope() {
        let repo = gix_testtools::scripted_fixture_read_only("local_config.sh").expect("script succeeds");

        let _cwd = gix_testtools::set_current_dir(repo).expect("can change to repo dir");
        let _env = gix_testtools::Env::new()
            .set("GIT_CONFIG_SYSTEM", NULL_DEVICE)
            .set("GIT_CONFIG_GLOBAL", NULL_DEVICE);

        let maybe_path = exe_info();
        assert_eq!(
            maybe_path, None,
            "Should find no config path if the config would be local (empty system config)"
        );
    }

    #[test]
    #[serial]
    fn never_from_local_scope_nosystem() {
        let repo = gix_testtools::scripted_fixture_read_only("local_config.sh").expect("script succeeds");

        let _cwd = gix_testtools::set_current_dir(repo).expect("can change to repo dir");
        let _env = gix_testtools::Env::new()
            .set("GIT_CONFIG_NOSYSTEM", "1")
            .set("GIT_CONFIG_GLOBAL", NULL_DEVICE);

        let maybe_path = exe_info();
        assert_eq!(
            maybe_path, None,
            "Should find no config path if the config would be local (suppressed system config)"
        );
    }

    #[test]
    #[serial]
    #[cfg(not(target_os = "macos"))] // Assumes no higher "unknown" scope. The `nosystem` case works.
    fn never_from_local_scope_even_if_temp_is_here() {
        let repo = gix_testtools::scripted_fixture_read_only("local_config.sh")
            .expect("script succeeds")
            .canonicalize()
            .expect("repo path is valid and exists");

        let _cwd = gix_testtools::set_current_dir(&repo).expect("can change to repo dir");
        let _env = set_temp_env_vars(&repo)
            .set("GIT_CONFIG_SYSTEM", NULL_DEVICE)
            .set("GIT_CONFIG_GLOBAL", NULL_DEVICE);

        let maybe_path = exe_info();
        assert_eq!(
            maybe_path, None,
            "Should find no config path if the config would be local even in a `/tmp`-like dir (empty system config)"
        );
    }

    #[test]
    #[serial]
    fn never_from_local_scope_even_if_temp_is_here_nosystem() {
        let repo = gix_testtools::scripted_fixture_read_only("local_config.sh")
            .expect("script succeeds")
            .canonicalize()
            .expect("repo path is valid and exists");

        let _cwd = gix_testtools::set_current_dir(&repo).expect("can change to repo dir");
        let _env = set_temp_env_vars(&repo)
            .set("GIT_CONFIG_NOSYSTEM", "1")
            .set("GIT_CONFIG_GLOBAL", NULL_DEVICE);

        let maybe_path = exe_info();
        assert_eq!(
            maybe_path, None,
            "Should find no config path if the config would be local even in a `/tmp`-like dir (suppressed system config)"
        );
    }

    #[test]
    #[serial]
    fn never_from_git_config_env_var() {
        let repo = gix_testtools::scripted_fixture_read_only("local_config.sh").expect("script succeeds");

        // Get an absolute path to a config file that is non-UNC if possible so any Git accepts it.
        let config_path = std::env::current_dir()
            .expect("got CWD")
            .join(repo)
            .join(".git")
            .join("config")
            .to_str()
            .expect("valid UTF-8")
            .to_owned();

        let _env = gix_testtools::Env::new()
            .set("GIT_CONFIG_NOSYSTEM", "1")
            .set("GIT_CONFIG_GLOBAL", NULL_DEVICE)
            .set("GIT_CONFIG", config_path);

        let maybe_path = exe_info();
        assert_eq!(
            maybe_path, None,
            "Should find no config path from GIT_CONFIG (even if nonempty)"
        );
    }

    #[test]
    fn first_file_from_config_with_origin() {
        let macos = "file:/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig\0credential.helper\0file:/Users/byron/.gitconfig\0push.default\0";
        let win_msys =
            "file:C:/git-sdk-64/etc/gitconfig\0core.symlinks\0file:C:/git-sdk-64/etc/gitconfig\0core.autocrlf\0";
        let win_cmd = "file:C:/Program Files/Git/etc/gitconfig\0diff.astextplain.textconv\0file:C:/Program Files/Git/etc/gitconfig\0filter.lfs.clean\0";
        let win_msys_old = "file:C:\\ProgramData/Git/config\0diff.astextplain.textconv\0file:C:\\ProgramData/Git/config\0filter.lfs.clean\0";
        let linux = "file:/home/parallels/.gitconfig\0core.excludesfile\0";
        let bogus = "something unexpected";
        let empty = "";

        for (source, expected) in [
            (
                macos,
                Some("/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig"),
            ),
            (win_msys, Some("C:/git-sdk-64/etc/gitconfig")),
            (win_msys_old, Some(r"C:\ProgramData/Git/config")),
            (win_cmd, Some("C:/Program Files/Git/etc/gitconfig")),
            (linux, Some("/home/parallels/.gitconfig")),
            (bogus, None),
            (empty, None),
        ] {
            assert_eq!(
                crate::env::git::first_file_from_config_with_origin(source.into()),
                expected.map(Into::into)
            );
        }
    }
}

#[test]
fn config_to_base_path() {
    for (input, expected) in [
        (
            "/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig",
            "/Applications/Xcode.app/Contents/Developer/usr/share/git-core",
        ),
        ("C:/git-sdk-64/etc/gitconfig", "C:/git-sdk-64/etc"),
        (r"C:\ProgramData/Git/config", r"C:\ProgramData/Git"),
        ("C:/Program Files/Git/etc/gitconfig", "C:/Program Files/Git/etc"),
    ] {
        assert_eq!(super::config_to_base_path(Path::new(input)), Path::new(expected));
    }
}
