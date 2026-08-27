use super::*;

#[test]
fn parse_version() {
    assert_eq!(git_version_from_bytes(b"git version 2.37.2").unwrap(), (2, 37, 2));
    assert_eq!(
        git_version_from_bytes(b"git version 2.32.1 (Apple Git-133)").unwrap(),
        (2, 32, 1)
    );
}

#[test]
fn parse_version_with_trailing_newline() {
    assert_eq!(git_version_from_bytes(b"git version 2.37.2\n").unwrap(), (2, 37, 2));
}

const SCOPE_ENV_VALUE: &str = "gitconfig";

fn populate_ad_hoc_config_files(dir: &Path) {
    const CONFIG_DATA: &[u8] = b"[foo]\n\tbar = baz\n";

    let paths: &[PathBuf] = if cfg!(windows) {
        let unc_literal_nul = dir.canonicalize().expect("directory exists").join("nul");
        &[dir.join(SCOPE_ENV_VALUE), dir.join("-"), unc_literal_nul]
    } else {
        &[dir.join(SCOPE_ENV_VALUE), dir.join("-"), dir.join(":")]
    };
    // Create the files.
    for path in paths {
        std::fs::write(path, CONFIG_DATA).expect("can write contents");
    }
    // Verify the files. This is mostly to show we really made a `\\?\...\nul` on Windows.
    for path in paths {
        let buf = std::fs::read(path).expect("the file really exists");
        assert_eq!(buf, CONFIG_DATA, "{path:?} should be a config file");
    }
}

#[test]
fn configure_command_clears_external_config() {
    let temp = tempfile::TempDir::new().expect("can create temp dir");
    populate_ad_hoc_config_files(temp.path());

    let mut cmd = std::process::Command::new(gix_path::env::exe_invocation());
    cmd.env("GIT_CONFIG_SYSTEM", SCOPE_ENV_VALUE);
    cmd.env("GIT_CONFIG_GLOBAL", SCOPE_ENV_VALUE);
    configure_command(
        &mut cmd,
        gix_hash::Kind::default(),
        ["config", "-l", "--show-origin"],
        temp.path(),
    );

    let output = cmd.output().expect("can run git");
    let lines: Vec<_> = output
        .stdout
        .to_str()
        .expect("valid UTF-8")
        .lines()
        .filter(|line| !line.starts_with("command line:\t"))
        .collect();
    let status = output.status.code().expect("terminated normally");
    assert_eq!(lines, Vec::<&str>::new(), "should be no config variables from files");
    assert_eq!(status, 0, "reading the config should succeed");
}

#[test]
fn isolated_git_helpers_preserve_paths_and_report_failures() {
    let temp = tempfile::TempDir::new().expect("a temporary repository location can be created");
    let output = isolated_git_output_checked(
        None,
        [OsStr::new("init"), OsStr::new("--quiet"), temp.path().as_os_str()],
    )
    .expect("the isolated helper can initialize a repository from OS-native arguments");
    assert!(output.status.success(), "the checked helper returns successful output");

    let output = isolated_git_output(Some(temp.path()), ["rev-parse", "--verify", "missing"])
        .expect("the unchecked helper returns Git's unsuccessful output");
    assert!(
        !output.status.success(),
        "the unchecked helper preserves an expected Git failure"
    );

    let error = isolated_git_output_checked(Some(temp.path()), ["rev-parse", "--verify", "missing"])
        .expect_err("the checked helper turns an unsuccessful exit into an error");
    let error = error.to_string();
    assert!(
        error.contains("rev-parse --verify missing"),
        "the checked helper identifies the failed arguments: {error}"
    );
    assert!(
        error.contains("stderr:"),
        "the checked helper retains Git's diagnostics: {error}"
    );
}

#[test]
fn an_absolute_selected_git_is_preferred_in_path() {
    let temp = tempfile::TempDir::new().expect("can create temp dir");
    let git = temp.path().join("bin").join("git");
    let mut command = std::process::Command::new("fixture-script");

    prefer_git_in_path(&mut command, &git);

    let path = command
        .get_envs()
        .find_map(|(key, value)| (key == "PATH").then_some(value))
        .flatten()
        .expect("an absolute Git executable overrides PATH");
    assert_eq!(
        std::env::split_paths(path).next().as_deref(),
        git.parent(),
        "the selected Git executable's directory is searched first"
    );
}

