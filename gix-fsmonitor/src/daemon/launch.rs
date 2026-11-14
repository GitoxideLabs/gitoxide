use std::{
    ffi::{OsStr, OsString},
    path::Path,
};

use gix_error::{OptionExt, Result, ResultExt, message};

use super::Options;
use crate::server;

#[cfg(not(windows))]
pub(crate) type Child = std::process::Child;

pub(crate) fn spawn(
    context: &server::Context,
    reopen_git_dir: &Path,
    arguments: &[OsString],
    idle_timeout: u64,
    ready_file: &Path,
    options: Options,
) -> Result<Child> {
    let arguments = run_arguments(arguments, idle_timeout, ready_file);
    let environment = environment(std::env::vars_os().collect(), context, reopen_git_dir, options)?;
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

fn same_environment_name(name: &OsStr, expected: &OsStr) -> bool {
    #[cfg(windows)]
    {
        name.to_str()
            .zip(expected.to_str())
            .is_some_and(|(name, expected)| name.eq_ignore_ascii_case(expected))
    }
    #[cfg(not(windows))]
    {
        name == expected
    }
}

fn redirects_repository(name: &OsStr) -> bool {
    [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_CEILING_DIRECTORIES",
        "GIT_DISCOVERY_ACROSS_FILESYSTEM",
        "GIT_NAMESPACE",
        "GIT_PREFIX",
        "GIT_SHALLOW_FILE",
    ]
    .into_iter()
    .any(|expected| same_environment_name(name, OsStr::new(expected)))
}

fn environment(
    mut environment: Vec<(OsString, OsString)>,
    context: &server::Context,
    reopen_git_dir: &Path,
    options: Options,
) -> Result<Vec<(OsString, OsString)>> {
    let count = environment
        .iter()
        .rev()
        .find(|(name, _)| same_environment_name(name, OsStr::new("GIT_CONFIG_COUNT")))
        .map(|(_, value)| {
            value
                .to_str()
                .ok_or_raise(|| message("inherited GIT_CONFIG_COUNT is not valid UTF-8"))?
                .parse::<usize>()
                .or_raise(|| message("parsing inherited GIT_CONFIG_COUNT for filesystem monitor startup"))
        })
        .transpose()?
        .unwrap_or(0);
    let new_count = count
        .checked_add(2)
        .ok_or_raise(|| message("appending filesystem monitor configuration would overflow GIT_CONFIG_COUNT"))?;
    let overrides: [(OsString, OsString); 5] = [
        (format!("GIT_CONFIG_KEY_{count}").into(), "fsmonitor.allowRemote".into()),
        (
            format!("GIT_CONFIG_VALUE_{count}").into(),
            options.allow_remote.to_string().into(),
        ),
        (
            format!("GIT_CONFIG_KEY_{}", count + 1).into(),
            "fsmonitor.startTimeout".into(),
        ),
        (
            format!("GIT_CONFIG_VALUE_{}", count + 1).into(),
            options.start_timeout.as_secs().to_string().into(),
        ),
        ("GIT_CONFIG_COUNT".into(), new_count.to_string().into()),
    ];
    // Later counted entries win; remove all copies of the count and any orphan slots we reuse.
    environment.retain(|(name, _)| {
        !redirects_repository(name)
            && !overrides
                .iter()
                .any(|(expected, _)| same_environment_name(name, expected))
    });
    environment.extend([
        ("GIT_DIR".into(), reopen_git_dir.as_os_str().to_owned()),
        ("GIT_WORK_TREE".into(), context.worktree.as_os_str().to_owned()),
        ("GIT_COMMON_DIR".into(), context.common_dir.as_os_str().to_owned()),
    ]);
    environment.extend(overrides);
    Ok(environment)
}

#[cfg(unix)]
#[expect(unsafe_code)]
fn native_spawn(arguments: &[OsString], environment: &[(OsString, OsString)]) -> std::io::Result<Child> {
    use std::{
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
        command.pre_exec(|| rustix::process::setsid().map(|_| ()).map_err(Into::into));
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
    use std::time::Duration;

    fn entries(values: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        values
            .iter()
            .map(|(name, value)| ((*name).into(), (*value).into()))
            .collect()
    }

    fn prepare(inherited: Vec<(OsString, OsString)>, options: Options) -> Result<Vec<(OsString, OsString)>> {
        let worktree = Path::new(if cfg!(windows) { r"C:\worktree" } else { "/worktree" });
        let git_dir = worktree.join(".git");
        let context = server::Context {
            worktree: worktree.to_owned(),
            common_dir: git_dir.clone(),
            endpoint: git_dir.join("fsmonitor--daemon.ipc"),
            git_dir,
            precompose_unicode: false,
            ignore_case: false,
        };
        environment(inherited, &context, &context.git_dir, options)
    }

    fn value<'a>(environment: &'a [(OsString, OsString)], name: &str) -> &'a OsStr {
        environment
            .iter()
            .rev()
            .find(|(candidate, _)| same_environment_name(candidate, OsStr::new(name)))
            .expect("the prepared environment contains the expected variable")
            .1
            .as_os_str()
    }

    fn config_value<'a>(
        environment: &'a [(OsString, OsString)],
        key: &str,
    ) -> gix_testtools::TestResult<Option<&'a OsStr>> {
        let count: usize = value(environment, "GIT_CONFIG_COUNT")
            .to_str()
            .expect("the prepared count is ASCII")
            .parse()?;
        Ok((0..count)
            .rev()
            .find(|index| value(environment, &format!("GIT_CONFIG_KEY_{index}")) == OsStr::new(key))
            .map(|index| value(environment, &format!("GIT_CONFIG_VALUE_{index}"))))
    }

    #[test]
    fn counted_config_preserves_inherited_options_and_appends_resolved_overrides() -> gix_testtools::TestResult {
        let inherited = entries(&[
            ("GIT_CONFIG_COUNT", "3"),
            ("GIT_CONFIG_KEY_0", "user.name"),
            ("GIT_CONFIG_VALUE_0", "Name with spaces"),
            ("GIT_CONFIG_KEY_1", "fsmonitor.allowRemote"),
            ("GIT_CONFIG_VALUE_1", "false"),
            ("GIT_CONFIG_KEY_2", "fsmonitor.startTimeout"),
            ("GIT_CONFIG_VALUE_2", "3"),
            ("GIT_CONFIG_GLOBAL", "global configuration"),
            ("GIT_CONFIG_PARAMETERS", "unchanged unsupported configuration"),
            ("UNRELATED", "unchanged"),
            ("GIT_DIR", "foreign repository"),
        ]);
        let environment = prepare(
            inherited,
            Options {
                allow_remote: true,
                start_timeout: Duration::from_secs(12),
            },
        )?;
        assert_eq!(
            value(&environment, "GIT_CONFIG_COUNT"),
            "5",
            "two resolved settings are appended"
        );
        assert_eq!(
            value(&environment, "GIT_CONFIG_VALUE_1"),
            "false",
            "inherited entries are not rewritten"
        );
        assert_eq!(
            value(&environment, "GIT_CONFIG_VALUE_2"),
            "3",
            "inherited timeout entries remain intact"
        );
        assert_eq!(
            config_value(&environment, "user.name")?,
            Some(OsStr::new("Name with spaces")),
            "unrelated counted config is preserved"
        );
        assert_eq!(
            config_value(&environment, "fsmonitor.allowRemote")?,
            Some(OsStr::new("true")),
            "later slots override inherited permission"
        );
        assert_eq!(
            config_value(&environment, "fsmonitor.startTimeout")?,
            Some(OsStr::new("12")),
            "later slots override inherited timeout"
        );
        assert_eq!(
            value(&environment, "GIT_CONFIG_GLOBAL"),
            "global configuration",
            "configuration paths are preserved"
        );
        assert_eq!(
            value(&environment, "GIT_CONFIG_PARAMETERS"),
            "unchanged unsupported configuration",
            "counted overrides do not rewrite unrelated environment configuration"
        );
        assert_eq!(
            value(&environment, "UNRELATED"),
            "unchanged",
            "unrelated environment survives"
        );
        assert_ne!(
            value(&environment, "GIT_DIR"),
            "foreign repository",
            "repository redirection is replaced"
        );
        Ok(())
    }

    #[test]
    fn counted_config_matches_resolved_boolean_and_seconds_without_inherited_options() -> gix_testtools::TestResult {
        for allow_remote in [false, true] {
            for seconds in [0, 12, 60] {
                for inherited in [Vec::new(), entries(&[("GIT_CONFIG_COUNT", "0")])] {
                    let environment = prepare(
                        inherited,
                        Options {
                            allow_remote,
                            start_timeout: Duration::from_secs(seconds),
                        },
                    )?;
                    assert_eq!(
                        value(&environment, "GIT_CONFIG_COUNT"),
                        "2",
                        "an absent or zero count starts at slot zero"
                    );
                    assert_eq!(
                        value(&environment, "GIT_CONFIG_VALUE_0"),
                        OsStr::new(&allow_remote.to_string()),
                        "permission is a Git boolean"
                    );
                    assert_eq!(
                        value(&environment, "GIT_CONFIG_VALUE_1"),
                        OsStr::new(&seconds.to_string()),
                        "timeout is serialized as seconds, including zero"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn repeated_injection_keeps_earlier_slots_and_uses_the_latest_resolved_options() -> gix_testtools::TestResult {
        let first = prepare(
            entries(&[
                ("GIT_CONFIG_COUNT", "1"),
                ("GIT_CONFIG_KEY_0", "user.name"),
                ("GIT_CONFIG_VALUE_0", "Name"),
            ]),
            Options {
                allow_remote: true,
                start_timeout: Duration::from_secs(12),
            },
        )?;
        let second = prepare(
            first,
            Options {
                allow_remote: false,
                start_timeout: Duration::ZERO,
            },
        )?;
        assert_eq!(
            value(&second, "GIT_CONFIG_COUNT"),
            "5",
            "each injection appends exactly two slots"
        );
        assert_eq!(
            value(&second, "GIT_CONFIG_VALUE_1"),
            "true",
            "earlier injected values remain intact"
        );
        assert_eq!(
            config_value(&second, "user.name")?,
            Some(OsStr::new("Name")),
            "original options survive repeated injection"
        );
        assert_eq!(
            config_value(&second, "fsmonitor.allowRemote")?,
            Some(OsStr::new("false")),
            "the latest permission wins"
        );
        assert_eq!(
            config_value(&second, "fsmonitor.startTimeout")?,
            Some(OsStr::new("0")),
            "the latest timeout wins"
        );
        assert_eq!(
            second
                .iter()
                .filter(|(name, _)| same_environment_name(name, OsStr::new("GIT_CONFIG_COUNT")))
                .count(),
            1,
            "only one active count remains"
        );
        Ok(())
    }

    #[test]
    fn overwritten_counts_and_orphan_slots_have_no_duplicate_names() -> gix_testtools::TestResult {
        let environment = prepare(
            entries(&[
                ("GIT_CONFIG_COUNT", "0"),
                ("GIT_CONFIG_COUNT", "1"),
                ("GIT_CONFIG_KEY_0", "user.name"),
                ("GIT_CONFIG_VALUE_0", "Name"),
                ("GIT_CONFIG_KEY_1", "orphan.one"),
                ("GIT_CONFIG_KEY_1", "orphan.two"),
                ("GIT_CONFIG_VALUE_1", "orphan one"),
                ("GIT_CONFIG_VALUE_1", "orphan two"),
                ("GIT_CONFIG_KEY_2", "orphan.three"),
                ("GIT_CONFIG_VALUE_2", "orphan three"),
            ]),
            Options::default(),
        )?;
        assert_eq!(
            value(&environment, "GIT_CONFIG_COUNT"),
            "3",
            "the last inherited count determines where to append"
        );
        for name in [
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_1",
            "GIT_CONFIG_VALUE_1",
            "GIT_CONFIG_KEY_2",
            "GIT_CONFIG_VALUE_2",
        ] {
            assert_eq!(
                environment
                    .iter()
                    .filter(|(candidate, _)| same_environment_name(candidate, OsStr::new(name)))
                    .count(),
                1,
                "every overwritten variable has exactly one replacement"
            );
        }
        assert_eq!(
            config_value(&environment, "user.name")?,
            Some(OsStr::new("Name")),
            "valid inherited slots are not removed"
        );
        Ok(())
    }

    #[test]
    fn counted_environment_names_are_case_insensitive_only_on_windows() -> gix_testtools::TestResult {
        let environment = prepare(
            entries(&[
                ("git_config_count", "0"),
                ("GIT_CONFIG_COUNT", "1"),
                ("GIT_CONFIG_KEY_0", "user.name"),
                ("GIT_CONFIG_VALUE_0", "Name"),
                ("git_config_key_1", "orphan.one"),
                ("git_config_value_1", "orphan one"),
                ("git_config_key_2", "orphan.two"),
                ("git_config_value_2", "orphan two"),
            ]),
            Options::default(),
        )?;
        for name in [
            "git_config_count",
            "git_config_key_1",
            "git_config_value_1",
            "git_config_key_2",
            "git_config_value_2",
        ] {
            assert_eq!(
                environment.iter().any(|(candidate, _)| candidate == OsStr::new(name)),
                !cfg!(windows),
                "differently cased variables are unrelated on Unix but replaced on Windows"
            );
        }
        assert_eq!(
            value(&environment, "GIT_CONFIG_COUNT"),
            "3",
            "the active count uses native name matching"
        );
        Ok(())
    }

    #[test]
    fn malformed_and_overflowing_inherited_counts_are_contextual_errors() -> gix_testtools::TestResult {
        for count in [
            String::new(),
            "invalid".into(),
            "-1".into(),
            "1k".into(),
            " 1".into(),
            usize::MAX.to_string(),
            (usize::MAX - 1).to_string(),
            format!("{}0", usize::MAX),
        ] {
            let error = prepare(entries(&[("GIT_CONFIG_COUNT", &count)]), Options::default())
                .expect_err("invalid or overflowing counted configuration cannot be extended");
            assert!(
                error.to_string().contains("GIT_CONFIG_COUNT"),
                "errors identify the inherited count: {error}"
            );
        }
        Ok(())
    }

    #[cfg(unix)]
    fn non_utf8() -> OsString {
        use std::os::unix::ffi::OsStringExt;
        OsString::from_vec(vec![b'n', 0xff])
    }

    #[cfg(windows)]
    fn non_utf8() -> OsString {
        use std::os::windows::ffi::OsStringExt;
        OsString::from_wide(&[b'n' as u16, 0xd800])
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn native_environment_names_and_values_are_preserved_but_count_must_parse() -> gix_testtools::TestResult {
        let native = non_utf8();
        let mut inherited = entries(&[("GIT_CONFIG_COUNT", "1"), ("GIT_CONFIG_KEY_0", "user.name")]);
        inherited.push(("GIT_CONFIG_VALUE_0".into(), native.clone()));
        inherited.push((native.clone(), native.clone()));
        let environment = prepare(inherited, Options::default())?;
        assert_eq!(
            value(&environment, "GIT_CONFIG_VALUE_0"),
            native.as_os_str(),
            "native counted values are not converted lossily"
        );
        assert!(
            environment.contains(&(native.clone(), native.clone())),
            "unrelated native environment names and values remain intact"
        );
        let error = prepare(vec![("GIT_CONFIG_COUNT".into(), native)], Options::default())
            .expect_err("the inherited count must be valid UTF-8 decimal text");
        assert!(
            error.to_string().contains("GIT_CONFIG_COUNT"),
            "non-UTF-8 count errors identify the variable: {error}"
        );
        Ok(())
    }

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
        assert_eq!(
            redirects_repository(OsStr::new("git_common_dir")),
            cfg!(windows),
            "environment names are case insensitive only on Windows"
        );
        assert!(
            !redirects_repository(OsStr::new("GIT_CONFIG_GLOBAL")),
            "the child must retain the caller's configuration environment"
        );
    }
}
