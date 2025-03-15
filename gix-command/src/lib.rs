//! Launch commands very similarly to `Command`, but with `git` specific capabilities and adjustments.
#![deny(rust_2018_idioms, missing_docs)]
#![forbid(unsafe_code)]

use std::io::Read;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bstr::{BString, ByteSlice};

/// A structure to keep settings to use when invoking a command via [`spawn()`][Prepare::spawn()],
/// after creating it with [`prepare()`].
pub struct Prepare {
    /// The command to invoke, either directly or with a shell depending on `use_shell`.
    pub command: OsString,
    /// Additional information to be passed to the spawned command.
    pub context: Option<Context>,
    /// The way standard input is configured.
    pub stdin: std::process::Stdio,
    /// The way standard output is configured.
    pub stdout: std::process::Stdio,
    /// The way standard error is configured.
    pub stderr: std::process::Stdio,
    /// The arguments to pass to the process being spawned.
    pub args: Vec<OsString>,
    /// Environment variables to set for the spawned process.
    pub env: Vec<(OsString, OsString)>,
    /// If `true`, we will use `shell_program` or `sh` to execute the `command`.
    pub use_shell: bool,
    /// If `true`, `command` is assumed to be a command or path to the program to execute, and it
    /// will be shell-quoted to assure it will be executed as is and without splitting across
    /// whitespace.
    pub quote_command: bool,
    /// The name or path to the shell program to use instead of `sh`.
    pub shell_program: Option<OsString>,
    /// If `true` (default `true` on Windows and `false` everywhere else) we will see if it's safe
    /// to manually invoke `command` after splitting its arguments as a shell would do.
    ///
    /// Note that outside of Windows, it's generally not advisable as this removes support for
    /// literal shell scripts with shell-builtins.
    ///
    /// This mimics the behaviour we see with `git` on Windows, which also won't invoke the shell
    /// there at all.
    ///
    /// Only effective if `use_shell` is `true` as well, as the shell will be used as a fallback if
    /// it's not possible to split arguments as the command-line contains 'scripting'.
    pub allow_manual_arg_splitting: bool,
}

/// Additional information that is relevant to spawned processes, which typically receive
/// a wealth of contextual information when spawned from `git`.
///
/// See [the git source code](https://github.com/git/git/blob/cfb8a6e9a93adbe81efca66e6110c9b4d2e57169/git.c#L191)
/// for details.
#[derive(Debug, Default, Clone)]
pub struct Context {
    /// The `.git` directory that contains the repository.
    ///
    /// If set, it will be used to set the `GIT_DIR` environment variable.
    pub git_dir: Option<PathBuf>,
    /// Set the `GIT_WORK_TREE` environment variable with the given path.
    pub worktree_dir: Option<PathBuf>,
    /// If `true`, set `GIT_NO_REPLACE_OBJECTS` to `1`, which turns off object replacements, or `0` otherwise.
    /// If `None`, the variable won't be set.
    pub no_replace_objects: Option<bool>,
    /// Set the `GIT_NAMESPACE` variable with the given value, effectively namespacing all
    /// operations on references.
    pub ref_namespace: Option<BString>,
    /// If `true`, set `GIT_LITERAL_PATHSPECS` to `1`, which makes globs literal and prefixes as well, or `0` otherwise.
    /// If `None`, the variable won't be set.
    pub literal_pathspecs: Option<bool>,
    /// If `true`, set `GIT_GLOB_PATHSPECS` to `1`, which lets wildcards not match the `/` character, and equals the `:(glob)` prefix.
    /// If `false`, set `GIT_NOGLOB_PATHSPECS` to `1` which lets globs match only themselves.
    /// If `None`, the variable won't be set.
    pub glob_pathspecs: Option<bool>,
    /// If `true`, set `GIT_ICASE_PATHSPECS` to `1`, to let patterns match case-insensitively, or `0` otherwise.
    /// If `None`, the variable won't be set.
    pub icase_pathspecs: Option<bool>,
    /// If `true`, inherit `stderr` just like it's the default when spawning processes.
    /// If `false`, suppress all stderr output.
    /// If not `None`, this will override any value set with [`Prepare::stderr()`].
    pub stderr: Option<bool>,
}