#[test]
fn a_path_resolved_selected_git_does_not_override_path() {
    let mut command = std::process::Command::new("fixture-script");
    command.env("PATH", "existing-path");

    prefer_git_in_path(&mut command, Path::new("git"));

    let path = command
        .get_envs()
        .find_map(|(key, value)| (key == "PATH").then_some(value))
        .flatten();
    assert_eq!(path, Some(std::ffi::OsStr::new("existing-path")));
}

#[test]
fn configure_command_overrides_xdg_config_home() {
    let temp = tempfile::TempDir::new().expect("can create temp dir");
    let mut cmd = std::process::Command::new(gix_path::env::exe_invocation());
    cmd.env("XDG_CONFIG_HOME", temp.path().join("external-config"));
    configure_command(&mut cmd, gix_hash::Kind::default(), ["--version"], temp.path());

    let xdg_config_home = cmd
        .get_envs()
        .find_map(|(key, value)| (key == "XDG_CONFIG_HOME").then_some(value))
        .flatten();
    assert_eq!(
        xdg_config_home,
        Some(temp.path().join(".gix-testtools-xdg-config").as_os_str())
    );
}

#[test]
#[cfg(windows)]
fn bash_program_ok_for_platform() {
    let path = bash_program();
    assert!(path.is_absolute());

    let for_version = std::process::Command::new(path)
        .arg("--version")
        .output()
        .expect("can pass it `--version`");
    assert!(for_version.status.success(), "passing `--version` succeeds");
    for_version
        .stdout
        .lines()
        .nth(0)
        .expect("`--version` output has first line");

    let for_uname_os = std::process::Command::new(path)
        .args(["-c", "uname -o"])
        .output()
        .expect("can tell it to run `uname -o`");
    assert!(for_uname_os.status.success(), "telling it to run `uname -o` succeeds");
    assert_eq!(
        for_uname_os.stdout.trim_end(),
        b"Msys",
        "it runs commands in an MSYS environment"
    );
}

#[test]
#[cfg(not(windows))]
fn bash_program_ok_for_platform() {
    assert_eq!(bash_program(), Path::new("bash"));
}

#[test]
fn bash_program_unix_path() {
    let path = bash_program()
        .to_str()
        .expect("This test depends on the bash path being valid Unicode");
    assert!(
        !path.contains('\\'),
        "The path to bash should have no backslashes, barring very unusual environments"
    );
}

fn is_rooted_relative(path: impl AsRef<Path>) -> bool {
    let p = path.as_ref();
    p.is_relative() && p.has_root()
}

#[test]
#[cfg(windows)]
fn unix_style_absolute_is_rooted_relative() {
    assert!(is_rooted_relative("/bin/bash"), "can detect paths like /bin/bash");
}

#[test]
fn bash_program_absolute_or_unrooted() {
    let bash = bash_program();
    assert!(!is_rooted_relative(bash), "{bash:?}");
}

#[test]
fn invoke_bash_runs_in_given_working_directory() {
    let dir = tempfile::TempDir::new().expect("can create temp dir");
    invoke_bash(dir.path(), "printf '%s' hello > out");
    assert_eq!(
        std::fs::read(dir.path().join("out")).expect("script wrote output"),
        b"hello"
    );
}

#[test]
fn invoke_bash_disables_auto_maintenance_for_git_commands() {
    let dir = tempfile::TempDir::new().expect("can create temp dir");
    invoke_bash(
        dir.path(),
        "git config --get maintenance.auto > out && git config --get gc.auto >> out",
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("out")).expect("script wrote output"),
        "false\n0\n",
        "Git commands run from the shell should not run automatic maintenance"
    );
}

#[test]
fn run_git_disables_auto_maintenance() -> Result {
    let dir = tempfile::TempDir::new().expect("can create temp dir");
    let status = run_git(dir.path(), &["config", "--get", "maintenance.auto"])?;
    assert!(status.success(), "command-scope maintenance.auto should be visible");
    let status = run_git(dir.path(), &["config", "--get", "gc.auto"])?;
    assert!(status.success(), "command-scope gc.auto should be visible");
    Ok(())
}

#[test]
fn git_helper_disables_auto_maintenance() -> Result {
    let dir = tempfile::TempDir::new().expect("can create temp dir");
    assert_eq!(
        git(dir.path(), "config --get maintenance.auto")?,
        "false\n",
        "Git commands run through gix-testtools should not run automatic maintenance"
    );
    assert_eq!(
        git(dir.path(), "config --get gc.auto")?,
        "0\n",
        "Auto-gc should be disabled for Git commands run through gix-testtools"
    );
    Ok(())
}

