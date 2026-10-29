use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::ByteSlice;
use gix_error::{Result, ResultExt, message};

use crate::{Context, Prepare, anchor_path, extract_interpreter, is_bare_command, split_paths, win_path_lookup};

/// Builder
impl Prepare {
    /// If called, the command will be checked for characters that are typical for shell
    /// scripts, and if found will use `sh` to execute it or whatever is set as
    /// [`with_shell_program()`](Self::with_shell_program()).
    ///
    /// Commands are inspected using their native encoded bytes, including commands that cannot
    /// be represented as UTF-8.
    ///
    /// If a shell is used, then arguments given here with [arg()](Self::arg) or
    /// [args()](Self::args) will be appended to the command via `"$@"`, even if the command already
    /// contains `"$@"`. The original command is passed as the shell's `$0`, matching Git.
    ///
    ///
    /// The [`command_may_be_shell_script_allow_manual_argument_splitting()`](Self::command_may_be_shell_script_allow_manual_argument_splitting())
    /// and [`command_may_be_shell_script_disallow_manual_argument_splitting()`](Self::command_may_be_shell_script_disallow_manual_argument_splitting())
    /// methods also call this method.
    ///
    /// If neither this method nor [`with_shell()`](Self::with_shell()) is called, commands are
    /// always executed verbatim and directly, without the use of a shell.
    pub fn command_may_be_shell_script(mut self) -> Self {
        self.use_shell = self
            .command
            .as_encoded_bytes()
            .find_byteset(b"|&;<>()$`\\\"' \t\n*?[#~=%")
            .is_some();
        self
    }

    /// If called, unconditionally use a shell to execute the command and its arguments.
    ///
    /// This uses `sh` to execute it, or whatever is set as
    /// [`with_shell_program()`](Self::with_shell_program()).
    ///
    /// Arguments given here with [arg()](Self::arg) or [args()](Self::args) will be
    /// appended via `"$@"`, even if the command already contains `"$@"`. The original command is
    /// passed as the shell's `$0`, matching Git.
    ///
    /// If neither this method nor
    /// [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) is called,
    /// commands are always executed verbatim and directly, without the use of a shell. (But
    /// see [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) on other
    /// methods that call that method.)
    ///
    /// We also disallow manual argument splitting
    /// (see [`command_may_be_shell_script_disallow_manual_argument_splitting`](Self::command_may_be_shell_script_disallow_manual_argument_splitting()))
    /// to assure a shell is indeed used, no matter what.
    pub fn with_shell(mut self) -> Self {
        self.use_shell = true;
        self.allow_manual_arg_splitting = false;
        self
    }

    /// Quote the command if it is run in a shell, so its path is left intact.
    ///
    /// This is only meaningful if the command has been arranged to run in a shell, either
    /// unconditionally with [`with_shell()`](Self::with_shell()), or conditionally with
    /// [`command_may_be_shell_script()`](Self::command_may_be_shell_script()).
    ///
    /// Note that this should not be used if the command is a script - quoting is only the
    /// right choice if it's known to be a program path.
    ///
    /// Note also that this does not affect arguments passed with [arg()](Self::arg) or
    /// [args()](Self::args), which do not have to be quoted by the *caller* because they are
    /// passed as `"$@"` positional parameters (`"$1"`, `"$2"`, and so on).
    pub fn with_quoted_command(mut self) -> Self {
        self.quote_command = true;
        self
    }

    /// Set the name or path to the shell `program` to use if a shell is to be used, to avoid
    /// using the default shell which is `sh`.
    ///
    /// Note that shells that are not Bourne-style cannot be expected to work correctly,
    /// because POSIX shell syntax is assumed when appending `"$@"` to receive arguments
    /// and in the behaviour of
    /// [`with_quoted_command()`](Self::with_quoted_command()), if called.
    pub fn with_shell_program(mut self, program: impl Into<OsString>) -> Self {
        self.shell_program = Some(program.into());
        self
    }

    /// Unconditionally turn off using the shell when spawning the command.
    ///
    /// Note that not using the shell is the default. So an effective use of this method
    /// is some time after [`command_may_be_shell_script()`](Self::command_may_be_shell_script())
    /// or [`with_shell()`](Self::with_shell()) was called.
    pub fn without_shell(mut self) -> Self {
        self.use_shell = false;
        self
    }

