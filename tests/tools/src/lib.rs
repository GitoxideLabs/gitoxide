//! Utilities for testing `gitoxide` crates, many of which might be useful for testing programs that use `git` in general.
//!
//! ## Feature Flags
#![cfg_attr(
    all(doc, feature = "document-features"),
    doc = ::document_features::document_features!()
)]
#![cfg_attr(all(doc, feature = "document-features"), feature(doc_cfg, doc_auto_cfg))]
#![deny(missing_docs)]

use std::{
    collections::BTreeMap,
    env,
    ffi::{OsStr, OsString},
    io::Read,
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

pub use bstr;
use bstr::ByteSlice;
use io_close::Close;
pub use is_ci;
pub use once_cell;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
pub use tempfile;

/// A result type to allow using the try operator `?` in unit tests.
///
/// Use it like so:
///
/// ```no_run
/// use gix_testtools::Result;
///
/// #[test]
/// fn this() -> Result {
///     let x: usize = "42".parse()?;
///     Ok(())
///
/// }
/// ```
pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// A wrapper for a running git-daemon process which is killed automatically on drop.
///
/// Note that we will swallow any errors, assuming that the test would have failed if the daemon crashed.
pub struct GitDaemon {
    child: std::process::Child,
    /// The base url under which all repositories are hosted, typically `git://127.0.0.1:port`.
    pub url: String,
}

impl Drop for GitDaemon {
    fn drop(&mut self) {
        self.child.kill().ok();
    }
}

static SCRIPT_IDENTITY: Lazy<Mutex<BTreeMap<PathBuf, u32>>> = Lazy::new(|| Mutex::new(BTreeMap::new()));

static EXCLUDE_LUT: Lazy<Mutex<Option<gix_worktree::Stack>>> = Lazy::new(|| {
    let cache = (|| {
        let (repo_path, _) = gix_discover::upwards(Path::new(".")).ok()?;
        let (gix_dir, work_tree) = repo_path.into_repository_and_work_tree_directories();
        let work_tree = work_tree?.canonicalize().ok()?;

        let mut buf = Vec::with_capacity(512);
        let case = if gix_fs::Capabilities::probe(&work_tree).ignore_case {
            gix_worktree::ignore::glob::pattern::Case::Fold
        } else {
            Default::default()
        };
        let state = gix_worktree::stack::State::IgnoreStack(gix_worktree::stack::state::Ignore::new(
            Default::default(),
            gix_worktree::ignore::Search::from_git_dir(
                &gix_dir,
                None,
                &mut buf,
                gix_worktree::stack::state::ignore::ParseIgnore {
                    support_precious: false,
                },
            )
            .ok()?,
            None,
            gix_worktree::stack::state::ignore::Source::WorktreeThenIdMappingIfNotSkipped,
            Default::default(),
        ));
        Some(gix_worktree::Stack::new(
            work_tree,
            state,
            case,
            buf,
            Default::default(),
        ))
    })();
    Mutex::new(cache)
});

#[cfg(windows)]
const GIT_PROGRAM: &str = "git.exe";
#[cfg(not(windows))]
const GIT_PROGRAM: &str = "git";

static GIT_CORE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let output = std::process::Command::new(GIT_PROGRAM)
        .arg("--exec-path")
        .output()
        .expect("can execute `git --exec-path`");

    assert!(output.status.success(), "`git --exec-path` failed");

    output
        .stdout
        .strip_suffix(b"\n")
        .expect("`git --exec-path` output to be well-formed")
        .to_os_str()
        .expect("no invalid UTF-8 in `--exec-path` except as OS allows")
        .into()
});

/// The major, minor and patch level of the git version on the system.
pub static GIT_VERSION: Lazy<(u8, u8, u8)> = Lazy::new(|| parse_git_version().expect("git version to be parsable"));

/// Define how [`scripted_fixture_writable_with_args()`] uses produces the writable copy.
pub enum Creation {
    /// Run the script once and copy the data from its output to the writable location.
    /// This is fast but won't work if absolute paths are produced by the script.
    ///
    /// ### Limitation
    ///
    /// Cannot handle symlinks currently. Waiting for [this PR](https://github.com/webdesus/fs_extra/pull/70).
    CopyFromReadOnly,
    /// Run the script in the writable location. That way, absolute paths match the location.
    ExecuteScript,
}

/// Returns true if the given `major`, `minor` and `patch` is smaller than the actual git version on the system
/// to facilitate skipping a test on the caller.
/// Will never return true on CI which is expected to have a recent enough git version.
///
/// # Panics
///
/// If `git` cannot be executed or if its version output cannot be parsed.
pub fn should_skip_as_git_version_is_smaller_than(major: u8, minor: u8, patch: u8) -> bool {
    if is_ci::cached() {
        return false; // CI should be made to use a recent git version, it should run there.
    }
    *GIT_VERSION < (major, minor, patch)
}