#[test]
fn split_git_arguments_handles_multiline_whitespace() {
    assert_eq!(
        split_git_arguments(
            "log
             --graph
             --oneline",
        )
        .expect("valid arguments"),
        ["log", "--graph", "--oneline"]
    );
}

#[test]
fn split_git_arguments_handles_quoted_arguments() {
    assert_eq!(
        split_git_arguments(
            "commit
             -m 'subject with spaces'
             --author=\"A U Thor <author@example.com>\"",
        )
        .expect("valid arguments"),
        [
            "commit",
            "-m",
            "subject with spaces",
            "--author=A U Thor <author@example.com>"
        ]
    );
}

#[test]
fn split_git_arguments_handles_empty_quoted_arguments() {
    assert_eq!(
        split_git_arguments("diff -- pathspec:''").expect("valid arguments"),
        ["diff", "--", "pathspec:"]
    );
    assert_eq!(
        split_git_arguments("diff -- ''").expect("valid arguments"),
        ["diff", "--", ""]
    );
}

#[test]
fn split_git_arguments_handles_escaped_whitespace() {
    assert_eq!(
        split_git_arguments(r"add path\ with\ spaces").expect("valid arguments"),
        ["add", "path with spaces"]
    );
}

#[test]
fn split_git_arguments_concatenates_quoted_and_unquoted_parts() {
    assert_eq!(
        split_git_arguments(r#"commit -m prefix" quoted "suffix"#).expect("valid arguments"),
        ["commit", "-m", "prefix quoted suffix"]
    );
}

#[test]
fn split_git_arguments_rejects_unterminated_quotes() {
    assert!(split_git_arguments("commit -m 'unterminated").is_err());
    assert!(split_git_arguments("commit -m \"unterminated").is_err());
}

#[test]
#[cfg(feature = "sha1")]
fn normalize_debug_snapshot_returns_replaced_ids_by_placeholder_index() {
    let first = gix_hash::ObjectId::from_hex(b"e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").expect("valid SHA1");
    let second = gix_hash::ObjectId::from_hex(b"496d6428b9cf92981dc9495211e6e1120fb6f2ba").expect("valid SHA1");
    let (snapshot, ids) = normalize_debug_snapshot(&vec![first, first, second, first]);

    assert_eq!(ids, vec![first, second]);
    assert_eq!(
        snapshot,
        r#"[
    Oid(1),
    Oid(1),
    Oid(2),
    Oid(1),
]"#
    );
}

#[test]
#[cfg(all(feature = "sha1", feature = "sha256"))]
fn normalize_hashes_replaces_raw_object_ids() {
    let sha1 = gix_hash::ObjectId::from_hex(b"e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").expect("valid SHA1");
    let sha256 = gix_hash::ObjectId::from_hex(b"473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813")
        .expect("valid SHA256");

    let (snapshot, ids) = normalize_hashes(
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 \
         473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813 \
         e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
    );

    assert_eq!(ids, vec![sha1, sha256]);
    assert_eq!(snapshot, "Oid(1) Oid(2) Oid(1)");
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_matches_archive_basename_patterns() {
    let lines = "\n# generated fixture archives\nrust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("tests/fixtures/generated-archives/rust-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("tests/fixtures/generated-archives/script-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_matches_paths_relative_to_fixture_base() {
    let lines = "generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("other-generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_treats_leading_slash_as_rooted_pattern() {
    let lines = "/generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_ignores_blank_lines_and_comments() {
    let lines = "\n  \n# generated-archives/rust-*.tar\ngenerated-archives/script-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new("generated-archives/script-basic.tar")
    ));
    assert!(!is_excluded_by_lines(
        lines,
        Path::new("generated-archives/rust-basic.tar")
    ));
}

#[test]
#[cfg(not(feature = "worktree-exclusions"))]
fn gitignore_fallback_normalizes_windows_path_separators() {
    let lines = "generated-archives/rust-*.tar\n";

    assert!(is_excluded_by_lines(
        lines,
        Path::new(r"generated-archives\rust-basic.tar")
    ));
}

#[test]
fn archive_required_fixtures_use_a_separate_cache_directory() {
    // Archive-required fixtures must not share the normal generated fixture
    // cache. Otherwise, a previous script run can leave platform-specific
    // output behind and make a later archive-required request skip extraction.
    // Using different paths makes sure they are actually from the archive if they exist.
    let fixture_base = Path::new("tests").join("fixtures");
    let (_, generated_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::default()),
        &1234,
        None,
    );
    let (_, archived_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::default()),
        &1234,
        Some("archive"),
    );

    assert_ne!(generated_dir, archived_dir);
    assert!(
        archived_dir
            .components()
            .any(|component| component.as_os_str() == "archive")
    );
}