    /// Set the working directory for the child process.
    ///
    /// Relative directories are anchored to the current process directory when converting to
    /// [`Command`] or calling [`spawn()`][Self::spawn()]. On Windows the same directory is used
    /// to resolve relative commands, non-empty PATH entries, and shebang interpreters before spawning.
    /// Set this before conversion instead of changing the returned command's directory afterwards.
    pub fn current_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(path.into());
        self
    }

    /// Set additional `ctx` to be used when spawning the process.
    ///
    /// Note that this is a must for most kind of commands that `git` usually spawns, as at
    /// least they need to know the correct Git repository to function.
    pub fn with_context(mut self, ctx: Context) -> Self {
        self.context = Some(ctx);
        self
    }

    /// Like [`command_may_be_shell_script()`](Self::command_may_be_shell_script()), but try to
    /// split arguments by hand if this can be safely done without a shell.
    ///
    /// This is useful on platforms where spawning processes is slow, or where many processes
    /// have to be spawned in a row which should be sped up. Manual argument splitting is
    /// enabled by default on Windows only.
    ///
    /// Note that this does *not* check for the use of possible shell builtins. Commands may
    /// fail or behave differently if they are available as shell builtins and no corresponding
    /// external command exists, or the external command behaves differently.
    /// Leading shell assignment words are applied to the environment when followed by a command.
    pub fn command_may_be_shell_script_allow_manual_argument_splitting(mut self) -> Self {
        self.allow_manual_arg_splitting = true;
        self.command_may_be_shell_script()
    }

    /// Like [`command_may_be_shell_script()`](Self::command_may_be_shell_script()), but don't
    /// allow to bypass the shell even if manual argument splitting can be performed safely.
    pub fn command_may_be_shell_script_disallow_manual_argument_splitting(mut self) -> Self {
        self.allow_manual_arg_splitting = false;
        self.command_may_be_shell_script()
    }

    /// Configure the process to use `stdio` for _stdin_.
    pub fn stdin(mut self, stdio: Stdio) -> Self {
        self.stdin = stdio;
        self
    }
    /// Configure the process to use `stdio` for _stdout_.
    pub fn stdout(mut self, stdio: Stdio) -> Self {
        self.stdout = stdio;
        self
    }
    /// Configure the process to use `stdio` for _stderr_.
    pub fn stderr(mut self, stdio: Stdio) -> Self {
        self.stderr = stdio;
        self
    }

    /// Add `arg` to the list of arguments to call the command with.
    pub fn arg(mut self, arg: impl Into<OsString>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add `args` to the list of arguments to call the command with.
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<OsString>>) -> Self {
        self.args
            .append(&mut args.into_iter().map(Into::into).collect::<Vec<_>>());
        self
    }

    /// Add `key` with `value` to the environment of the spawned command.
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

/// Finalization
impl Prepare {
    /// Spawn the command as configured.
    ///
    /// Encoding errors during preparation are returned with [`std::io::ErrorKind::InvalidInput`].
    pub fn spawn(self) -> std::io::Result<std::process::Child> {
        let mut cmd =
            Command::try_from(self).map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
        gix_trace::debug!(cmd = ?cmd);
        cmd.spawn()
    }
}

impl TryFrom<Prepare> for Command {
    type Error = gix_error::Error;

