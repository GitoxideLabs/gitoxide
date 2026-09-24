#[test]
fn component_is_windows_device() {
    for device in ["con", "CONIN$", "lpt1.txt", "AUX", "Prn", "NUL", "COM9", "nul.a.b "] {
        assert!(gix_validate::path::component_is_windows_device(device.into()));
    }
    for not_device in ["coni", "CONIN", "lpt", "AUXi", "aPrn", "NULl", "COM", "a.nul.b "] {
        assert!(!gix_validate::path::component_is_windows_device(not_device.into()));
    }
}

mod component {
    use gix_validate::path::component;

    const NO_OPTS: component::Options = component::Options {
        protect_windows: false,
        protect_hfs: false,
        protect_ntfs: false,
    };
    const ALL_OPTS: component::Options = component::Options {
        protect_windows: true,
        protect_hfs: true,
        protect_ntfs: true,
    };

    mod valid {
        use bstr::ByteSlice;
        use gix_validate::path::{component, component::Mode::Symlink};

        use crate::path::component::{ALL_OPTS, NO_OPTS};
        macro_rules! mktest {
            ($name:ident, $input:expr) => {
                mktest!($name, $input, ALL_OPTS);
            };
            ($name:ident, $input:expr, $opts:expr) => {
                #[test]
                fn $name() {
                    assert!(gix_validate::path::component($input.as_bstr(), None, $opts).is_ok())
                }
            };
            ($name:ident, $input:expr, $mode:expr, $opts:expr) => {
                #[test]
                fn $name() {
                    assert!(gix_validate::path::component($input.as_bstr(), Some($mode), $opts).is_ok())
                }
            };
        }

        const UNIX_OPTS: component::Options = component::Options {
            protect_windows: false,
            protect_hfs: true,
            protect_ntfs: true,
        };

        mktest!(ascii, b"ascii-only_and-that");
        mktest!(unicode, "😁👍👌".as_bytes());
        mktest!(backslashes_on_unix, br"\", UNIX_OPTS);
        mktest!(drive_letters_on_unix, b"c:", UNIX_OPTS);
        mktest!(virtual_drive_letters_on_unix, "֍:".as_bytes(), UNIX_OPTS);
        mktest!(unc_path_on_unix, br"\\?\pictures", UNIX_OPTS);
        mktest!(not_dot_git_longer, b".gitu", NO_OPTS);
        mktest!(not_dot_git_longer_all, b".gitu");
        mktest!(not_dot_gitmodules_shorter, b".gitmodule", Symlink, NO_OPTS);
        mktest!(not_dot_gitmodules_shorter_all, b".gitmodule", Symlink, ALL_OPTS);
        mktest!(not_dot_gitmodules_longer, b".gitmodulesa", Symlink, NO_OPTS);
        mktest!(not_dot_gitmodules_longer_all, b".gitmodulesa", Symlink, ALL_OPTS);
        mktest!(dot_gitmodules_as_file, b".gitmodules", UNIX_OPTS);
        mktest!(
            starts_with_dot_git_with_backslashes_on_linux,
            br".git\hooks\pre-commit",
            UNIX_OPTS
        );
        mktest!(not_dot_git_shorter, b".gi", NO_OPTS);
        mktest!(not_dot_git_shorter_ntfs_8_3, b"gi~1");
        mktest!(not_dot_git_longer_ntfs_8_3, b"gitu~1");
        mktest!(not_dot_git_shorter_ntfs_8_3_disabled, b"git~1", NO_OPTS);
        mktest!(not_dot_git_longer_hfs, ".g\u{200c}itu".as_bytes());
        mktest!(not_dot_git_shorter_hfs, ".g\u{200c}i".as_bytes());
        mktest!(com_0_lower, b"com0");
        mktest!(com_without_number_0_lower, b"comm");
        mktest!(conout_without_dollar_with_extension, b"conout.c");
        mktest!(conin_without_dollar_with_extension, b"conin.c");
        mktest!(conin_without_dollar, b"conin");
        mktest!(not_con, b"com");
        mktest!(also_not_con, b"co");
        mktest!(con_as_middle, b"x.CON.zip");
        mktest!(con_after_space, b" CON");
        mktest!(con_after_space_mixed, b" coN.tar.xz");
        mktest!(not_nul, b"null");
        mktest!(
            not_dot_gitmodules_shorter_hfs,
            ".gitm\u{200c}odule".as_bytes(),
            Symlink,
            UNIX_OPTS
        );
        mktest!(dot_gitmodules_as_file_hfs, ".g\u{200c}itmodules".as_bytes(), UNIX_OPTS);
        mktest!(dot_gitmodules_ntfs_8_3_disabled, b"gItMOD~1", Symlink, NO_OPTS);
        mktest!(
            not_dot_gitmodules_longer_hfs,
            "\u{200c}.gitmodulesa".as_bytes(),
            Symlink,
            UNIX_OPTS
        );
    }

