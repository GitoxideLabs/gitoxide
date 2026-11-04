use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::{self, Read, Write},
    net::Shutdown,
    os::unix::{
        fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt},
        net::{UnixListener, UnixStream},
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, ResultExt, bail, message, validation};

use crate::{Program, helper::Action, program::Kind};

/// An explicit executable entrypoint for the cache daemon.
///
/// The socket's absolute path is appended to `args`. The child must bind its socket, write exactly
/// `ok\n` to stdout, close or redirect stdout, and continue serving requests in the background.
/// Neither Git nor the current executable is inferred by the library.
#[derive(Debug, Clone)]
pub struct DaemonCommand {
    /// The executable to start.
    pub executable: PathBuf,
    /// Arguments preceding the socket path.
    pub args: Vec<OsString>,
}

/// Resolve Git's default cache socket from the home and XDG cache directories.
/// The legacy home directory takes precedence when it already exists.
pub fn default_socket() -> Result<PathBuf> {
    default_socket_with_environment(&[], None)
}

fn default_socket_with_environment(
    environment: &[(OsString, OsString)],
    current_dir: Option<&Path>,
) -> Result<PathBuf> {
    let env_var = |name: &str| {
        environment
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.clone())
            .or_else(|| gix_path::env::var(name))
    };
    let home = env_var("HOME").map(PathBuf::from);
    if let Some(legacy) = home.as_ref().map(|home| home.join(".git-credential-cache"))
        && current_dir.unwrap_or(Path::new("")).join(&legacy).is_dir()
    {
        return Ok(legacy.join("socket"));
    }
    env_var("XDG_CACHE_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join(".cache")))
        .map(|cache| cache.join("git/credential/socket"))
        .ok_or_else(|| validation("Credential cache needs a home directory, XDG_CACHE_HOME or --socket").raise())
}

/// Send a credential-cache `action` and raw helper `input` to `socket`.
///
/// `timeout` is the cache lifetime in seconds for `store`, with Git's default being 900.
/// Missing daemons yield an empty response for `get`, `erase`, and `exit`. Only `store` starts the
/// explicitly configured `launcher`; an absent launcher is an error. Unknown actions do nothing.
/// The socket path must be absolute and fit the platform's Unix socket address limit.
pub fn request(
    socket: &Path,
    action: &str,
    timeout: i32,
    input: &[u8],
    launcher: Option<&DaemonCommand>,
) -> Result<Vec<u8>> {
    request_inner(socket, action, timeout, input, launcher, true, &[], None)
}

#[expect(clippy::too_many_arguments)]
fn request_inner(
    socket: &Path,
    action: &str,
    timeout: i32,
    input: &[u8],
    launcher: Option<&DaemonCommand>,
    stderr: bool,
    environment: &[(OsString, OsString)],
    current_dir: Option<&Path>,
) -> Result<Vec<u8>> {
    if action == "capability" {
        return Ok(b"version 0\ncapability authtype\n".to_vec());
    }
    if !matches!(action, "get" | "store" | "erase" | "exit") {
        return Ok(Vec::new());
    }
    if !socket.is_absolute() {
        bail!(validation("Credential cache socket must be an absolute path"));
    }
    let mut payload = format!("action={action}\ntimeout={timeout}\n").into_bytes();
    if action != "exit" {
        payload.extend_from_slice(input);
    }
    match send(socket, &payload) {
        Ok(response) => Ok(response),
        Err(err) if absent(&err) && action != "store" => Ok(Vec::new()),
        Err(err) if absent(&err) => {
            let launcher = launcher.ok_or_else(|| {
                message("Credential cache daemon is not running and no daemon launcher is configured").raise()
            })?;
            let launched = launch(launcher, socket, stderr, environment, current_dir);
            // Another client may have won a concurrent startup, in which case its daemon serves us.
            let response = send(socket, &payload);
            match response {
                Ok(response) => Ok(response),
                Err(err) => {
                    launched?;
                    Err(err.and_raise(message("Could not connect to credential cache daemon")))
                }
            }
        }
        Err(err) => Err(err.and_raise(message("Could not communicate with credential cache daemon"))),
    }
}

fn absent(err: &io::Error) -> bool {
    matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused)
}

fn send(socket: &Path, payload: &[u8]) -> io::Result<Vec<u8>> {
    let mut connection = UnixStream::connect(socket)?;
    connection.write_all(payload)?;
    connection.shutdown(Shutdown::Write)?;
    let mut response = Vec::new();
    match connection.read_to_end(&mut response) {
        Ok(_) => Ok(response),
        Err(err)
            if matches!(
                err.kind(),
                io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted
            ) =>
        {
            Ok(response)
        }
        Err(err) => Err(err),
    }
}