fn parse_git_version() -> Result<(u8, u8, u8)> {
    let output = std::process::Command::new(GIT_PROGRAM).arg("--version").output()?;
    git_version_from_bytes(&output.stdout)
}

fn git_version_from_bytes(bytes: &[u8]) -> Result<(u8, u8, u8)> {
    let mut numbers = bytes
        .split(|b| *b == b' ' || *b == b'\n')
        .nth(2)
        .expect("git version <version>")
        .split(|b| *b == b'.')
        .take(3)
        .map(|n| std::str::from_utf8(n).expect("valid utf8 in version number"))
        .map(u8::from_str);

    Ok((|| -> Result<_> {
        Ok((
            numbers.next().expect("major")?,
            numbers.next().expect("minor")?,
            numbers.next().expect("patch")?,
        ))
    })()
    .map_err(|err| {
        format!(
            "Could not parse version from output of 'git --version' ({:?}) with error: {}",
            bytes.to_str_lossy(),
            err
        )
    })?)
}

/// Set the current working dir to `new_cwd` and return a type that returns to the previous working dir on drop.
pub fn set_current_dir(new_cwd: impl AsRef<Path>) -> std::io::Result<AutoRevertToPreviousCWD> {
    let cwd = env::current_dir()?;
    env::set_current_dir(new_cwd)?;
    Ok(AutoRevertToPreviousCWD(cwd))
}

/// A utility to set the current working dir to the given value, on drop.
///
/// # Panics
///
/// Note that this will panic if the CWD cannot be set on drop.
#[derive(Debug)]
#[must_use]
pub struct AutoRevertToPreviousCWD(PathBuf);

impl Drop for AutoRevertToPreviousCWD {
    fn drop(&mut self) {
        env::set_current_dir(&self.0).unwrap();
    }
}

/// Run `git` in `working_dir` with all provided `args`.
pub fn run_git(working_dir: &Path, args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new(GIT_PROGRAM)
        .current_dir(working_dir)
        .args(args)
        .status()
}

/// Spawn a git daemon process to host all repository at or below `working_dir`.
pub fn spawn_git_daemon(working_dir: impl AsRef<Path>) -> std::io::Result<GitDaemon> {
    let mut ports: Vec<_> = (9419u16..9419 + 100).collect();
    fastrand::shuffle(&mut ports);
    let addr_at = |port| std::net::SocketAddr::from(([127, 0, 0, 1], port));
    let free_port = {
        let listener = std::net::TcpListener::bind(ports.into_iter().map(addr_at).collect::<Vec<_>>().as_slice())?;
        listener.local_addr().expect("listener address is available").port()
    };

    let child =
        std::process::Command::new(GIT_CORE_DIR.join(if cfg!(windows) { "git-daemon.exe" } else { "git-daemon" }))
            .current_dir(working_dir)
            .args(["--verbose", "--base-path=.", "--export-all", "--user-path"])
            .arg(format!("--port={free_port}"))
            .spawn()?;

    let server_addr = addr_at(free_port);
    for time in gix_lock::backoff::Quadratic::default_with_random() {
        std::thread::sleep(time);
        if std::net::TcpStream::connect(server_addr).is_ok() {
            break;
        }
    }
    Ok(GitDaemon {
        child,
        url: format!("git://{server_addr}"),
    })
}

#[derive(Copy, Clone)]
enum DirectoryRoot {
    IntegrationTest,
    StandaloneTest,
}

/// Don't add a suffix to the archive name as `args` are platform dependent, none-deterministic,
/// or otherwise don't influence the content of the archive.
/// Note that this also means that `args` won't be used to control the hash of the archive itself.
#[derive(Copy, Clone)]
enum ArgsInHash {
    Yes,
    No,
}

/// Return the path to the `<crate-root>/tests/fixtures/<path>` directory.
pub fn fixture_path(path: impl AsRef<Path>) -> PathBuf {
    fixture_path_inner(path, DirectoryRoot::IntegrationTest)
}

/// Return the path to the `<crate-root>/fixtures/<path>` directory.
pub fn fixture_path_standalone(path: impl AsRef<Path>) -> PathBuf {
    fixture_path_inner(path, DirectoryRoot::StandaloneTest)
}
/// Return the path to the `<crate-root>/tests/fixtures/<path>` directory.
fn fixture_path_inner(path: impl AsRef<Path>, root: DirectoryRoot) -> PathBuf {
    match root {
        DirectoryRoot::StandaloneTest => PathBuf::from("fixtures").join(path.as_ref()),
        DirectoryRoot::IntegrationTest => PathBuf::from("tests").join("fixtures").join(path.as_ref()),
    }
}

/// Load the fixture from `<crate-root>/tests/fixtures/<path>` and return its data, or _panic_.
pub fn fixture_bytes(path: impl AsRef<Path>) -> Vec<u8> {
    fixture_bytes_inner(path, DirectoryRoot::IntegrationTest)
}