struct Included;

impl IsExcluded for Included {
    fn is_excluded(&self, _archive: &Path) -> bool {
        false
    }
}

fn write_test_archive(source: &Path, archive: &Path, identity: u32) {
    let meta_dir = populate_meta_dir(source, identity).expect("archive metadata can be created");
    let mut archive_buf = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut archive_buf);
        builder.append_dir_all(".", source).expect("fixture can be archived");
        builder.finish().expect("archive can be finished");
    }

    #[cfg(feature = "xz")]
    {
        use std::io::Write;

        let mut encoder = xz2::write::XzEncoder::new(Vec::new(), 3);
        encoder.write_all(&archive_buf).expect("archive can be compressed");
        std::fs::write(archive, encoder.finish().expect("compression can finish"))
            .expect("compressed archive can be written");
    }
    #[cfg(not(feature = "xz"))]
    std::fs::write(archive, archive_buf).expect("archive can be written");

    std::fs::remove_dir_all(meta_dir).expect("temporary metadata can be removed");
}

#[test]
fn required_archives_never_fall_back_to_fixture_generation() {
    let temp = tempfile::TempDir::new().expect("temporary directory can be created");
    let archive = temp.path().join("missing.tar");
    let destination = temp.path().join("fixture");
    let mut generator_was_called = false;

    let result = run_fixture_generator_with_marker_handling(
        &archive,
        &destination,
        42,
        false,
        ArchivePolicy::Require,
        &Included,
        "from a test generator",
        |_| {
            generator_was_called = true;
            Ok(())
        },
    )
    .expect("a missing required archive is not an error");

    assert!(result.is_none(), "the unavailable fixture is reported to the caller");
    assert!(
        !generator_was_called,
        "an incompatible Git must never generate the fixture"
    );
    assert!(
        !destination.exists(),
        "an unavailable archive leaves no reusable cache directory"
    );
}

#[test]
#[serial_test::serial]
fn required_archives_are_extracted_even_when_archives_are_ignored() {
    let temp = tempfile::TempDir::new().expect("temporary directory can be created");
    let source = temp.path().join("source");
    std::fs::create_dir(&source).expect("source directory can be created");
    std::fs::write(source.join("payload"), "from archive").expect("payload can be written");
    let archive = temp.path().join(tar_extension());
    write_test_archive(&source, &archive, 42);
    let destination = temp.path().join("fixture");
    let _env = Env::new().set("GIX_TEST_IGNORE_ARCHIVES", "1");

    let result = run_fixture_generator_with_marker_handling(
        &archive,
        &destination,
        42,
        false,
        ArchivePolicy::Require,
        &Included,
        "from a test generator",
        |state| {
            assert!(matches!(state, FixtureState::Fresh(_)), "the generator is not invoked");
            std::fs::read_to_string(state.path().join("payload")).map_err(Into::into)
        },
    )
    .expect("the required archive can be extracted");

    assert_eq!(result.as_deref(), Some("from archive"));
}

/// Verify that forced execution with normal archive policy honors the explicit destination instead of extracting a
/// cached fixture there. This matters for fixtures such as linked worktrees whose administration files contain
/// absolute paths: extracting an archive into a new location would leave those paths pointing at the archived
/// location. In-place execution must also leave the canonical archive unchanged.
#[test]
fn forced_normal_fixtures_execute_in_place_instead_of_extracting_archives() {
    let temp = tempfile::TempDir::new().expect("temporary directory can be created");
    let source = temp.path().join("source");
    std::fs::create_dir(&source).expect("source directory can be created");
    std::fs::write(source.join("payload"), "from archive").expect("archive payload can be written");
    let archive = temp.path().join(tar_extension());
    write_test_archive(&source, &archive, 42);
    let archived_contents = std::fs::read(&archive).expect("archive can be read");
    let destination = temp.path().join("fixture");

    let result = run_fixture_generator_with_marker_handling(
        &archive,
        &destination,
        42,
        true,
        ArchivePolicy::Normal,
        &Included,
        "from a test generator",
        |state| {
            assert!(
                matches!(state, FixtureState::Uninitialized(_)),
                "forced normal fixtures are generated rather than extracted"
            );
            assert!(
                !state.path().join("payload").exists(),
                "archived contents were not copied into the explicit destination"
            );
            std::fs::write(state.path().join("payload"), "from script")?;
            std::fs::read_to_string(state.path().join("payload")).map_err(Into::into)
        },
    )
    .expect("the fixture can be generated in place");

    assert_eq!(result.as_deref(), Some("from script"));
    assert_eq!(
        std::fs::read(&archive).expect("archive can still be read"),
        archived_contents,
        "executing in a writable location does not replace the canonical archive"
    );
}