fn launch(
    launcher: &DaemonCommand,
    socket: &Path,
    stderr: bool,
    environment: &[(OsString, OsString)],
    current_dir: Option<&Path>,
) -> Result<()> {
    let mut command = Command::new(&launcher.executable);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    let mut child = command
        .args(&launcher.args)
        .arg(socket)
        .envs(environment.iter().map(|(key, value)| (key, value)))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(if stderr { Stdio::inherit() } else { Stdio::null() })
        .spawn()
        .or_raise(|| message("Could not start credential cache daemon"))?;
    let mut ready = Vec::new();
    let result = child
        .stdout
        .take()
        .expect("stdout is piped")
        .take(128)
        .read_to_end(&mut ready);
    if result.is_err() || ready != b"ok\n" {
        child.kill().ok();
        child
            .wait()
            .or_raise(|| message("Could not reap failed credential cache daemon"))?;
        bail!(message("Credential cache daemon did not report successful startup"));
    }
    // A long-running caller must also reap its daemon when credentials eventually expire.
    std::thread::spawn(move || {
        child.wait().ok();
    });
    Ok(())
}

pub(crate) fn invoke(
    helper: &Program,
    action: &Action,
    environment: &[(OsString, OsString)],
) -> Option<Result<Option<Vec<u8>>>> {
    let Kind::ExternalName { name_and_args } = &helper.kind else {
        return None;
    };
    if !super::builtin::is_static(name_and_args) {
        return None;
    }
    let parsed = gix_command::parse::command_line(name_and_args.as_bstr()).ok()?;
    if !parsed.env.is_empty() || parsed.command != "cache" {
        return None;
    }
    Some((|| {
        let mut socket = None;
        let mut timeout = 900;
        let mut args = parsed.args.iter();
        while let Some(arg) = args.next() {
            let arg = gix_path::os_str_into_bstr(arg)?;
            match arg.as_bytes() {
                b"--socket" => {
                    socket =
                        Some(PathBuf::from(args.next().ok_or_else(|| {
                            validation("Credential cache --socket needs a path").raise()
                        })?));
                }
                b"--timeout" => {
                    timeout = parse_timeout(
                        gix_path::os_str_into_bstr(
                            args.next()
                                .ok_or_else(|| validation("Credential cache --timeout needs seconds").raise())?,
                        )?
                        .as_ref(),
                    )?;
                }
                b"--no-socket" => socket = None,
                b"--no-timeout" => timeout = 0,
                b"--" if args.len() == 0 => {}
                value if value.starts_with(b"--socket=") => {
                    socket = Some(gix_path::from_bstr(value[9..].as_bstr())?.into_owned());
                }
                value if value.starts_with(b"--timeout=") => timeout = parse_timeout(&value[10..])?,
                _ => bail!(validation("Unsupported credential cache option")),
            }
        }
        let socket = match socket {
            Some(socket) => socket,
            None => default_socket_with_environment(environment, helper.current_dir.as_deref())?,
        };
        let mut input = Vec::new();
        action
            .send(&mut input)
            .map_err(|_| validation("Invalid credential cache input").raise())?;
        let response = request_inner(
            &socket,
            action.as_arg(true),
            timeout,
            &input,
            helper.cache_daemon.as_ref(),
            helper.stderr,
            environment,
            helper.current_dir.as_deref(),
        )?;
        Ok(matches!(action, Action::Get(_)).then_some(response))
    })())
}

fn parse_timeout(input: &[u8]) -> Result<i32> {
    std::str::from_utf8(input)
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| validation("Credential cache timeout must be an integer number of seconds").raise())
}

/// A bound Unix credential-cache daemon, storing credentials only in memory.
///
/// Binding does not announce readiness or change process-wide state. Executable entrypoints can use
/// [`run_with_stdio()`][Self::run_with_stdio()] or announce readiness before calling [`run()`][Self::run()].
/// Dropping the daemon removes its socket.
pub struct Daemon {
    listener: UnixListener,
    socket: PathBuf,
    identity: (u64, u64),
}