/// Like [`scripted_fixture_writable`], but does not prefix the fixture directory with `tests`
pub fn fixture_bytes_standalone(path: impl AsRef<Path>) -> Vec<u8> {
    fixture_bytes_inner(path, DirectoryRoot::StandaloneTest)
}

fn fixture_bytes_inner(path: impl AsRef<Path>, root: DirectoryRoot) -> Vec<u8> {
    match std::fs::read(fixture_path_inner(path.as_ref(), root)) {
        Ok(res) => res,
        Err(_) => panic!("File at '{}' not found", path.as_ref().display()),
    }
}

/// Run the executable at `script_name`, like `make_repo.sh` or `my_setup.py` to produce a read-only directory to which
/// the path is returned.
///
/// Note that it persists and the script at `script_name` will only be executed once if it ran without error.
///
/// ### Automatic Archive Creation
///
/// In order to speed up CI and even local runs should the cache get purged, the result of each script run
/// is automatically placed into a compressed _tar_ archive.
/// If a script result doesn't exist, these will be checked first and extracted if present, which they are by default.
/// This behaviour can be prohibited by setting the `GIX_TEST_IGNORE_ARCHIVES` to any value.
///
/// To speed CI up, one can add these archives to the repository. Since LFS is not currently being used, it is
/// important to check their size first, though in most cases generated archives will not be very large.
///
/// #### Disable Archive Creation
///
/// If archives aren't useful, they can be disabled by using `.gitignore` specifications.
/// That way it's trivial to prevent creation of all archives with `generated-archives/*.tar{.xz}` in the root
/// or more specific `.gitignore` configurations in lower levels of the work tree.
///
/// The latter is useful if the script's output is platform specific.
pub fn scripted_fixture_read_only(script_name: impl AsRef<Path>) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args(script_name, None::<String>)
}

/// Like [`scripted_fixture_read_only`], but does not prefix the fixture directory with `tests`
pub fn scripted_fixture_read_only_standalone(script_name: impl AsRef<Path>) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args_standalone(script_name, None::<String>)
}

/// Run the executable at `script_name`, like `make_repo.sh` to produce a writable directory to which
/// the tempdir is returned. It will be removed automatically, courtesy of [`tempfile::TempDir`].
///
/// Note that `script_name` is only executed once, so the data can be copied from its read-only location.
pub fn scripted_fixture_writable(script_name: impl AsRef<Path>) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args(script_name, None::<String>, Creation::CopyFromReadOnly)
}

/// Like [`scripted_fixture_writable`], but does not prefix the fixture directory with `tests`
pub fn scripted_fixture_writable_standalone(script_name: &str) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args_standalone(script_name, None::<String>, Creation::CopyFromReadOnly)
}

/// Like [`scripted_fixture_writable()`], but passes `args` to `script_name` while providing control over
/// the way files are created with `mode`.
pub fn scripted_fixture_writable_with_args(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
    mode: Creation,
) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args_inner(script_name, args, mode, DirectoryRoot::IntegrationTest, ArgsInHash::Yes)
}

/// Like [`scripted_fixture_writable()`], but passes `args` to `script_name` while providing control over
/// the way files are created with `mode`.
///
/// See [`scripted_fixture_read_only_with_args_single_archive()`] for important details on what `single_archive` means.
pub fn scripted_fixture_writable_with_args_single_archive(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
    mode: Creation,
) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args_inner(script_name, args, mode, DirectoryRoot::IntegrationTest, ArgsInHash::No)
}

/// Like [`scripted_fixture_writable_with_args`], but does not prefix the fixture directory with `tests`
pub fn scripted_fixture_writable_with_args_standalone(
    script_name: &str,
    args: impl IntoIterator<Item = impl Into<String>>,
    mode: Creation,
) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args_inner(script_name, args, mode, DirectoryRoot::StandaloneTest, ArgsInHash::Yes)
}

/// Like [`scripted_fixture_writable_with_args`], but does not prefix the fixture directory with `tests`
///
/// See [`scripted_fixture_read_only_with_args_single_archive()`] for important details on what `single_archive` means.
pub fn scripted_fixture_writable_with_args_standalone_single_archive(
    script_name: &str,
    args: impl IntoIterator<Item = impl Into<String>>,
    mode: Creation,
) -> Result<tempfile::TempDir> {
    scripted_fixture_writable_with_args_inner(script_name, args, mode, DirectoryRoot::StandaloneTest, ArgsInHash::No)
}

fn scripted_fixture_writable_with_args_inner(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
    mode: Creation,
    root: DirectoryRoot,
    args_in_hash: ArgsInHash,
) -> Result<tempfile::TempDir> {
    let dst = tempfile::TempDir::new()?;
    Ok(match mode {
        Creation::CopyFromReadOnly => {
            let ro_dir = scripted_fixture_read_only_with_args_inner(script_name, args, None, root, args_in_hash)?;
            copy_recursively_into_existing_dir(ro_dir, dst.path())?;
            dst
        }
        Creation::ExecuteScript => {
            scripted_fixture_read_only_with_args_inner(script_name, args, dst.path().into(), root, args_in_hash)?;
            dst
        }
    })
}