mod prepare {
    use std::borrow::Cow;
    use std::{
        ffi::OsString,
        process::{Command, Stdio},
    };

    use bstr::ByteSlice;

    use crate::{extract_interpreter, win_path_lookup, Context, Prepare};

    /// Builder
    impl Prepare {
        /// If called, the command will be checked for characters that are typical for shell
        /// scripts, and if found will use `sh` to execute it or whatever is set as
        /// [`with_shell_program()`](Self::with_shell_program()).
        ///
        /// If the command isn't valid UTF-8, a shell will always be used.
        ///
        /// If a shell is used, then arguments given here with [arg()](Self::arg) or
        /// [args()](Self::args) will be substituted via `"$@"` if it's not already present in the
        /// command.
        ///
        ///
        /// The [`command_may_be_shell_script_allow_manual_argument_splitting()`](Self::command_may_be_shell_script_allow_manual_argument_splitting())
        /// and [`command_may_be_shell_script_disallow_manual_argument_splitting()`](Self::command_may_be_shell_script_disallow_manual_argument_splitting())
        /// methods also call this method.
        ///
        /// If neither this method nor [`with_shell()`](Self::with_shell()) is called, commands are
        /// always executed verbatim and directly, without the use of a shell.
        pub fn command_may_be_shell_script(mut self) -> Self {
            self.use_shell = self.command.to_str().map_or(true, |cmd| {
                cmd.as_bytes().find_byteset(b"|&;<>()$`\\\"' \t\n*?[#~=%").is_some()
            });
            self
        }

        /// If called, unconditionally use a shell to execute the command and its arguments.
        ///
        /// This uses `sh` to execute it, or whatever is set as
        /// [`with_shell_program()`](Self::with_shell_program()).
        ///
        /// Arguments given here with [arg()](Self::arg) or [args()](Self::args) will be
        /// substituted via `"$@"` if it's not already present in the command.
        ///
        /// If neither this method nor
        /// [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) is called,
        /// commands are always executed verbatim and directly, without the use of a shell. (But
        /// see [`command_may_be_shell_script()`](Self::command_may_be_shell_script()) on other
        /// methods that call that method.)
        pub fn with_shell(mut self) -> Self {
            self.use_shell = true;
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
        /// Note that that shells that are not Bourne-style cannot be expected to work correctly,
        /// because POSIX shell syntax is assumed when searching for and conditionally adding
        /// `"$@"` to receive arguments, where applicable (and in the behaviour of
        /// [`with_quoted_command()`](Self::with_quoted_command()), if called).
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
        pub fn spawn(self) -> std::io::Result<std::process::Child> {
            let mut cmd = Command::from(self);
            gix_trace::debug!(cmd = ?cmd);
            cmd.spawn()
        }
    }

