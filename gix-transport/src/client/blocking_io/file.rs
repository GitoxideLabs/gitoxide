use std::{
    any::Any,
    borrow::Cow,
    error::Error,
    ffi::{OsStr, OsString},
    io::Write,
    process::{self, Stdio},
};

use bstr::{io::BufReadExt, BStr, BString, ByteSlice};

use crate::{
    client::{
        self,
        blocking_io::{ssh, RequestWriter, SetServiceResponse},
        git::blocking_io::Connection,
        MessageKind, WriteMode,
    },
    Protocol, Service,
};

// from https://github.com/git/git/blob/20de7e7e4f4e9ae52e6cc7cfaa6469f186ddb0fa/environment.c#L115:L115
const ENV_VARS_TO_REMOVE: &[&str] = &[
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_INTERNAL_SUPER_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
    "GIT_CONFIG_COUNT",
];

/// A utility to spawn a helper process to actually transmit data, possibly over `ssh`.
///
/// It can only be instantiated using the local [`connect()`] or [ssh connect][crate::client::ssh::connect()].
pub struct SpawnProcessOnDemand {
    desired_version: Protocol,
    url: gix_url::Url,
    path: BString,
    ssh_cmd: Option<(OsString, ssh::ProgramKind)>,
    /// The environment variables to set in the invoked command.
    envs: Vec<(&'static str, String)>,
    ssh_disallow_shell: bool,
    connection: Option<Connection<Box<dyn std::io::Read + Send>, process::ChildStdin>>,
    child: Option<process::Child>,
    trace: bool,
}

impl SpawnProcessOnDemand {
    pub(crate) fn new_ssh(
        url: gix_url::Url,
        program: impl Into<OsString>,
        path: BString,
        ssh_kind: ssh::ProgramKind,
        ssh_disallow_shell: bool,
        version: Protocol,
        trace: bool,
    ) -> SpawnProcessOnDemand {
        SpawnProcessOnDemand {
            url,
            path,
            ssh_cmd: Some((program.into(), ssh_kind)),
            envs: Default::default(),
            ssh_disallow_shell,
            child: None,
            connection: None,
            desired_version: version,
            trace,
        }
    }
    fn new_local(path: BString, version: Protocol, trace: bool) -> SpawnProcessOnDemand {
        SpawnProcessOnDemand {
            url: gix_url::Url::from_parts(gix_url::Scheme::File, None, None, None, None, path.clone(), true)
                .expect("valid url"),
            path,
            ssh_cmd: None,
            envs: if version != Protocol::V1 {
                vec![("GIT_PROTOCOL", format!("version={}", version as usize))]
            } else {
                Default::default()
            },
            ssh_disallow_shell: false,
            child: None,
            connection: None,
            desired_version: version,
            trace,
        }
    }
}

impl client::TransportWithoutIO for SpawnProcessOnDemand {
    fn set_identity(&mut self, identity: gix_sec::identity::Account) -> Result<(), client::Error> {
        if self.url.scheme == gix_url::Scheme::Ssh {
            self.url
                .set_user((!identity.username.is_empty()).then_some(identity.username));
            Ok(())
        } else {
            Err(client::Error::AuthenticationUnsupported)
        }
    }

    fn to_url(&self) -> Cow<'_, BStr> {
        Cow::Owned(self.url.to_bstring())
    }

    fn connection_persists_across_multiple_requests(&self) -> bool {
        true
    }

    fn configure(&mut self, _config: &dyn Any) -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
        Ok(())
    }
}

impl Drop for SpawnProcessOnDemand {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // The child process (e.g. `ssh`) may still be running at this point, so kill it before joining/waiting.
            // In the happy-path case, it should have already exited gracefully, but in error cases or if the user
            // interrupted the operation, it will likely still be running.
            child.kill().ok();
            child.wait().ok();
        }
    }
}

struct ReadStdoutFailOnError {
    recv: std::sync::mpsc::Receiver<std::io::Error>,
    read: std::process::ChildStdout,
}

impl std::io::Read for ReadStdoutFailOnError {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let res = self.read.read(buf);
        self.swap_err_if_present_in_stderr(buf.len(), res)
    }
}

impl ReadStdoutFailOnError {
    fn swap_err_if_present_in_stderr(&self, wanted: usize, res: std::io::Result<usize>) -> std::io::Result<usize> {
        match self.recv.try_recv().ok() {
            Some(err) => Err(err),
            None => match res {
                Ok(n) if n == wanted => Ok(n),
                Ok(n) => {
                    // TODO: fix this
                    // When parsing refs this seems to happen legitimately
                    // (even though we read packet lines only and should always know exactly how much to read)
                    // Maybe this still happens in `read_exact()` as sometimes we just don't get enough bytes
                    // despite knowing how many.
                    // To prevent deadlock, we have to set a timeout which slows down legitimate parts of the protocol.
                    // This code was specifically written to make the `cargo` test-suite pass, and we can reduce
                    // the timeouts even more once there is a native ssh transport that is used by `cargo`, it will
                    // be able to handle these properly.
                    // Alternatively, one could implement something like `read2` to avoid blocking on stderr entirely.
                    self.recv
                        .recv_timeout(std::time::Duration::from_millis(5))
                        .ok()
                        .map_or(Ok(n), Err)
                }
                Err(err) => Err(self.recv.recv().ok().unwrap_or(err)),
            },
        }
    }
}