/// A utility to copy the entire contents of `src_dir` into `dst_dir`.
pub fn copy_recursively_into_existing_dir(src_dir: impl AsRef<Path>, dst_dir: impl AsRef<Path>) -> std::io::Result<()> {
    fs_extra::copy_items(
        &std::fs::read_dir(src_dir)?
            .map(|e| e.map(|e| e.path()))
            .collect::<std::result::Result<Vec<_>, _>>()?,
        dst_dir,
        &fs_extra::dir::CopyOptions {
            overwrite: false,
            skip_exist: false,
            copy_inside: false,
            content_only: false,
            ..Default::default()
        },
    )
    .map_err(std::io::Error::other)?;
    Ok(())
}

/// Like `scripted_fixture_read_only()`], but passes `args` to `script_name`.
pub fn scripted_fixture_read_only_with_args(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args_inner(script_name, args, None, DirectoryRoot::IntegrationTest, ArgsInHash::Yes)
}

/// Like `scripted_fixture_read_only()`], but passes `args` to `script_name`.
///
/// Also, don't add a suffix to the archive name as `args` are platform dependent, none-deterministic,
/// or otherwise don't influence the content of the archive.
/// Note that this also means that `args` won't be used to control the hash of the archive itself.
///
/// Sometimes, this should be combined with adding the archive name to `.gitignore` to prevent its creation
/// in the first place.
///
/// Note that suffixing archives by default helps to learn what calls are made, and forces the author to
/// think about what should be done to get it right.
pub fn scripted_fixture_read_only_with_args_single_archive(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args_inner(script_name, args, None, DirectoryRoot::IntegrationTest, ArgsInHash::No)
}

/// Like [`scripted_fixture_read_only_with_args()`], but does not prefix the fixture directory with `tests`
pub fn scripted_fixture_read_only_with_args_standalone(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args_inner(script_name, args, None, DirectoryRoot::StandaloneTest, ArgsInHash::Yes)
}

/// Like [`scripted_fixture_read_only_with_args_standalone()`], only has a single archive.
pub fn scripted_fixture_read_only_with_args_standalone_single_archive(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
) -> Result<PathBuf> {
    scripted_fixture_read_only_with_args_inner(script_name, args, None, DirectoryRoot::StandaloneTest, ArgsInHash::No)
}