    impl From<Prepare> for Command {
        fn from(mut prep: Prepare) -> Command {
            let mut cmd = if prep.use_shell {
                let split_args = prep
                    .allow_manual_arg_splitting
                    .then(|| {
                        if gix_path::into_bstr(std::borrow::Cow::Borrowed(prep.command.as_ref()))
                            .find_byteset(b"\\|&;<>()$`\n*?[#~%")
                            .is_none()
                        {
                            prep.command
                                .to_str()
                                .and_then(|args| shell_words::split(args).ok().map(Vec::into_iter))
                        } else {
                            None
                        }
                    })
                    .flatten();
                match split_args {
                    Some(mut args) => {
                        let mut cmd = Command::new(args.next().expect("non-empty input"));
                        cmd.args(args);
                        cmd
                    }
                    None => {
                        let shell = prep.shell_program.unwrap_or_else(|| gix_path::env::shell().into());
                        let mut cmd = Command::new(shell);
                        cmd.arg("-c");
                        if !prep.args.is_empty() {
                            if prep.command.to_str().map_or(true, |cmd| !cmd.contains("$@")) {
                                if prep.quote_command {
                                    if let Ok(command) = gix_path::os_str_into_bstr(&prep.command) {
                                        prep.command = gix_path::from_bstring(gix_quote::single(command)).into();
                                    }
                                }
                                prep.command.push(" \"$@\"");
                            } else {
                                gix_trace::debug!(
                                    "Will not add '\"$@\"' to '{:?}' as it seems to contain '$@' already",
                                    prep.command
                                );
                            }
                        }
                        cmd.arg(prep.command);
                        cmd.arg("--");
                        cmd
                    }
                }
            } else if cfg!(windows) {
                let program: Cow<'_, std::path::Path> = std::env::var_os("PATH")
                    .and_then(|path| win_path_lookup(prep.command.as_ref(), &path))
                    .map(Cow::Owned)
                    .unwrap_or(Cow::Borrowed(prep.command.as_ref()));
                if let Some(shebang) = extract_interpreter(program.as_ref()) {
                    let mut cmd = Command::new(shebang.interpreter);
                    // For relative paths, we may have picked up a file in the current repository
                    // for which an attacker could control everything. Hence, strip options just like Git.
                    // If the file was found in the PATH though, it should be trustworthy.
                    if program.is_absolute() {
                        cmd.args(shebang.args);
                    }
                    cmd.arg(prep.command);
                    cmd
                } else {
                    Command::new(prep.command)
                }
            } else {
                Command::new(prep.command)
            };
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
                    cmd.env("GIT_NAMESPACE", gix_path::from_bstring(namespace));
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
            cmd
        }
    }
}

fn is_exe(executable: &Path) -> bool {
    executable.extension() == Some(std::ffi::OsStr::new("exe"))
}

/// Try to find `command` in the `path_value` (the value of `PATH`) as separated by `;`, or return `None`.
/// Has special handling for `.exe` extensions, as these will be appended automatically if needed.
/// Note that just like Git, no lookup is performed if a slash or backslash is in `command`.
fn win_path_lookup(command: &Path, path_value: &std::ffi::OsStr) -> Option<PathBuf> {
    fn lookup(root: &bstr::BStr, command: &Path, is_exe: bool) -> Option<PathBuf> {
        let mut path = gix_path::try_from_bstr(root).ok()?.join(command);
        if !is_exe {
            path.set_extension("exe");
        }
        if path.is_file() {
            return Some(path);
        }
        if is_exe {
            return None;
        }
        path.set_extension("");
        path.is_file().then_some(path)
    }
    if command.components().take(2).count() == 2 {
        return None;
    }
    let path = gix_path::os_str_into_bstr(path_value).ok()?;
    let is_exe = is_exe(command);

    for root in path.split(|b| *b == b';') {
        if let Some(executable) = lookup(root.as_bstr(), command, is_exe) {
            return Some(executable);
        }
    }
    None
}

/// Parse the shebang (`#!<path>`) from the first line of `executable`, and return the shebang
/// data when available.
pub fn extract_interpreter(executable: &Path) -> Option<shebang::Data> {
    #[cfg(windows)]
    if is_exe(executable) {
        return None;
    }
    let mut buf = [0; 100]; // Note: just like Git
    let mut file = std::fs::File::open(executable).ok()?;
    let n = file.read(&mut buf).ok()?;
    shebang::parse(buf[..n].as_bstr())
}

///
pub mod shebang {
    use bstr::{BStr, ByteSlice};
    use std::ffi::OsString;
    use std::path::PathBuf;