    mod invalid {
        use bstr::ByteSlice;
        use gix_validate::path::component::{Error, Mode::Symlink};

        use crate::path::component::{ALL_OPTS, NO_OPTS};

        macro_rules! mktest {
            ($name:ident, $input:expr, $expected:pat, @$snapshot:literal) => {
                mktest!($name, $input, $expected, ALL_OPTS, @$snapshot);
            };
            ($name:ident, $input:expr, $expected:pat, $opts:expr, @$snapshot:literal) => {
                #[test]
                fn $name() {
                    let err = gix_validate::path::component($input.as_bstr(), None, $opts).expect_err("the input is invalid");
                    insta::assert_debug_snapshot!(err, "invalid path components retain their specific failure", @$snapshot);
                    assert!(matches!(err, $expected), "the failure retains its error variant");
                }
            };
            ($name:ident, $input:expr, $expected:pat, $mode:expr, $opts:expr, @$snapshot:literal) => {
                #[test]
                fn $name() {
                    let err = gix_validate::path::component($input.as_bstr(), Some($mode), $opts).expect_err("the input is invalid");
                    insta::assert_debug_snapshot!(err, "invalid path components retain their specific failure", @$snapshot);
                    assert!(matches!(err, $expected), "the failure retains its error variant");
                }
            };
        }

