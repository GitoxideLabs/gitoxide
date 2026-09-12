use std::{fs, io, io::prelude::*, path::PathBuf};

fn bash_program() -> io::Result<()> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        eprintln!("warning: `bash-program` subcommand not meant for scripting, format may change");
    }
    println!("{}", gix_testtools::bash_program().display());
    Ok(())
}

fn mess_in_the_middle(path: PathBuf) -> io::Result<()> {
    let mut file = fs::OpenOptions::new().read(false).write(true).open(path)?;
    file.seek(io::SeekFrom::Start(file.metadata()?.len() / 2))?;
    file.write_all(b"hello")?;
    Ok(())
}

#[cfg(unix)]
fn umask() -> io::Result<()> {
    println!("{:04o}", gix_testtools::umask());
    Ok(())
}

/// Run a Git protocol test daemon on an OS-assigned loopback port.
/// This function blocks and the process needs to be killed.
///
/// Journey tests use this instead of `git daemon --port=<n>` because Git
/// treats `--port=0` as "use the default port", so it can't bind an
/// ephemeral port and report it back. This wrapper owns the listening socket,
/// writes the resulting `git://127.0.0.1:<port>/` URL to `url_file`, and then
/// hands every accepted connection to `git daemon --inetd`.
#[cfg(unix)]
fn git_daemon(url_file: PathBuf) -> io::Result<()> {
    let daemon = gix_testtools::spawn_git_daemon(".")?;
    fs::write(url_file, format!("{}/\n", daemon.url))?;
    loop {
        std::thread::park();
    }
}

#[cfg(not(unix))]
fn git_daemon(_url_file: PathBuf) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "`jtt git-daemon` is only supported on Unix",
    ))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let scmd = args.next().expect("sub command");
    match scmd.to_str().ok_or("subcommand is not UTF-8")? {
        "run" => {
            let mut cmd = std::process::Command::new(args.next().ok_or("run requires a program")?);
            let config_dir = gix_testtools::tempfile::TempDir::new()?;
            let status = gix_testtools::configure_git_environment(&mut cmd, config_dir.path())
                .args(args)
                .status()?;
            drop(config_dir);
            std::process::exit(status.code().unwrap_or(1));
        }
        "bash-program" | "bp" => bash_program()?,
        "git-daemon" => git_daemon(PathBuf::from(args.next().expect("path to write the git:// URL to")))?,
        "mess-in-the-middle" => mess_in_the_middle(PathBuf::from(args.next().expect("path to file to mess with")))?,
        #[cfg(unix)]
        "umask" => umask()?,
        _ => unreachable!("Unknown subcommand: {}", scmd.to_string_lossy()),
    }
    Ok(())
}