    /// Parse `buf` to extract all shebang information.
    pub fn parse(buf: &BStr) -> Option<Data> {
        let mut line = buf.lines().next()?;
        line = line.strip_prefix(b"#!")?;

        let slash_idx = line.rfind_byteset(b"/\\")?;
        Some(match line[slash_idx..].find_byte(b' ') {
            Some(space_idx) => {
                let space = slash_idx + space_idx;
                Data {
                    interpreter: gix_path::from_byte_slice(line[..space].trim()).to_owned(),
                    args: line
                        .get(space + 1..)
                        .and_then(|mut r| {
                            r = r.trim();
                            if r.is_empty() {
                                return None;
                            }

                            match r.as_bstr().to_str() {
                                Ok(args) => shell_words::split(args)
                                    .ok()
                                    .map(|args| args.into_iter().map(Into::into).collect()),
                                Err(_) => Some(vec![gix_path::from_byte_slice(r).to_owned().into()]),
                            }
                        })
                        .unwrap_or_default(),
                }
            }
            None => Data {
                interpreter: gix_path::from_byte_slice(line.trim()).to_owned(),
                args: Vec::new(),
            },
        })
    }

    /// Shebang information as [parsed](parse()) from a buffer that should contain at least one line.
    ///
    /// ### Deviation
    ///
    /// According to the [shebang documentation](https://en.wikipedia.org/wiki/Shebang_(Unix)), it will only consider
    /// the path of the executable, along with the arguments as the consecutive portion after the space that separates
    /// them. Argument splitting would then have to be done elsewhere, probably in the kernel.
    ///
    /// To make that work without the kernel, we perform the splitting while Git just ignores options.
    /// For now it seems more compatible to not ignore options, but if it is important this could be changed.
    #[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
    pub struct Data {
        /// The interpreter to run.
        pub interpreter: PathBuf,
        /// The remainder of the line past the space after `interpreter`, without leading or trailing whitespace,
        /// as pre-split arguments just like a shell would do it.
        /// Note that we accept that illformed UTF-8 will prevent argument splitting.
        pub args: Vec<OsString>,
    }
}

/// Prepare `cmd` for [spawning][std::process::Command::spawn()] by configuring it with various builder methods.
///
/// Note that the default IO is configured for typical API usage, that is
///
/// - `stdin` is null to prevent blocking unexpectedly on consumption of stdin
/// - `stdout` is captured for consumption by the caller
/// - `stderr` is inherited to allow the command to provide context to the user
///
/// On Windows, terminal Windows will be suppressed automatically.
///
/// ### Warning
///
/// When using this method, be sure that the invoked program doesn't rely on the current working dir and/or
/// environment variables to know its context. If so, call instead [`Prepare::with_context()`] to provide
/// additional information.
pub fn prepare(cmd: impl Into<OsString>) -> Prepare {
    Prepare {
        command: cmd.into(),
        shell_program: None,
        context: None,
        stdin: std::process::Stdio::null(),
        stdout: std::process::Stdio::piped(),
        stderr: std::process::Stdio::inherit(),
        args: Vec::new(),
        env: Vec::new(),
        use_shell: false,
        quote_command: false,
        allow_manual_arg_splitting: cfg!(windows),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_win_path_lookup() -> gix_testtools::Result {
        let root = gix_testtools::scripted_fixture_read_only("win_path_lookup.sh")?;
        let mut paths: Vec<_> = std::fs::read_dir(&root)?
            .filter_map(Result::ok)
            .map(|e| e.path().to_str().expect("no illformed UTF8").to_owned())
            .collect();
        paths.sort();
        let lookup_path: OsString = paths.join(";").into();

        assert_eq!(
            win_path_lookup("a/b".as_ref(), &lookup_path),
            None,
            "any path with separator is considered ready to use"
        );
        assert_eq!(
            win_path_lookup("x".as_ref(), &lookup_path),
            Some(root.join("a").join("x.exe")),
            "exe will be preferred, and it searches left to right thus doesn't find c/x.exe"
        );
        assert_eq!(
            win_path_lookup("x.exe".as_ref(), &lookup_path),
            Some(root.join("a").join("x.exe")),
            "no matter what, a/x won't be found as it's shadowed by an exe file"
        );
        assert_eq!(
            win_path_lookup("exe".as_ref(), &lookup_path),
            Some(root.join("b").join("exe")),
            "it finds files further down the path as well"
        );
        Ok(())
    }
}