fn scripted_fixture_read_only_with_args_inner(
    script_name: impl AsRef<Path>,
    args: impl IntoIterator<Item = impl Into<String>>,
    destination_dir: Option<&Path>,
    root: DirectoryRoot,
    args_in_hash: ArgsInHash,
) -> Result<PathBuf> {
    // Assure tempfiles get removed when aborting the test.
    gix_tempfile::signal::setup(
        gix_tempfile::signal::handler::Mode::DeleteTempfilesOnTerminationAndRestoreDefaultBehaviour,
    );

    let script_location = script_name.as_ref();
    let script_path = fixture_path_inner(script_location, root);

    // keep this lock to assure we don't return unfinished directories for threaded callers
    let args: Vec<String> = args.into_iter().map(Into::into).collect();
    let script_identity = {
        let mut map = SCRIPT_IDENTITY.lock();
        map.entry(args.iter().fold(script_path.clone(), |p, a| p.join(a)))
            .or_insert_with(|| {
                let crc_value = crc::Crc::<u32>::new(&crc::CRC_32_CKSUM);
                let mut crc_digest = crc_value.digest();
                crc_digest.update(&std::fs::read(&script_path).unwrap_or_else(|err| {
                    panic!(
                        "file {script_path} in CWD '{cwd}' could not be read: {err}",
                        cwd = env::current_dir().expect("valid cwd").display(),
                        script_path = script_path.display(),
                    )
                }));
                for arg in &args {
                    crc_digest.update(arg.as_bytes());
                }
                crc_digest.finalize()
            })
            .to_owned()
    };

    let script_basename = script_location.file_stem().unwrap_or(script_location.as_os_str());
    let archive_file_path = fixture_path_inner(
        {
            let suffix = match args_in_hash {
                ArgsInHash::Yes => {
                    let mut suffix = args.join("_");
                    if !suffix.is_empty() {
                        suffix.insert(0, '_');
                    }
                    suffix.replace(['\\', '/', ' ', '.'], "_")
                }
                ArgsInHash::No => "".into(),
            };
            Path::new("generated-archives").join(format!(
                "{}{suffix}.tar{}",
                script_basename.to_str().expect("valid UTF-8"),
                if cfg!(feature = "xz") { ".xz" } else { "" }
            ))
        },
        root,
    );
    let (force_run, script_result_directory) = destination_dir.map_or_else(
        || {
            let dir = fixture_path_inner(
                Path::new("generated-do-not-edit").join(script_basename).join(format!(
                    "{}-{}",
                    script_identity,
                    family_name()
                )),
                root,
            );
            (false, dir)
        },
        |d| (true, d.to_owned()),
    );

    // We may that destination_dir is already unique (i.e. temp-dir) - thus there is no need for a lock,
    // and we can execute scripts in parallel.
    let _marker = destination_dir
        .is_none()
        .then(|| {
            gix_lock::Marker::acquire_to_hold_resource(
                script_basename,
                gix_lock::acquire::Fail::AfterDurationWithBackoff(Duration::from_secs(6 * 60)),
                None,
            )
        })
        .transpose()?;
    let failure_marker = script_result_directory.join("_invalid_state_due_to_script_failure_");
    if force_run || !script_result_directory.is_dir() || failure_marker.is_file() {
        if failure_marker.is_file() {
            std::fs::remove_dir_all(&script_result_directory).map_err(|err| {
                format!("Failed to remove '{script_result_directory}', please try to do that by hand. Original error: {err}",
                        script_result_directory = script_result_directory.display())
            })?;
        }
        std::fs::create_dir_all(&script_result_directory)?;
        let script_identity_for_archive = match args_in_hash {
            ArgsInHash::Yes => script_identity,
            ArgsInHash::No => 0,
        };
        match extract_archive(
            &archive_file_path,
            &script_result_directory,
            script_identity_for_archive,
        ) {
            Ok((archive_id, platform)) => {
                eprintln!(
                    "Extracted fixture from archive '{}' ({}, {:?})",
                    archive_file_path.display(),
                    archive_id,
                    platform
                );
            }
            Err(err) => {
                if err.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("failed to extract '{}': {}", archive_file_path.display(), err);
                    std::fs::remove_dir_all(&script_result_directory)
                        .map_err(|err| {
                            format!("Failed to remove '{script_result_directory}', please try to do that by hand. Original error: {err}",
                                    script_result_directory = script_result_directory.display())
                        })?;
                    std::fs::create_dir_all(&script_result_directory)?;
                } else if !is_excluded(&archive_file_path) {
                    eprintln!(
                        "Archive at '{}' not found, creating fixture using script '{}'",
                        archive_file_path.display(),
                        script_location.display()
                    );
                }
                let script_absolute_path = env::current_dir()?.join(script_path);
                let mut cmd = std::process::Command::new(&script_absolute_path);
                let output = match configure_command(&mut cmd, &args, &script_result_directory).output() {
                    Ok(out) => out,
                    Err(err)
                    if err.kind() == std::io::ErrorKind::PermissionDenied || err.raw_os_error() == Some(193) /* windows */ =>
                        {
                            cmd = std::process::Command::new(bash_program());
                            configure_command(cmd.arg(script_absolute_path), &args, &script_result_directory).output()?
                        }
                    Err(err) => return Err(err.into()),
                };
                if !output.status.success() {
                    write_failure_marker(&failure_marker);
                    eprintln!("stdout: {}", output.stdout.as_bstr());
                    eprintln!("stderr: {}", output.stderr.as_bstr());
                    return Err(format!("fixture script of {cmd:?} failed").into());
                }
                create_archive_if_we_should(
                    &script_result_directory,
                    &archive_file_path,
                    script_identity_for_archive,
                )
                .inspect_err(|_err| {
                    write_failure_marker(&failure_marker);
                })?;
            }
        }
    }
    Ok(script_result_directory)
}

#[cfg(windows)]
const NULL_DEVICE: &str = "nul"; // See `gix_path::env::git::NULL_DEVICE` on why this form is used.
#[cfg(not(windows))]
const NULL_DEVICE: &str = "/dev/null";

fn configure_command<'a, I: IntoIterator<Item = S>, S: AsRef<OsStr>>(
    cmd: &'a mut std::process::Command,
    args: I,
    script_result_directory: &Path,
) -> &'a mut std::process::Command {
    // For simplicity, we extend the `MSYS` variable from our own environment. This disregards
    // state from any prior `cmd.env("MSYS")` or `cmd.env_remove("MSYS")` calls. Such calls should
    // either be avoided, or made after this function returns (but before spawning the command).
    let mut msys_for_git_bash_on_windows = env::var_os("MSYS").unwrap_or_default();
    msys_for_git_bash_on_windows.push(" winsymlinks:nativestrict");
    cmd.args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .current_dir(script_result_directory)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .env("MSYS", msys_for_git_bash_on_windows)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
        .env("GIT_TERMINAL_PROMPT", "false")
        .env("GIT_AUTHOR_DATE", "2000-01-01 00:00:00 +0000")
        .env("GIT_AUTHOR_EMAIL", "author@example.com")
        .env("GIT_AUTHOR_NAME", "author")
        .env("GIT_COMMITTER_DATE", "2000-01-02 00:00:00 +0000")
        .env("GIT_COMMITTER_EMAIL", "committer@example.com")
        .env("GIT_COMMITTER_NAME", "committer")
        .env("GIT_CONFIG_COUNT", "4")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .env("GIT_CONFIG_KEY_1", "tag.gpgsign")
        .env("GIT_CONFIG_VALUE_1", "false")
        .env("GIT_CONFIG_KEY_2", "init.defaultBranch")
        .env("GIT_CONFIG_VALUE_2", "main")
        .env("GIT_CONFIG_KEY_3", "protocol.file.allow")
        .env("GIT_CONFIG_VALUE_3", "always")
}