    fn try_from(mut prep: Prepare) -> Result<Self> {
        let current_dir = prep
            .current_dir
            .map(std::path::absolute)
            .transpose()
            .or_raise(|| message("Could not resolve the command's working directory"))?;
        let mut inline_env = Vec::new();
        let mut cmd = if prep.use_shell {
            let split_args = prep
                .allow_manual_arg_splitting
                .then(|| {
                    let command = gix_path::os_str_into_bstr(&prep.command).ok()?;
                    if command.find_byteset(b"\\|&;<>()$`\n*?[#~%").is_none() {
                        crate::parse::command_line(command).ok()
                    } else {
                        None
                    }
                })
                .flatten();
            match split_args {
                Some(parsed) => {
                    let mut cmd = if cfg!(windows) {
                        windows_command(parsed.command, &prep.env, &parsed.env, current_dir.as_deref())?
                    } else {
                        Command::new(parsed.command)
                    };
                    cmd.args(parsed.args);
                    inline_env = parsed.env;
                    cmd
                }
                None => {
                    let mut cmd = match prep.shell_program {
                        Some(shell) => Command::new(shell),
                        None => gix_path::env::shell_command(),
                    };
                    if cfg!(windows) {
                        cmd = windows_shell_command(cmd, &prep.env, current_dir.as_deref())?;
                    }
                    let arg0 = prep.command.clone();
                    if prep.quote_command {
                        let command = gix_path::os_str_into_bstr(&prep.command)?;
                        prep.command = gix_path::from_bstring(gix_quote::single(command))?.into();
                    }
                    if !prep.args.is_empty() {
                        prep.command.push(r#" "$@""#);
                    }
                    cmd.arg("-c");
                    cmd.arg(prep.command);
                    cmd.arg(arg0);
                    cmd
                }
            }
        } else if cfg!(windows) {
            windows_command(prep.command, &prep.env, &[], current_dir.as_deref())?
        } else {
            Command::new(prep.command)
        };
        if let Some(current_dir) = current_dir {
            cmd.current_dir(current_dir);
        }
        // We never want to have terminals pop-up on Windows if this runs from a GUI application.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }
        cmd.stdin(prep.stdin)
            .stdout(prep.stdout)
            .stderr(prep.stderr)
            .envs(prep.env)
            .args(prep.args);
        if let Some(ctx) = prep.context {
            if let Some(git_dir) = ctx.git_dir {
                cmd.env("GIT_DIR", &git_dir);
            }
            if let Some(worktree_dir) = ctx.worktree_dir {
                cmd.env("GIT_WORK_TREE", worktree_dir);
            }
            if let Some(value) = ctx.no_replace_objects {
                cmd.env("GIT_NO_REPLACE_OBJECTS", usize::from(value).to_string());
            }
            if let Some(namespace) = ctx.ref_namespace {
                cmd.env("GIT_NAMESPACE", gix_path::from_bstring(namespace)?);
            }
            if let Some(value) = ctx.literal_pathspecs {
                cmd.env("GIT_LITERAL_PATHSPECS", usize::from(value).to_string());
            }
            if let Some(value) = ctx.glob_pathspecs {
                cmd.env(
                    if value {
                        "GIT_GLOB_PATHSPECS"
                    } else {
                        "GIT_NOGLOB_PATHSPECS"
                    },
                    "1",
                );
            }
            if let Some(value) = ctx.icase_pathspecs {
                cmd.env("GIT_ICASE_PATHSPECS", usize::from(value).to_string());
            }
            if let Some(stderr) = ctx.stderr {
                cmd.stderr(if stderr { Stdio::inherit() } else { Stdio::null() });
            }
        }
        cmd.envs(inline_env);
        Ok(cmd)
    }
}

/// Resolve the shell too, retaining arguments required by a Git for Windows shell shim.
fn windows_shell_command(shell: Command, env: &[(OsString, OsString)], current_dir: Option<&Path>) -> Result<Command> {
    let mut command = windows_command(shell.get_program().to_owned(), env, &[], current_dir)?;
    command.args(shell.get_args());
    Ok(command)
}

/// Create a Windows command using Git-compatible `PATH` lookup and shebang dispatch.
///
/// The last `PATH` in `inline_env` overrides the last one in `env`, which overrides the inherited value (of this process).
/// The selected `PATH` is searched in order for both commands and shebang interpreter basenames. Interpreters must
/// have an `.exe` suffix; their shebang directory and arguments are ignored, like Git for Windows. If `PATH` does not
/// resolve a bare command, the missing program remains anchored in its first entry so Rust's broader Windows lookup
/// cannot find it elsewhere. If present, `current_dir` is absolute and anchors every relative path.
fn windows_command(
    command: OsString,
    env: &[(OsString, OsString)],
    inline_env: &[(String, OsString)],
    current_dir: Option<&Path>,
) -> Result<Command> {
    let explicit_joined_paths = inline_env
        .iter()
        .rev()
        .find(|(name, _)| name.eq_ignore_ascii_case("PATH"))
        .map(|(_, value)| value.as_os_str())
        .or_else(|| {
            env.iter()
                .rev()
                .find(|(name, _)| name.to_str().is_some_and(|name| name.eq_ignore_ascii_case("PATH")))
                .map(|(_, value)| value.as_os_str())
        });
    let joined_paths = explicit_joined_paths
        .map(Cow::Borrowed)
        .or_else(|| std::env::var_os("PATH").map(Cow::Owned));
    let looked_up = joined_paths
        .as_deref()
        .map(|joined_paths| win_path_lookup(command.as_ref(), joined_paths, current_dir))
        .transpose()?
        .flatten();
    let program: Cow<'_, Path> = match looked_up {
        // Use the manually resolved path.
        Some(program) => Cow::Owned(program),
        // A bare PATH miss must neither probe a worktree file nor use Rust's broader Windows search.
        None if is_bare_command(Path::new(&command)) => {
            return Ok(Command::new(prevent_further_path_lookup(
                command.as_ref(),
                joined_paths.as_deref().unwrap_or_else(|| OsStr::new("")),
                current_dir,
            )?));
        }
        None if command.is_empty() => Cow::Borrowed(command.as_ref()),
        None => Cow::Owned(anchor_path(command.as_ref(), current_dir)?),
    };
    if let Some(mut interpreter) = extract_interpreter(program.as_ref()).and_then(|shebang| {
        let path = gix_path::into_bstr(shebang.interpreter).ok()?;
        let basename = path.rsplit(|byte| matches!(byte, b'/' | b'\\')).next()?;
        gix_path::from_byte_slice(basename)
            .ok()
            .map(|path| path.as_os_str().to_owned())
    }) {
        // Git for Windows searches PATH for the interpreter's basename, accepting only .exe files.
        if !Path::new(&interpreter)
            .extension()
            .is_some_and(|extension| extension.as_encoded_bytes().eq_ignore_ascii_case(b"exe"))
        {
            interpreter.push(".exe");
        }
        let interpreter = match joined_paths
            .as_deref()
            .map(|paths| win_path_lookup(Path::new(&interpreter), paths, current_dir))
            .transpose()?
            .flatten()
        {
            Some(path) => path,
            None => prevent_further_path_lookup(
                Path::new(&interpreter),
                joined_paths.as_deref().unwrap_or_else(|| OsStr::new("")),
                current_dir,
            )?,
        };
        let mut cmd = Command::new(interpreter);
        // Git for Windows ignores shebang arguments and passes only the script path.
        cmd.arg(program.as_ref());
        Ok(cmd)
    } else {
        Ok(Command::new(program.as_ref()))
    }
}

