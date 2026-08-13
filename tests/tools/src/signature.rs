//! Test support for Git signing without using the user's identities or configuration.
//!
//! The bundled, passwordless SSH, OpenPGP, and X.509 identities are copied or imported
//! into disposable directories with suitably restrictive permissions. Callers are
//! responsible for checking that the required signing [`crate::signature::program_available()`] and must keep the
//! returned [`tempfile::TempDir`](crate::tempfile::TempDir) alive while using it.
//!
//! These public test identities provide no security and must never be used outside tests.

use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::Result;

/// The identity associated with all signing fixtures.
pub const IDENTITY: &str = "signing@example.com";

/// Return the path to a signing fixture named `name`.
pub fn fixture(name: impl AsRef<Path>) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/signature/fixtures")
        .join(name)
}

/// Return `path` in a form understood by Unix-derived programs on Windows.
///
/// Git for Windows commonly provides GnuPG and OpenSSH programs which interpret backslashes as
/// escapes instead of path separators. Native programs accept the resulting forward slashes as well.
pub fn path_for_command(path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    #[cfg(windows)]
    {
        PathBuf::from(
            path.to_str()
                .expect("signing test fixture paths must be valid UTF-8")
                .replace('\\', "/"),
        )
    }
    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

/// Return whether signing `program` can be launched.
pub fn program_available(program: &str) -> bool {
    Command::new(program)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

/// Create an isolated signer home with suitably restrictive permissions.
pub fn isolated_home() -> Result<crate::tempfile::TempDir> {
    let home = crate::tempfile::TempDir::new()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(home.path(), std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(home)
}

/// Copy the passwordless SSH signing key to a temporary file with suitably restrictive permissions.
pub fn ssh_private_key() -> Result<(crate::tempfile::TempDir, PathBuf)> {
    let home = crate::tempfile::TempDir::new()?;
    let key = home.path().join("key");
    std::fs::copy(fixture("ssh-private"), &key)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok((home, key))
}

/// Import the passwordless OpenPGP signing identity into a temporary home.
///
/// Passing this directory to `gpg --homedir` keeps the test's keys, configuration,
/// and trust state separate from the user's GnuPG home and removes them with the directory.
pub fn openpgp_home() -> Result<crate::tempfile::TempDir> {
    let home = isolated_home()?;
    run(Command::new("gpg")
        .args(["--batch", "--homedir"])
        .arg(path_for_command(home.path()))
        .args(["--import"])
        .arg(path_for_command(fixture("openpgp-secret.asc"))))?;
    Ok(home)
}

/// Import and trust the passwordless X.509 signing identity in a temporary home.
///
/// Passing this directory to `gpgsm --homedir` keeps the test's keys, configuration,
/// and trust list separate from the user's GnuPG home and removes them with the directory.
pub fn x509_home() -> Result<crate::tempfile::TempDir> {
    let home = isolated_home()?;
    run(Command::new("gpgsm")
        .args([
            "--batch",
            "--pinentry-mode",
            "loopback",
            "--passphrase",
            "",
            "--homedir",
        ])
        .arg(path_for_command(home.path()))
        .arg("--import")
        .arg(path_for_command(fixture("x509-identity.p12"))))?;
    let keys = Command::new("gpgsm")
        .args(["--batch", "--homedir"])
        .arg(path_for_command(home.path()))
        .args(["-K", "--with-colons"])
        .output()?;
    assert!(keys.status.success(), "the imported X.509 key can be listed");
    let keys = String::from_utf8_lossy(&keys.stdout);
    let fingerprint = keys
        .lines()
        .find_map(|line| {
            line.strip_prefix("fpr:::::::::")
                .and_then(|line| line.split(':').next())
        })
        .expect("gpgsm reports a fingerprint for the imported key");
    std::fs::write(home.path().join("trustlist.txt"), format!("{fingerprint} S relax\n"))?;
    let _ = Command::new("gpgconf")
        .args(["--homedir"])
        .arg(path_for_command(home.path()))
        .args(["--reload", "all"])
        .status();
    Ok(home)
}

fn run(command: &mut Command) -> Result {
    let output = command.output()?;
    assert!(
        output.status.success(),
        "signature fixture setup succeeds: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}