#[test]
#[serial_test::serial]
fn version_incompatible_writable_fixtures_use_required_archives_in_both_creation_modes() {
    let temp = tempfile::TempDir::new().expect("temporary directory can be created");
    let fixture_base = temp.path().join("tests/fixtures");
    let archive_dir = fixture_base.join(ARCHIVE_DIR_NAME);
    std::fs::create_dir_all(&archive_dir).expect("fixture directories can be created");
    let script = b"#!/bin/sh\nprintf from-script >payload\n";
    std::fs::write(fixture_base.join("make_required.sh"), script).expect("fixture script can be written");

    let source = temp.path().join("archive-source");
    std::fs::create_dir(&source).expect("archive source can be created");
    std::fs::write(source.join("payload"), "from archive").expect("archive payload can be written");
    let crc = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);
    let mut digest = crc.digest();
    digest.update(script);
    let object_hash = object_hash();
    let hash_suffix = if is_sha1(object_hash) {
        String::new()
    } else {
        format!("_{object_hash}")
    };
    write_test_archive(
        &source,
        &archive_dir.join(format!("make_required{hash_suffix}.{}", tar_extension())),
        digest.finalize(),
    );

    let _cwd = set_current_dir(temp.path()).expect("temporary fixture root is accessible");
    let _env = Env::new().set("GIX_TEST_IGNORE_ARCHIVES", "1");

    for mode in [Creation::CopyFromReadOnly, Creation::Execute] {
        let fixture =
            scripted_fixture_writable_with_args_with_git_version("make_required.sh", None::<String>, mode, |_| false)
                .expect("required archive can be loaded")
                .expect("matching required archive is available");
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("payload")).expect("archived payload can be read"),
            "from archive",
            "the fixture comes from the archive instead of the incompatible script"
        );
    }
}

#[test]
fn hash_kinds_are_classified_independently_of_gix_testtools_features() {
    for kind in gix_hash::Kind::all() {
        assert_eq!(is_sha1(*kind), kind.to_string() == "sha1");
    }
}

#[test]
fn stale_required_archives_are_unavailable_instead_of_generated() {
    let temp = tempfile::TempDir::new().expect("temporary directory can be created");
    let source = temp.path().join("source");
    std::fs::create_dir(&source).expect("source directory can be created");
    let archive = temp.path().join(tar_extension());
    write_test_archive(&source, &archive, 41);
    let destination = temp.path().join("fixture");
    let mut generator_was_called = false;

    let result = run_fixture_generator_with_marker_handling(
        &archive,
        &destination,
        42,
        false,
        ArchivePolicy::Require,
        &Included,
        "from a test generator",
        |_| {
            generator_was_called = true;
            Ok(())
        },
    )
    .expect("a stale required archive is not an error");

    assert!(result.is_none(), "the stale fixture is reported to the caller");
    assert!(
        !generator_was_called,
        "a stale archive must not fall back to generation"
    );
}

#[test]
fn required_archives_use_a_dedicated_cache_directory() {
    let fixture_base = Path::new("tests").join("fixtures");
    let (_, generated_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::default()),
        &1234,
        None,
    );
    let (_, preferred_archive_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::default()),
        &1234,
        Some("archive"),
    );
    let (_, required_archive_dir) = force_and_dir(
        None,
        &fixture_base,
        "scripted",
        Some(gix_hash::Kind::default()),
        &1234,
        Some("required-archive"),
    );

    assert_ne!(required_archive_dir, generated_dir);
    assert_ne!(required_archive_dir, preferred_archive_dir);
    assert!(
        required_archive_dir
            .components()
            .any(|component| component.as_os_str() == "required-archive")
    );
}