/// Represent the failed lookup of `command` in `PATH` without permitting another search.
///
/// `joined_paths` is the complete value of `PATH`, not one of its entries. The first non-empty entry is
/// joined with `command`, producing a path that Rust's Windows resolver will not look up elsewhere. If there is no such
/// entry, a trailing separator makes `command` invalid instead.
fn prevent_further_path_lookup(command: &Path, joined_paths: &OsStr, current_dir: Option<&Path>) -> Result<PathBuf> {
    let missing = if let Some(mut root) = split_paths(joined_paths).next() {
        root.push(command);
        root
    } else {
        let mut missing = command.to_owned();
        // A trailing separator makes the program invalid without allowing Rust to search ambient locations.
        missing.push("");
        missing
    };
    anchor_path(&missing, current_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inherited_path_is_used_for_both_probing_and_execution() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let root = gix_testtools::tempfile::tempdir()?;
        let bin = root.path().join("bin");
        std::fs::create_dir(&bin)?;
        let _cwd = gix_testtools::set_current_dir(root.path())?;
        std::fs::write("ssh", b"#!/untrusted/interpreter\n")?;

        for path in [Some(bin.to_str().expect("temporary path is UTF-8")), Some(""), None] {
            let environment = gix_testtools::Env::new();
            let _environment = match path {
                Some(path) => environment.set("PATH", path),
                None => environment.unset("PATH"),
            };
            let mut cmd = windows_command("ssh".into(), &[], &[], None)?;
            let expected = match path {
                Some(path) if !path.is_empty() => bin.join("ssh"),
                _ => Path::new("ssh").join(""),
            };
            assert_eq!(
                cmd.get_program(),
                expected,
                "a PATH miss cannot probe a worktree file or leave a bare command for another lookup"
            );
            assert!(cmd.get_args().next().is_none(), "the planted shebang is never used");
            assert!(cmd.spawn().is_err(), "a missing program fails to spawn");
        }

        let _environment = gix_testtools::Env::new().set("PATH", bin.to_str().expect("temporary path is UTF-8"));
        let executable = bin.join("ssh.exe");
        std::fs::write(&executable, b"executable placeholder")?;
        assert_eq!(
            windows_command("ssh".into(), &[], &[], None)?.get_program(),
            executable,
            "the executable selected from PATH is also the one passed to the process launcher"
        );
        let explicit_script = windows_command("./ssh".into(), &[], &[], None)?;
        assert_eq!(
            explicit_script.get_program(),
            bin.join("interpreter.exe"),
            "an explicit script's missing interpreter stays confined to PATH"
        );
        assert_eq!(
            explicit_script.get_args().collect::<Vec<_>>(),
            [OsStr::new("./ssh")],
            "the interpreter receives the explicitly requested script"
        );
        Ok(())
    }

    #[test]
    fn windows_commands_resolve_from_the_child_directory() -> gix_testtools::TestResult {
        let root = gix_testtools::scripted_fixture_read_only("windows_shebang.sh")?.canonicalize()?;
        let paths = std::env::join_paths(["", "first", "second"])?;
        let environment = [(OsString::from("PATH"), paths)];
        for program in [
            OsString::from("hook"),
            OsString::from("./hook"),
            root.join("hook").into_os_string(),
        ] {
            let command = windows_command(program, &environment, &[], Some(&root))?;
            assert_eq!(
                command.get_program(),
                root.join("second/sh.exe"),
                "relative commands, PATH entries, and interpreter lookup use the child directory"
            );
        }
        let command = windows_command(
            "./hook".into(),
            &environment,
            &[("PATH".into(), "third".into())],
            Some(&root),
        )?;
        assert_eq!(
            command.get_program(),
            root.join("third/sh.exe"),
            "inline relative PATH also starts at the child directory"
        );
        let command = windows_command("hook".into(), &[("PATH".into(), "".into())], &[], Some(&root))?;
        assert_eq!(
            command.get_program(),
            root.join("hook").join(""),
            "an empty PATH does not probe a hook in the child directory"
        );
        assert!(
            command.get_args().next().is_none(),
            "empty PATH does not trigger shebang discovery in cwd"
        );
        for environment in [&environment[..], &[(OsString::from("PATH"), OsString::new())]] {
            let mut shell = Command::new("sh.exe");
            shell.arg("--posix");
            let shell = windows_shell_command(shell, environment, Some(&root))?;
            let expected = if environment[0].1.is_empty() {
                root.join("sh.exe").join("")
            } else {
                root.join("second/sh.exe")
            };
            assert_eq!(
                shell.get_program(),
                expected,
                "a fallback shell uses the selected PATH without ambient lookup"
            );
            assert_eq!(
                shell.get_args().collect::<Vec<_>>(),
                [OsStr::new("--posix")],
                "shell shim options survive program resolution"
            );
        }
        let missing = windows_command("missing".into(), &environment, &[], Some(&root))?;
        assert_eq!(
            missing.get_program(),
            root.join("first/missing"),
            "a failed relative PATH lookup cannot search elsewhere"
        );
        let missing_interpreter =
            windows_command("./missing-hook".into(), &[("PATH".into(), "".into())], &[], Some(&root))?;
        assert_eq!(
            missing_interpreter.get_program(),
            root.join("absent.exe").join(""),
            "an empty interpreter PATH also stays confined to the child directory"
        );
        assert_eq!(
            windows_command("".into(), &environment, &[], Some(&root))?.get_program(),
            "",
            "an empty command must not become the working directory"
        );
        Ok(())
    }

    #[test]
    fn windows_shebang_interpreters_use_the_selected_path() -> gix_testtools::TestResult {
        let root = gix_testtools::scripted_fixture_read_only("windows_shebang.sh")?;
        let paths = std::env::join_paths([root.join("first"), root.join("second")])?;
        let environment = [(OsString::from("PATH"), paths.clone())];
        for (script, interpreter) in [
            ("hook", "sh.exe"),
            ("backslash-hook", "sh.exe"),
            ("exe-hook", "sh.exe"),
            ("dotted-hook", "python3.11.exe"),
        ] {
            let script = root.join(script);
            let command = windows_command(script.clone().into(), &environment, &[], None)?;
            assert_eq!(
                command.get_program(),
                root.join("second").join(interpreter),
                "the interpreter basename resolves to an .exe in PATH, skipping extensionless files"
            );
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [script.as_os_str()],
                "only the literal script path is passed; shebang options are ignored like Git for Windows"
            );
        }
        let command = windows_command(
            root.join("hook").into(),
            &environment,
            &[("PATH".to_owned(), root.join("third").into())],
            None,
        )?;
        assert_eq!(
            command.get_program(),
            root.join("third").join("sh.exe"),
            "inline PATH assignments also govern interpreter lookup"
        );
        for paths in [paths, OsString::new()] {
            let command = windows_command(
                root.join("missing-hook").into(),
                &[(OsString::from("PATH"), paths.clone())],
                &[],
                None,
            )?;
            assert_eq!(
                command.get_program(),
                prevent_further_path_lookup(Path::new("absent.exe"), &paths, None)?,
                "a missing interpreter stays confined to the selected PATH"
            );
        }
        Ok(())
    }

    #[test]
    fn explicit_path_lookup_failure_stays_within_that_path() -> gix_testtools::TestResult {
        let joined_paths = std::env::join_paths(["", "not/a/real/path", "also/not/real"])?;
        let cmd = windows_command("missing.exe".into(), &[], &[("PATH".into(), joined_paths)], None)?;
        assert_eq!(
            cmd.get_program(),
            std::path::Path::new("not/a/real/path/missing.exe"),
            "an inline PATH miss must not leave a bare program for Rust to resolve elsewhere"
        );
        Ok(())
    }
}