/// Get the path attempted as a `bash` interpreter, for fixture scripts having no `#!` we can use.
///
/// This is rarely called on Unix-like systems, provided that fixture scripts have usable shebang
/// (`#!`) lines and are marked executable. However, Windows does not recognize `#!` when executing
/// a file. If all fixture scripts that cannot be directly executed are `bash` scripts or can be
/// treated as such, fixture generation still works on Windows, as long as this function manages to
/// find or guess a suitable `bash` interpreter.
///
/// ### Search order
///
/// This function is used internally. It is public to facilitate diagnostic use. The following
/// details are subject to change without warning, and changes are treated as non-breaking.
///
/// The `bash.exe` found in a path search is not always suitable on Windows. This is mainly because
/// `bash.exe` in `System32`, which is associated with WSL, would often be found first. But even
/// where that is not the case, the best `bash.exe` to use to run fixture scripts to set up Git
/// repositories for testing is usually one associated with Git for Windows, even if some other
/// `bash.exe` would be found in a path search. Currently, the search order we use is as follows:
///
/// 1. The shim `bash.exe`, which sets environment variables when run and is, on some systems,
///    needed to find the POSIX utilities that scripts need (or correct versions of them).
///
/// 2. The non-shim `bash.exe`, which is sometimes available even when the shim is not available.
///    This is mainly because the Git for Windows SDK does not come with a `bash.exe` shim.
///
/// 3. As a fallback, the simple name `bash.exe`, which triggers a path search when run.
///
/// On non-Windows systems, the simple name `bash` is used, which triggers a path search when run.
pub fn bash_program() -> &'static Path {
    // TODO(deps): Unify with `gix_path::env::shell()` by having both call a more general function
    //             in `gix-path`. See https://github.com/GitoxideLabs/gitoxide/issues/1886.
    static GIT_BASH: Lazy<PathBuf> = Lazy::new(|| {
        if cfg!(windows) {
            GIT_CORE_DIR
                .ancestors()
                .nth(3)
                .map(OsStr::new)
                .iter()
                .flat_map(|prefix| {
                    // Go down to places `bash.exe` usually is. Keep using `/` separators, not `\`.
                    ["/bin/bash.exe", "/usr/bin/bash.exe"].into_iter().map(|suffix| {
                        let mut raw_path = (*prefix).to_owned();
                        raw_path.push(suffix);
                        raw_path
                    })
                })
                .map(PathBuf::from)
                .find(|bash| bash.is_file())
                .unwrap_or_else(|| "bash.exe".into())
        } else {
            "bash".into()
        }
    });
    GIT_BASH.as_ref()
}

fn write_failure_marker(failure_marker: &Path) {
    std::fs::write(failure_marker, []).ok();
}

fn should_skip_all_archive_creation() -> bool {
    // On Windows, we fail to remove the meta_dir and can't do anything about it, which means tests will see more
    // in the directory than they should which makes them fail. It's probably a bad idea to generate archives on Windows
    // anyway. Either Unix is portable OR no archive is created anywhere. This also means that Windows users can't create
    // archives, but that's not a deal-breaker.
    cfg!(windows) || (is_ci::cached() && env::var_os("GIX_TEST_CREATE_ARCHIVES_EVEN_ON_CI").is_none())
}

fn is_lfs_pointer_file(path: &Path) -> bool {
    const PREFIX: &[u8] = b"version https://git-lfs";
    let mut buf = [0_u8; PREFIX.len()];
    std::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .is_ok_and(|mut f| f.read_exact(&mut buf).is_ok_and(|_| buf.starts_with(PREFIX)))
}