fn supervise_stderr(
    ssh_kind: ssh::ProgramKind,
    stderr: std::process::ChildStderr,
    stdout: std::process::ChildStdout,
) -> ReadStdoutFailOnError {
    let (send, recv) = std::sync::mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("supervise ssh stderr".into())
        .stack_size(128 * 1024)
        .spawn(move || -> std::io::Result<()> {
            let mut process_stderr = std::io::stderr();
            for line in std::io::BufReader::new(stderr).byte_lines() {
                let line = line?;
                match ssh_kind.line_to_err(line.into()) {
                    Ok(err) => {
                        send.send(err).ok();
                    }
                    Err(line) => {
                        process_stderr.write_all(&line).ok();
                        writeln!(&process_stderr).ok();
                    }
                }
            }
            Ok(())
        })
        .expect("named threads with small stack work on all platforms");
    ReadStdoutFailOnError { read: stdout, recv }
}

impl client::blocking_io::Transport for SpawnProcessOnDemand {
    fn handshake<'a>(
        &mut self,
        service: Service,
        extra_parameters: &'a [(&'a str, Option<&'a str>)],
    ) -> Result<SetServiceResponse<'_>, client::Error> {
        let (mut cmd, ssh_kind, cmd_name) = match &self.ssh_cmd {
            Some((command, kind)) => (
                kind.prepare_invocation(command, &self.url, self.desired_version, self.ssh_disallow_shell)
                    .map_err(client::Error::SshInvocation)?
                    .stderr(Stdio::piped()),
                Some(*kind),
                Cow::Owned(command.to_owned()),
            ),
            None => (
                gix_command::prepare(service.as_str()).stderr(Stdio::null()),
                None,
                Cow::Borrowed(OsStr::new(service.as_str())),
            ),
        };
        cmd.stdin = Stdio::piped();
        cmd.stdout = Stdio::piped();
        if self.path.trim().first() == Some(&b'-') {
            return Err(client::Error::AmbiguousPath {
                path: self.path.clone(),
            });
        }
        let repo_path = if self.ssh_cmd.is_some() {
            cmd.args.push(service.as_str().into());
            gix_quote::single(self.path.as_ref()).to_os_str_lossy().into_owned()
        } else {
            self.path.to_os_str_lossy().into_owned()
        };
        cmd.args.push(repo_path);

        let mut cmd = std::process::Command::from(cmd);
        for env_to_remove in ENV_VARS_TO_REMOVE {
            cmd.env_remove(env_to_remove);
        }
        cmd.envs(std::mem::take(&mut self.envs));

        gix_features::trace::debug!(command = ?cmd, "gix_transport::SpawnProcessOnDemand");
        let mut child = cmd.spawn().map_err(|err| client::Error::InvokeProgram {
            source: err,
            command: cmd_name.into_owned(),
        })?;
        let stdout: Box<dyn std::io::Read + Send> = match ssh_kind {
            Some(ssh_kind) => Box::new(supervise_stderr(
                ssh_kind,
                child.stderr.take().expect("configured beforehand"),
                child.stdout.take().expect("configured"),
            )),
            None => Box::new(child.stdout.take().expect("stdout configured")),
        };
        self.connection = Some(Connection::new_for_spawned_process(
            stdout,
            child.stdin.take().expect("stdin configured"),
            self.desired_version,
            self.path.clone(),
            self.trace,
        ));
        self.child = Some(child);
        self.connection
            .as_mut()
            .expect("connection to be there right after setting it")
            .handshake(service, extra_parameters)
    }

    fn request(
        &mut self,
        write_mode: WriteMode,
        on_into_read: MessageKind,
        trace: bool,
    ) -> Result<RequestWriter<'_>, client::Error> {
        self.connection
            .as_mut()
            .ok_or(client::Error::MissingHandshake)?
            .request(write_mode, on_into_read, trace)
    }
}

/// Connect to a locally readable repository at `path` using the given `desired_version`.
/// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
///
/// This will spawn a `git` process locally.
pub fn connect(
    path: impl Into<BString>,
    desired_version: Protocol,
    trace: bool,
) -> Result<SpawnProcessOnDemand, std::convert::Infallible> {
    Ok(SpawnProcessOnDemand::new_local(path.into(), desired_version, trace))
}

#[cfg(test)]
mod tests {
    mod ssh {
        mod connect {
            use crate::{client::blocking_io::ssh, Protocol};

            #[test]
            fn path() {
                for (url, expected) in [
                    ("ssh://host.xy/~/repo", "~/repo"),
                    ("ssh://host.xy/~username/repo", "~username/repo"),
                    ("user@host.xy:/username/repo", "/username/repo"),
                    ("user@host.xy:username/repo", "username/repo"),
                    ("user@host.xy:../username/repo", "../username/repo"),
                    ("user@host.xy:~/repo", "~/repo"),
                ] {
                    let url = gix_url::parse((*url).into()).expect("valid url");
                    let cmd = ssh::connect(url, Protocol::V1, Default::default(), false).expect("parse success");
                    assert_eq!(cmd.path, expected, "the path will be substituted by the remote shell");
                }
            }

            #[test]
            fn ambiguous_host_disallowed() {
                for url in [
                    "ssh://-oProxyCommand=open$IFS-aCalculator/foo",
                    "user@-oProxyCommand=open$IFS-aCalculator:username/repo",
                ] {
                    let url = gix_url::parse((*url).into()).expect("valid url");
                    let options = ssh::connect::Options {
                        command: Some("unrecognized".into()),
                        disallow_shell: false,
                        kind: None,
                    };
                    assert!(matches!(
                        ssh::connect(url, Protocol::V1, options, false),
                        Err(ssh::Error::AmbiguousHostName { host }) if host == "-oProxyCommand=open$IFS-aCalculator",
                    ));
                }
            }
        }
    }
}