        mktest!(empty, b"", Error::Empty, @"Empty");
        mktest!(dot_git_lower, b".git", Error::DotGitDir, NO_OPTS, @"DotGitDir");
        mktest!(dot_git_lower_hfs, ".g\u{200c}it".as_bytes(), Error::DotGitDir, @"DotGitDir");
        mktest!(dot_git_mixed_hfs_simple, b".Git", Error::DotGitDir, @"DotGitDir");
        mktest!(dot_git_upper, b".GIT", Error::DotGitDir, NO_OPTS, @"DotGitDir");
        mktest!(
            starts_with_dot_git_with_backslashes_on_windows,
            br".git\hooks\pre-commit",
            Error::PathSeparator, @"PathSeparator");
        mktest!(dot_git_upper_hfs, ".GIT\u{200e}".as_bytes(), Error::DotGitDir, @"DotGitDir");
        mktest!(dot_git_upper_ntfs_8_3, b"GIT~1", Error::DotGitDir, @"DotGitDir");
        mktest!(dot_git_mixed, b".gIt", Error::DotGitDir, NO_OPTS, @"DotGitDir");
        mktest!(dot_git_mixed_ntfs_8_3, b"gIt~1", Error::DotGitDir, @"DotGitDir");
        mktest!(
            dot_gitmodules_mixed,
            b".gItmodules",
            Error::SymlinkedGitModules,
            Symlink,
            NO_OPTS, @"SymlinkedGitModules");
        mktest!(dot_git_mixed_hfs, "\u{206e}.gIt".as_bytes(), Error::DotGitDir, @"DotGitDir");
        mktest!(
            dot_git_ntfs_8_3_numbers_only,
            b"~1000000",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            dot_git_ntfs_8_3_numbers_only_too,
            b"~9999999",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            dot_gitmodules_mixed_hfs,
            "\u{206e}.gItmodules".as_bytes(),
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            dot_gitmodules_mixed_ntfs_8_3,
            b"gItMOD~1",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            dot_gitmodules_mixed_ntfs_stream,
            b".giTmodUles:$DATA",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            dot_gitmodules_lower_ntfs_stream_default_implicit,
            b".gitmodules::$DATA",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            ntfs_stream_default_implicit,
            b"file::$DATA",
            Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(
            ntfs_stream_explicit,
            b"file:ANYTHING_REALLY:$DATA",
            Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(
            dot_gitmodules_lower_ntfs_stream,
            b".gitmodules:$DATA:$DATA",
            Error::SymlinkedGitModules,
            Symlink,
            ALL_OPTS, @"SymlinkedGitModules");
        mktest!(
            not_gitmodules_trailing_space,
            b".gitmodules x ",
            Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(
            not_gitmodules_trailing_stream,
            b".gitmodules,:$DATA",
            Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(path_separator_slash_between, b"a/b", Error::PathSeparator, @"PathSeparator");
        mktest!(path_separator_slash_leading, b"/a", Error::PathSeparator, @"PathSeparator");
        mktest!(path_separator_slash_trailing, b"a/", Error::PathSeparator, @"PathSeparator");
        mktest!(path_separator_slash_only, b"/", Error::PathSeparator, @"PathSeparator");
        mktest!(slashes_on_windows, b"/", Error::PathSeparator, ALL_OPTS, @"PathSeparator");
        mktest!(backslashes_on_windows, br"\", Error::PathSeparator, ALL_OPTS, @"PathSeparator");
        mktest!(path_separator_backslash_between, br"a\b", Error::PathSeparator, @"PathSeparator");
        mktest!(path_separator_backslash_leading, br"\a", Error::PathSeparator, @"PathSeparator");
        mktest!(path_separator_backslash_trailing, br"a\", Error::PathSeparator, @"PathSeparator");
        mktest!(aux_mixed, b"Aux", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(aux_with_extension, b"aux.c", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(com_lower, b"com1", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(com_upper_with_extension, b"COM9.c", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(trailing_space, b"win32 ", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(trailing_dot, b"win32.", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(trailing_dot_dot, b"win32 . .", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(colon_inbetween, b"colon:separates", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(left_arrow, b"arrow<left", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(right_arrow, b"arrow>right", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(apostrophe, b"a\"b", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(pipe, b"a|b", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(questionmark, b"a?b", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(asterisk, b"a*b", Error::WindowsIllegalCharacter, @"WindowsIllegalCharacter");
        mktest!(lpt_mixed_with_number, b"LPt8", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(nul_mixed, b"NuL", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(prn_mixed_with_extension, b"PrN.abc", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(con, b"CON", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(con_with_extension, b"CON.abc", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(con_with_middle, b"CON.tar.xz", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(con_mixed_with_middle, b"coN.tar.xz ", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(dot_dot, b"..", Error::Relative, @"Relative");
        mktest!(dot_dot_no_opts, b"..", Error::Relative, NO_OPTS, @"Relative");
        mktest!(single_dot, b".", Error::Relative, @"Relative");
        mktest!(single_dot_no_opts, b".", Error::Relative, NO_OPTS, @"Relative");
        mktest!(
            conout_mixed_with_extension,
            b"ConOut$  .xyz",
            Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(conin_mixed, b"conIn$  ", Error::WindowsReservedName, @"WindowsReservedName");
        mktest!(drive_letters, b"c:", Error::WindowsPathPrefix, ALL_OPTS, @"WindowsPathPrefix");
        mktest!(
            virtual_drive_letters,
            "֍:".as_bytes(),
            Error::WindowsPathPrefix,
            ALL_OPTS, @"WindowsPathPrefix");
        mktest!(unc_net_path, br"\\host", Error::PathSeparator, ALL_OPTS, @"PathSeparator");
        mktest!(unc_path, br"\\?\pictures", Error::PathSeparator, ALL_OPTS, @"PathSeparator");
        mktest!(unc_device_path, br"\\.\pictures", Error::PathSeparator, ALL_OPTS, @"PathSeparator");
        mktest!(unc_nt_obj_path, br"\??\pictures", Error::PathSeparator, ALL_OPTS, @"PathSeparator");

        #[test]
        fn ntfs_gitmodules() {
            for invalid in [
                ".gitmodules",
                ".Gitmodules",
                ".gitmoduleS",
                ".gitmodules ",
                ".gitmodules.",
                ".gitmodules  ",
                ".gitmodules. ",
                ".gitmodules .",
                ".gitmodules..",
                ".gitmodules   ",
                ".gitmodules.  ",
                ".gitmodules . ",
                ".gitmodules  .",
                ".Gitmodules ",
                ".Gitmodules.",
                ".Gitmodules  ",
                ".Gitmodules. ",
                ".Gitmodules .",
                ".Gitmodules..",
                ".Gitmodules   ",
                ".Gitmodules.  ",
                ".Gitmodules . ",
                ".Gitmodules  .",
                "GITMOD~1",
                "gitmod~1",
                "GITMOD~2",
                "giTmod~3",
                "GITMOD~4",
                "GITMOD~1 ",
                "gitMod~2.",
                "GITMOD~3  ",
                "gitmod~4. ",
                "GITMoD~1 .",
                "gitmod~2   ",
                "GITMOD~3.  ",
                "gitmoD~4 . ",
                "GI7EBA~1",
                "gi7eba~9",
                "GI7EB~10",
                "GI7EB~11",
                "GI7EB~99",
                "GI7EB~10",
                "GI7E~100",
                "GI7E~101",
                "GI7E~999",
                ".gitmodules:$DATA",
                "gitmod~4 . :$DATA",
            ] {
                match gix_validate::path::component(invalid.into(), Some(Symlink), ALL_OPTS) {
                    Ok(_) => {
                        unreachable!("{invalid:?} should not validate successfully")
                    }
                    Err(err) => {
                        assert!(matches!(err, Error::SymlinkedGitModules));
                    }
                }
            }

            for valid in [
                ".gitmodules x",
                ".gitmodules .x",
                " .gitmodules",
                "..gitmodules",
                "gitmodules",
                ".gitmodule",
                ".gitmodules .x",
                "GI7EBA~",
                "GI7EBA~0",
                "GI7EBA~~1",
                "GI7EBA~X",
                "Gx7EBA~1",
                "GI7EBX~1",
                "GI7EB~1",
                "GI7EB~01",
                "GI7EB~1X",
            ] {
                gix_validate::path::component(valid.into(), Some(Symlink), ALL_OPTS)
                    .unwrap_or_else(|_| panic!("{valid:?} should have been valid"));
            }
        }
    }
}