impl Daemon {
    /// Bind `socket` in a private directory, creating the final directory with mode `0700`.
    ///
    /// Existing directories accessible by other users are rejected. A stale socket is removed, but
    /// a live daemon or an unrelated file is left intact. Paths must be absolute and fit the native
    /// Unix socket path limit; this library never changes the process's working directory.
    pub fn bind(socket: &Path) -> Result<Self> {
        if !socket.is_absolute() {
            bail!(validation("Credential cache socket must be an absolute path"));
        }
        let directory = socket
            .parent()
            .ok_or_else(|| validation("Credential cache socket needs a parent directory").raise())?;
        if !directory.exists() {
            if let Some(parent) = directory.parent() {
                fs::create_dir_all(parent).or_error()?;
            }
            match fs::DirBuilder::new().mode(0o700).create(directory) {
                Ok(()) => {}
                Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
                Err(err) => return Err(err.and_raise(message("Could not create credential cache directory"))),
            }
        }
        let metadata = fs::metadata(directory).or_raise(|| message("Could not inspect credential cache directory"))?;
        if !metadata.is_dir() || metadata.permissions().mode() & 0o077 != 0 {
            bail!(validation(
                "Credential cache socket directory must be private (mode 0700)"
            ));
        }
        let _lock = gix_lock::File::acquire(socket, Duration::from_secs(1).into(), None, -0o600, None)
            .or_raise(|| message("Could not lock credential cache startup"))?;
        match fs::symlink_metadata(socket) {
            Ok(metadata) => {
                if !metadata.file_type().is_socket() {
                    bail!(validation("Credential cache socket path is occupied by another file"));
                }
                match UnixStream::connect(socket) {
                    Err(err) if absent(&err) => fs::remove_file(socket).or_error()?,
                    Ok(_) => bail!(message("Credential cache daemon is already running")),
                    Err(err) => {
                        return Err(err.and_raise(message("Could not inspect existing credential cache daemon")));
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.and_raise(message("Could not inspect credential cache socket"))),
        }
        let listener = UnixListener::bind(socket).or_raise(|| {
            message("Could not bind credential cache socket (path must fit the platform's Unix socket limit)")
        })?;
        let metadata = fs::symlink_metadata(socket).or_error()?;
        Ok(Daemon {
            listener,
            socket: socket.into(),
            identity: (metadata.dev(), metadata.ino()),
        })
    }

    /// Announce readiness and serve as a dedicated daemon process.
    ///
    /// Call this only from an executable's daemon entrypoint, after installing its signal policy.
    /// This writes and flushes `ok\n`, then redirects stdout to the null device so the launching
    /// client's readiness pipe reaches EOF. Unless `keep_stderr` is set, stderr is redirected too.
    /// The daemon then runs until [`run()`][Self::run()] finishes; the caller owns process termination.
    pub fn run_with_stdio(self, keep_stderr: bool, interrupt: &AtomicBool) -> Result<()> {
        let null = fs::OpenOptions::new().write(true).open("/dev/null").or_error()?;
        {
            let mut stdout = io::stdout().lock();
            stdout.write_all(b"ok\n").or_error()?;
            stdout.flush().or_error()?;
            rustix::stdio::dup2_stdout(&null).or_error()?;
        }
        if !keep_stderr {
            rustix::stdio::dup2_stderr(&null).or_error()?;
        }
        self.run(interrupt)
    }

    /// Serve until an `exit` request, interruption, or 30 seconds without any cached credential.
    ///
    /// Expired and erased credentials are discarded before another client is served. Each removal
    /// gives clients another 30 seconds to provide a replacement. `interrupt` is checked at least
    /// once per second while waiting for clients. Malformed requests are ignored without disclosing
    /// their contents. Like Git, each connected client's request is read before serving the next.
    pub fn run(self, interrupt: &AtomicBool) -> Result<()> {
        use rustix::event::{PollFd, PollFlags, Timespec, poll};
        let mut entries: Vec<(Credential, Instant)> = Vec::new();
        let mut empty_until = Instant::now() + Duration::from_secs(30);
        while !interrupt.load(Ordering::Relaxed) {
            let now = Instant::now();
            let previous_len = entries.len();
            entries.retain(|(_, expiry)| *expiry > now);
            if entries.len() != previous_len {
                empty_until = now + Duration::from_secs(30);
            }
            if entries.is_empty() && now >= empty_until {
                break;
            }
            let deadline = entries.iter().map(|(_, expiry)| *expiry).min().unwrap_or(empty_until);
            let duration = deadline.saturating_duration_since(now).min(Duration::from_secs(1));
            let timeout = Timespec {
                tv_sec: duration.as_secs() as i64,
                tv_nsec: i64::from(duration.subsec_nanos()),
            };
            let mut descriptors = [PollFd::new(&self.listener, PollFlags::IN)];
            match poll(&mut descriptors, Some(&timeout)) {
                Ok(0) => continue,
                Ok(_) => {}
                Err(rustix::io::Errno::INTR) => continue,
                Err(err) => return Err(err.and_raise(message("Could not wait for credential cache clients"))),
            }
            let (mut connection, _) = self.listener.accept().or_error()?;
            let mut input = Vec::new();
            if connection.read_to_end(&mut input).is_err() {
                continue;
            }
            let mut lines = input.splitn(3, |byte| *byte == b'\n');
            let Some(action) = lines.next().and_then(|line| line.strip_prefix(b"action=")) else {
                continue;
            };
            let Some(timeout) = lines
                .next()
                .and_then(|line| line.strip_prefix(b"timeout="))
                .and_then(|value| parse_timeout(value).ok())
            else {
                continue;
            };
            let Some(credential) = Credential::parse(lines.next().unwrap_or_default()) else {
                continue;
            };
            match action {
                b"exit" => {
                    // Remove the socket before EOF tells the caller it can start a new daemon.
                    self.remove_socket().or_error()?;
                    break;
                }
                b"get" => {
                    if let Some((entry, _)) = entries.iter().find(|(entry, _)| credential.matches(entry, false)) {
                        entry
                            .write_response(&mut connection, credential.authtype_capability)
                            .ok();
                    }
                }
                b"erase" => {
                    let len = entries.len();
                    entries.retain(|(entry, _)| !credential.matches(entry, true));
                    if entries.len() != len {
                        empty_until = Instant::now() + Duration::from_secs(30);
                    }
                }
                b"store" if timeout >= 0 && !credential.ephemeral && credential.complete() => {
                    entries.retain(|(entry, _)| !credential.matches(entry, false));
                    entries.push((credential, Instant::now() + Duration::from_secs(timeout as u64)));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Daemon {
    fn remove_socket(&self) -> io::Result<()> {
        match fs::symlink_metadata(&self.socket) {
            Ok(metadata) if (metadata.dev(), metadata.ino()) == self.identity => fs::remove_file(&self.socket),
            Ok(_) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        self.remove_socket().ok();
    }
}

#[derive(Default)]
struct Credential {
    fields: BTreeMap<&'static str, Vec<u8>>,
    ephemeral: bool,
    authtype_capability: bool,
}

impl Credential {
    fn parse(input: &[u8]) -> Option<Self> {
        const FIELDS: [&str; 10] = [
            "protocol",
            "host",
            "path",
            "username",
            "password",
            "authtype",
            "credential",
            "password_expiry_utc",
            "oauth_refresh_token",
            "quit",
        ];
        let mut credential = Credential::default();
        for line in input.split(|byte| *byte == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
                break;
            }
            if line.contains(&0) {
                return None;
            }
            let separator = line.find_byte(b'=')?;
            let (key, value) = (&line[..separator], &line[separator + 1..]);
            match key {
                b"url" => {
                    let url = super::builtin::parse_url(value)?;
                    credential = Credential::default();
                    credential
                        .fields
                        .extend([("protocol", url.protocol), ("host", url.host)]);
                    for (key, value) in [
                        ("username", url.username),
                        ("password", url.password),
                        ("path", url.path),
                    ] {
                        if let Some(value) = value {
                            credential.fields.insert(key, value);
                        }
                    }
                }
                b"capability[]" if value == b"authtype" => credential.authtype_capability = true,
                b"ephemeral" => credential.ephemeral = gix_config_value::Boolean::try_from(value.as_bstr()).ok()?.0,
                b"password_expiry_utc" => {
                    if let Some(expiry) = std::str::from_utf8(value)
                        .ok()
                        .and_then(|value| value.parse::<u64>().ok())
                        .filter(|expiry| *expiry != 0)
                    {
                        credential
                            .fields
                            .insert("password_expiry_utc", expiry.to_string().into_bytes());
                    } else {
                        credential.fields.remove("password_expiry_utc");
                    }
                }
                _ => {
                    if let Some(key) = FIELDS.iter().find(|field| field.as_bytes() == key) {
                        credential.fields.insert(key, value.into());
                    }
                }
            }
        }
        Some(credential)
    }

    fn complete(&self) -> bool {
        self.fields.contains_key("username") && self.fields.contains_key("password")
            || self.fields.contains_key("authtype") && self.fields.contains_key("credential")
    }

    fn matches(&self, entry: &Self, password: bool) -> bool {
        ["protocol", "host", "path", "username"]
            .into_iter()
            .chain(password.then_some("password"))
            .chain(password.then_some("credential"))
            .all(|key| {
                self.fields
                    .get(key)
                    .is_none_or(|value| entry.fields.get(key) == Some(value))
            })
    }

    fn write_response(&self, out: &mut impl Write, authtype: bool) -> io::Result<()> {
        out.write_all(b"capability[]=authtype\n")?;
        for key in [
            "username",
            "password",
            "authtype",
            "credential",
            "password_expiry_utc",
            "oauth_refresh_token",
        ] {
            if matches!(key, "authtype" | "credential") && !authtype {
                continue;
            }
            if let Some(value) = self.fields.get(key) {
                out.write_all(key.as_bytes())?;
                out.write_all(b"=")?;
                out.write_all(value)?;
                out.write_all(b"\n")?;
            }
        }
        Ok(())
    }
}