/// The `script_identity` will be baked into the soon to be created `archive` as it identifies the script
/// that created the contents of `source_dir`.
fn create_archive_if_we_should(source_dir: &Path, archive: &Path, script_identity: u32) -> std::io::Result<()> {
    if should_skip_all_archive_creation() || is_excluded(archive) {
        return Ok(());
    }
    if is_lfs_pointer_file(archive) {
        eprintln!(
            "Refusing to overwrite `gix-lfs` pointer file at \"{}\" - git lfs might not be properly installed.",
            archive.display()
        );
        return Ok(());
    }
    std::fs::create_dir_all(archive.parent().expect("archive is a file"))?;

    let meta_dir = populate_meta_dir(source_dir, script_identity)?;
    let res = (move || {
        let mut buf = Vec::<u8>::new();
        {
            let mut ar = tar::Builder::new(&mut buf);
            ar.mode(tar::HeaderMode::Deterministic);
            ar.follow_symlinks(false);
            ar.append_dir_all(".", source_dir)?;
            ar.finish()?;
        }
        #[cfg_attr(feature = "xz", allow(unused_mut))]
        let mut archive = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(archive)?;
        #[cfg(feature = "xz")]
        {
            let mut xz_write = xz2::write::XzEncoder::new(archive, 3);
            std::io::copy(&mut &*buf, &mut xz_write)?;
            xz_write.finish()?.close()
        }
        #[cfg(not(feature = "xz"))]
        {
            use std::io::Write;
            archive.write_all(&buf)?;
            archive.close()
        }
    })();
    #[cfg(not(windows))]
    std::fs::remove_dir_all(meta_dir)?;
    #[cfg(windows)]
    std::fs::remove_dir_all(meta_dir).ok(); // it really can't delete these directories for some reason (even after 10 seconds)

    res
}

fn is_excluded(archive: &Path) -> bool {
    let mut lut = EXCLUDE_LUT.lock();
    lut.as_mut()
        .and_then(|cache| {
            let archive = env::current_dir().ok()?.join(archive);
            let relative_path = archive.strip_prefix(cache.base()).ok()?;
            cache
                .at_path(
                    relative_path,
                    Some(gix_worktree::index::entry::Mode::FILE),
                    &gix_worktree::object::find::Never,
                )
                .ok()?
                .is_excluded()
                .into()
        })
        .unwrap_or(false)
}

const META_DIR_NAME: &str = "__gitoxide_meta__";
const META_IDENTITY: &str = "identity";
const META_GIT_VERSION: &str = "git-version";

fn populate_meta_dir(destination_dir: &Path, script_identity: u32) -> std::io::Result<PathBuf> {
    let meta_dir = destination_dir.join(META_DIR_NAME);
    std::fs::create_dir_all(&meta_dir)?;
    std::fs::write(
        meta_dir.join(META_IDENTITY),
        format!("{}-{}", script_identity, family_name()).as_bytes(),
    )?;
    std::fs::write(
        meta_dir.join(META_GIT_VERSION),
        std::process::Command::new(GIT_PROGRAM)
            .arg("--version")
            .output()?
            .stdout,
    )?;
    Ok(meta_dir)
}

/// `required_script_identity` is the identity of the script that generated the state that is contained in `archive`.
/// If this is not the case, the arvhive will be ignored.
fn extract_archive(
    archive: &Path,
    destination_dir: &Path,
    required_script_identity: u32,
) -> std::io::Result<(u32, Option<String>)> {
    let archive_buf: Vec<u8> = {
        let mut buf = Vec::new();
        #[cfg_attr(feature = "xz", allow(unused_mut))]
        let mut input_archive = std::fs::File::open(archive)?;
        if env::var_os("GIX_TEST_IGNORE_ARCHIVES").is_some() {
            return Err(std::io::Error::other(format!(
                "Ignoring archive at '{}' as GIX_TEST_IGNORE_ARCHIVES is set.",
                archive.display()
            )));
        }
        #[cfg(feature = "xz")]
        {
            let mut decoder = xz2::bufread::XzDecoder::new(std::io::BufReader::new(input_archive));
            std::io::copy(&mut decoder, &mut buf)?;
        }
        #[cfg(not(feature = "xz"))]
        {
            input_archive.read_to_end(&mut buf)?;
        }
        buf
    };

    let mut entry_buf = Vec::<u8>::new();
    let (archive_identity, platform): (u32, _) = tar::Archive::new(std::io::Cursor::new(&mut &*archive_buf))
        .entries_with_seek()?
        .filter_map(std::result::Result::ok)
        .find_map(|mut e: tar::Entry<'_, _>| {
            let path = e.path().ok()?;
            if path.parent()?.file_name()? == META_DIR_NAME && path.file_name()? == META_IDENTITY {
                entry_buf.clear();
                e.read_to_end(&mut entry_buf).ok()?;
                let mut tokens = entry_buf.to_str().ok()?.trim().splitn(2, '-');
                match (tokens.next(), tokens.next()) {
                    (Some(id), platform) => Some((id.parse().ok()?, platform.map(ToOwned::to_owned))),
                    _ => None,
                }
            } else {
                None
            }
        })
        .ok_or_else(|| std::io::Error::other("BUG: Could not find meta directory in our own archive"))
        .map_err(|err| {
            std::io::Error::other(format!(
                "Could not extract archive at '{archive}': {err}",
                archive = archive.display()
            ))
        })?;
    if archive_identity != required_script_identity {
        eprintln!(
            "Ignoring archive at '{}' as its generating script changed",
            archive.display()
        );
        return Err(std::io::ErrorKind::NotFound.into());
    }

    for entry in tar::Archive::new(&mut &*archive_buf).entries()? {
        let mut entry = entry?;
        let path = entry.path()?;
        if path.to_str() == Some(META_DIR_NAME) || path.parent().and_then(Path::to_str) == Some(META_DIR_NAME) {
            continue;
        }
        entry.unpack_in(destination_dir)?;
    }
    Ok((archive_identity, platform))
}

