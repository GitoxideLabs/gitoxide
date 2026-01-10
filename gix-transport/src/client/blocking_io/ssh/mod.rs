use std::{
    ffi::OsStr,
    process::{Command, Stdio},
};

use gix_url::{ArgumentSafety::*, Url};

use crate::{client::blocking_io::file::SpawnProcessOnDemand, Protocol};

/// The error used in [`connect()`].
pub type Error = crate::Error;

/// The kind of SSH programs we have built-in support for.
///
/// Various different programs exists with different capabilities, and we have a few built in.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramKind {
    /// The standard linux ssh program
    Ssh,
    /// The `(plink|putty).exe` binaries, typically only on windows.
    Plink,
    /// The `putty.exe` binary, typically only on windows.
    Putty,
    /// The `tortoiseplink.exe` binary, only on windows.
    TortoisePlink,
    /// A minimal ssh client that supports on options.
    Simple,
}

mod program_kind;

///
pub mod invocation {
    /// The error returned when producing ssh invocation arguments based on a selected invocation kind.
    pub type Error = crate::Error;
}

///
pub mod connect {
    use std::ffi::{OsStr, OsString};

    use crate::client::blocking_io::ssh::ProgramKind;

    /// The options for use when [connecting][super::connect()] via the `ssh` protocol.
    #[derive(Debug, Clone, Default)]
    pub struct Options {
        /// The program or script to use.
        /// If unset, it defaults to `ssh` or `ssh.exe`, or the program implied by `kind` if that one is set.
        pub command: Option<OsString>,
        /// If `true`, a shell must not be used to execute `command`.
        /// This defaults to `false`, and a shell can then be used if `command` seems to require it, but won't be
        /// used unnecessarily.
        pub disallow_shell: bool,
        /// The ssh variant further identifying `program`. This determines which arguments will be used
        /// when invoking the program.
        /// If unset, the `program` basename determines the variant, or an invocation of the `command` itself.
        pub kind: Option<ProgramKind>,
    }

    impl Options {
        /// Return the configured ssh command, defaulting to `ssh` if neither the `command` nor the `kind` fields are set.
        pub fn ssh_command(&self) -> &OsStr {
            self.command
                .as_deref()
                .or_else(|| self.kind.and_then(|kind| kind.exe()))
                .unwrap_or_else(|| OsStr::new("ssh"))
        }
    }
}

/// Connect to `host` using the ssh program to obtain data from the repository at `path` on the remote.
///
/// The optional `user` identifies the user's account to which to connect, while `port` allows to specify non-standard
/// ssh ports.
///
/// The `desired_version` is the preferred protocol version when establishing the connection, but note that it can be
/// downgraded by servers not supporting it.
/// If `trace` is `true`, all packetlines received or sent will be passed to the facilities of the `gix-trace` crate.
#[allow(clippy::result_large_err)]
pub fn connect(
    url: Url,
    desired_version: Protocol,
    options: connect::Options,
    trace: bool,
) -> Result<SpawnProcessOnDemand, Error> {
    if url.scheme != gix_url::Scheme::Ssh || url.host().is_none() {
        return Err(Error::UnsupportedScheme(url.scheme));
    }
    let ssh_cmd = options.ssh_command();
    let kind = determine_client_kind(options.kind, ssh_cmd, &url, options.disallow_shell)?;
    let path = gix_url::expand_path::for_shell(url.path.clone());
    Ok(SpawnProcessOnDemand::new_ssh(
        url,
        ssh_cmd,
        path,
        kind,
        options.disallow_shell,
        desired_version,
        trace,
    ))
}

#[allow(clippy::result_large_err)]
fn determine_client_kind(
    known_kind: Option<ProgramKind>,
    ssh_cmd: &OsStr,
    url: &Url,
    disallow_shell: bool,
) -> Result<ProgramKind, Error> {
    let mut kind = known_kind.unwrap_or_else(|| ProgramKind::from(ssh_cmd));
    if known_kind.is_none() && kind == ProgramKind::Simple {
        let mut cmd = build_client_feature_check_command(ssh_cmd, url, disallow_shell)?;
        gix_features::trace::debug!(cmd = ?cmd, "invoking `ssh` for feature check");
        kind = if cmd.status().ok().is_some_and(|status| status.success()) {
            ProgramKind::Ssh
        } else {
            ProgramKind::Simple
        };
    }
    Ok(kind)
}

#[allow(clippy::result_large_err)]
fn build_client_feature_check_command(ssh_cmd: &OsStr, url: &Url, disallow_shell: bool) -> Result<Command, Error> {
    let mut prepare = gix_command::prepare(ssh_cmd)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .stdin(Stdio::null())
        .command_may_be_shell_script()
        .arg("-G")
        .arg(match url.host_as_argument() {
            Usable(host) => host,
            Dangerous(host) => Err(Error::AmbiguousHostName { host: host.into() })?,
            Absent => panic!("BUG: host should always be present in SSH URLs"),
        });
    if disallow_shell {
        prepare.use_shell = false;
    }
    Ok(prepare.into())
}

#[cfg(test)]
mod tests;
