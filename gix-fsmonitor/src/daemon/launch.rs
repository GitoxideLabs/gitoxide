use std::{
    ffi::{OsStr, OsString},
    path::Path,
};

use gix_error::{Result, ResultExt, message};

use crate::server;

#[cfg(not(windows))]
pub(crate) type Child = std::process::Child;

pub(crate) fn spawn(
    context: &server::Context,
    reopen_git_dir: &Path,
    arguments: &[OsString],
    idle_timeout: u64,
    ready_file: &Path,
) -> Result<Child> {
    let arguments = run_arguments(arguments, idle_timeout, ready_file);
    let environment = environment(context, reopen_git_dir);
    native_spawn(&arguments, &environment).or_raise(|| message("starting the filesystem monitor from this executable"))
}

fn run_arguments(arguments: &[OsString], idle_timeout: u64, ready_file: &Path) -> Vec<OsString> {
    let mut arguments = arguments.to_vec();
    arguments.extend([
        "--idle-timeout".into(),
        idle_timeout.to_string().into(),
        "--ready-file".into(),
        ready_file.as_os_str().to_owned(),
    ]);
    arguments
}

fn redirects_repository(name: &OsStr) -> bool {
    matches!(
        name.to_string_lossy().to_ascii_uppercase().as_str(),
        "GIT_DIR"
            | "GIT_WORK_TREE"
            | "GIT_COMMON_DIR"
            | "GIT_INDEX_FILE"
            | "GIT_OBJECT_DIRECTORY"
            | "GIT_ALTERNATE_OBJECT_DIRECTORIES"
            | "GIT_CEILING_DIRECTORIES"
            | "GIT_DISCOVERY_ACROSS_FILESYSTEM"
            | "GIT_NAMESPACE"
            | "GIT_PREFIX"
            | "GIT_SHALLOW_FILE"
    )
}

fn environment(context: &server::Context, reopen_git_dir: &Path) -> Vec<(OsString, OsString)> {
    let mut environment: Vec<_> = std::env::vars_os()
        .filter(|(name, _)| !redirects_repository(name))
        .collect();
    environment.extend([
        ("GIT_DIR".into(), reopen_git_dir.as_os_str().to_owned()),
        ("GIT_WORK_TREE".into(), context.worktree.as_os_str().to_owned()),
        ("GIT_COMMON_DIR".into(), context.common_dir.as_os_str().to_owned()),
    ]);
    environment
}

#[cfg(unix)]
#[expect(unsafe_code)]
fn native_spawn(arguments: &[OsString], environment: &[(OsString, OsString)]) -> std::io::Result<Child> {
    use std::{
        io,
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(arguments)
        .env_clear()
        .envs(environment.iter().cloned())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: Only async-signal-safe setsid runs between fork and exec.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
    command.spawn()
}

#[cfg(not(any(unix, windows)))]
fn native_spawn(_arguments: &[OsString], _environment: &[(OsString, OsString)]) -> std::io::Result<Child> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "detached filesystem monitors are unavailable",
    ))
}

#[cfg(windows)]
pub(crate) use windows::Child;
#[cfg(windows)]
use windows::native_spawn;

#[cfg(any(windows, test))]
fn quote(argument: &[u16], output: &mut Vec<u16>) {
    output.push(u16::from(b'"'));
    let mut slashes = 0;
    for &character in argument {
        if character == u16::from(b'\\') {
            slashes += 1;
            continue;
        }
        output.extend(std::iter::repeat_n(
            u16::from(b'\\'),
            if character == u16::from(b'"') {
                2 * slashes + 1
            } else {
                slashes
            },
        ));
        output.push(character);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(u16::from(b'\\'), 2 * slashes));
    output.push(u16::from(b'"'));
}