/// Transform a verbose parser errors from raw bytes into a `BStr` to make printing/debugging human-readable.
pub fn to_bstr_err(
    err: winnow::error::ErrMode<winnow::error::TreeError<&[u8], winnow::error::StrContext>>,
) -> winnow::error::TreeError<&winnow::stream::BStr, winnow::error::StrContext> {
    let err = err.into_inner().expect("not a streaming parser");
    err.map_input(winnow::stream::BStr::new)
}

fn family_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else {
        "unix"
    }
}

/// A utility to set and unset environment variables, while restoring or removing them on drop.
#[derive(Default)]
pub struct Env<'a> {
    altered_vars: Vec<(&'a str, Option<OsString>)>,
}

impl<'a> Env<'a> {
    /// Create a new instance.
    pub fn new() -> Self {
        Env {
            altered_vars: Vec::new(),
        }
    }

    /// Set `var` to `value`.
    pub fn set(mut self, var: &'a str, value: impl Into<String>) -> Self {
        let prev = env::var_os(var);
        env::set_var(var, value.into());
        self.altered_vars.push((var, prev));
        self
    }

    /// Unset `var`.
    pub fn unset(mut self, var: &'a str) -> Self {
        let prev = env::var_os(var);
        env::remove_var(var);
        self.altered_vars.push((var, prev));
        self
    }
}

impl Drop for Env<'_> {
    fn drop(&mut self) {
        for (var, prev_value) in self.altered_vars.iter().rev() {
            match prev_value {
                Some(value) => env::set_var(var, value),
                None => env::remove_var(var),
            }
        }
    }
}

/// Check data structure size, comparing strictly on 64-bit targets.
///
/// - On 32-bit targets, checks if `actual_size` is at most `expected_64_bit_size`.
/// - On 64-bit targets, checks if `actual_size` is exactly `expected_64_bit_size`.
///
/// This is for assertions about the size of data structures, when the goal is to keep them from
/// growing too large even across breaking changes. Such assertions must always fail when data
/// structures grow larger than they have ever been, for which `<=` is enough. But it also helps to
/// know when they have shrunk unexpectedly. They may shrink, other changes may rely on the smaller
/// size for acceptable performance, and then they may grow again to their earlier size.
///
/// The problem with `==` is that data structures are often smaller on 32-bit targets. This could
/// be addressed by asserting separate exact 64-bit and 32-bit sizes. But sizes may also differ
/// across 32-bit targets, due to ABI and layout/packing details. That can happen across 64-bit
/// targets too, but it seems less common.
///
/// For those reasons, this function does a `==` on 64-bit targets, but a `<=` on 32-bit targets.
pub fn size_ok(actual_size: usize, expected_64_bit_size: usize) -> bool {
    #[cfg(target_pointer_width = "64")]
    return actual_size == expected_64_bit_size;
    #[cfg(target_pointer_width = "32")]
    return actual_size <= expected_64_bit_size;
}

/// Get the umask in a way that is safe, but may be too slow for use outside of tests.
#[cfg(unix)]
pub fn umask() -> u32 {
    let output = std::process::Command::new("/bin/sh")
        .args(["-c", "umask"])
        .output()
        .expect("can execute `sh -c umask`");
    assert!(output.status.success(), "`sh -c umask` failed");
    assert_eq!(output.stderr.as_bstr(), "", "`sh -c umask` unexpected message");
    let text = output.stdout.to_str().expect("valid Unicode").trim();
    u32::from_str_radix(text, 8).expect("parses as octal number")
}

#[cfg(test)]
mod tests {
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

        let mut cmd = std::process::Command::new(GIT_PROGRAM);
        cmd.env("GIT_CONFIG_SYSTEM", SCOPE_ENV_VALUE);
        cmd.env("GIT_CONFIG_GLOBAL", SCOPE_ENV_VALUE);
        configure_command(&mut cmd, ["config", "-l", "--show-origin"], temp.path());

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
    #[cfg(windows)]
    fn bash_program_ok_for_platform() {
        let path = bash_program();
        assert!(path.is_absolute());

        let for_version = std::process::Command::new(path)
            .arg("--version")
            .output()
            .expect("can pass it `--version`");
        assert!(for_version.status.success(), "passing `--version` succeeds");
        let version_line = for_version
            .stdout
            .lines()
            .nth(0)
            .expect("`--version` output has first line");
        assert!(
            version_line.ends_with(b"-pc-msys)"), // On Windows, "-pc-linux-gnu)" would be WSL.
            "it is an MSYS bash (such as Git Bash)"
        );

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
}