#[cfg(windows)]
mod windows {
    use super::*;
    use std::{
        io,
        os::windows::{
            ffi::{OsStrExt, OsStringExt},
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
            process::ExitStatusExt,
        },
        path::PathBuf,
        process::ExitStatus,
    };
    use windows_sys::Win32::{
        Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT},
        System::Threading::{
            CREATE_NEW_PROCESS_GROUP, CREATE_UNICODE_ENVIRONMENT, CreateProcessW, DETACHED_PROCESS, GetExitCodeProcess,
            INFINITE, PROCESS_INFORMATION, STARTUPINFOW, TerminateProcess, WaitForSingleObject,
        },
    };

    pub(crate) struct Child {
        process: OwnedHandle,
        id: u32,
    }

    #[expect(unsafe_code)]
    pub(super) fn native_spawn(arguments: &[OsString], environment: &[(OsString, OsString)]) -> io::Result<Child> {
        let executable_path = std::env::current_exe()?;
        let executable = wide(executable_path.as_os_str())?;
        let directory = working_directory(&std::env::current_dir()?)?;
        let mut command = Vec::new();
        for argument in std::iter::once(executable_path.as_os_str()).chain(arguments.iter().map(OsString::as_os_str)) {
            if !command.is_empty() {
                command.push(u16::from(b' '));
            }
            let argument: Vec<_> = argument.encode_wide().collect();
            if argument.contains(&0) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in daemon argument"));
            }
            quote(&argument, &mut command);
        }
        command.push(0);
        let mut entries: Vec<_> = environment.iter().collect();
        entries.sort_by_cached_key(|(name, _)| name.to_string_lossy().to_ascii_uppercase());
        let mut environment = Vec::new();
        for (name, value) in entries {
            let mut entry: Vec<_> = name
                .encode_wide()
                .chain(std::iter::once(u16::from(b'=')))
                .chain(value.encode_wide())
                .collect();
            if entry.contains(&0) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in daemon environment"));
            }
            environment.append(&mut entry);
            environment.push(0);
        }
        environment.push(0);
        let startup = STARTUPINFOW {
            cb: std::mem::size_of::<STARTUPINFOW>() as u32,
            ..Default::default()
        };
        let mut process = PROCESS_INFORMATION::default();
        // SAFETY: All buffers are terminated and live, and no handles (including capture pipes) are inherited.
        if unsafe {
            CreateProcessW(
                executable.as_ptr(),
                command.as_mut_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP | CREATE_UNICODE_ENVIRONMENT,
                environment.as_ptr().cast(),
                directory.as_ptr(),
                &startup,
                &mut process,
            )
        } == 0
        {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: Successful process creation transfers these two distinct handles to us.
        let process_handle = unsafe { OwnedHandle::from_raw_handle(process.hProcess) };
        let thread_handle = unsafe { OwnedHandle::from_raw_handle(process.hThread) };
        drop(thread_handle);
        Ok(Child {
            process: process_handle,
            id: process.dwProcessId,
        })
    }

    fn wide(value: &OsStr) -> io::Result<Vec<u16>> {
        let mut out: Vec<_> = value.encode_wide().collect();
        if out.contains(&0) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "NUL in process path"));
        }
        out.push(0);
        Ok(out)
    }

    fn working_directory(path: &Path) -> io::Result<Vec<u16>> {
        let canonical = path.canonicalize()?;
        let mut directory: Vec<_> = canonical.as_os_str().encode_wide().collect();
        let verbatim: Vec<_> = "\\\\?\\".encode_utf16().collect();
        let verbatim_unc: Vec<_> = "\\\\?\\UNC\\".encode_utf16().collect();
        if directory.starts_with(&verbatim_unc) {
            directory.splice(..verbatim_unc.len(), "\\\\".encode_utf16());
        } else if directory.starts_with(&verbatim) {
            directory.drain(..verbatim.len());
        }
        let directory = PathBuf::from(OsString::from_wide(&directory));
        if !directory.is_absolute() || directory.canonicalize()? != canonical {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "working directory has no equivalent non-verbatim path",
            ));
        }
        wide(directory.as_os_str())
    }

    #[expect(unsafe_code)]
    impl Child {
        pub(crate) fn id(&self) -> u32 {
            self.id
        }
        pub(crate) fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            // SAFETY: The process handle is owned and valid during this nonblocking wait.
            match unsafe { WaitForSingleObject(self.process.as_raw_handle(), 0) } {
                WAIT_TIMEOUT => Ok(None),
                WAIT_OBJECT_0 => self.status().map(Some),
                _ => Err(io::Error::last_os_error()),
            }
        }
        pub(crate) fn kill(&mut self) -> io::Result<()> {
            // SAFETY: This handle refers only to our own detached child.
            if unsafe { TerminateProcess(self.process.as_raw_handle(), 1) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        }
        pub(crate) fn wait(&mut self) -> io::Result<ExitStatus> {
            // SAFETY: The process handle remains valid until waiting completes.
            if unsafe { WaitForSingleObject(self.process.as_raw_handle(), INFINITE) } != WAIT_OBJECT_0 {
                return Err(io::Error::last_os_error());
            }
            self.status()
        }
        fn status(&self) -> io::Result<ExitStatus> {
            let mut code = 0;
            // SAFETY: The process handle is valid and code is writable stack storage.
            if unsafe { GetExitCodeProcess(self.process.as_raw_handle(), &mut code) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(ExitStatus::from_raw(code))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreground_argument_prefix_is_preserved_before_lifecycle_options() {
        let prefix: Vec<OsString> = ["--repository", "work tree", "fsmonitor", "run"]
            .into_iter()
            .map(OsString::from)
            .collect();
        let ready_file = Path::new("ready file");
        let arguments = run_arguments(&prefix, 42, ready_file);
        assert_eq!(
            &arguments[..prefix.len()],
            prefix.as_slice(),
            "the adapter's complete foreground argument prefix remains unchanged"
        );
        assert_eq!(
            &arguments[prefix.len()..],
            [
                OsString::from("--idle-timeout"),
                OsString::from("42"),
                OsString::from("--ready-file"),
                ready_file.as_os_str().to_owned(),
            ],
            "lifecycle options are appended without splitting paths containing spaces"
        );
    }

    #[test]
    fn windows_arguments_preserve_spaces_quotes_and_trailing_backslashes() -> gix_testtools::TestResult {
        for (argument, expected) in [
            ("", "\"\""),
            ("two words", "\"two words\""),
            ("a\"b", "\"a\\\"b\""),
            ("trailing\\", "\"trailing\\\\\""),
        ] {
            let mut encoded = Vec::new();
            quote(&argument.encode_utf16().collect::<Vec<_>>(), &mut encoded);
            assert_eq!(
                String::from_utf16(&encoded)?,
                expected,
                "every argument remains one exact native argument"
            );
        }
        Ok(())
    }

    #[test]
    fn inherited_repository_redirection_is_distinct_from_configuration() {
        assert!(
            redirects_repository(OsStr::new("GIT_WORK_TREE")),
            "parent worktree overrides must be replaced by the prepared context"
        );
        assert!(
            redirects_repository(OsStr::new("git_common_dir")),
            "Windows environment names are case insensitive"
        );
        assert!(
            !redirects_repository(OsStr::new("GIT_CONFIG_GLOBAL")),
            "the child must retain the caller's configuration environment"
        );
    }
}
