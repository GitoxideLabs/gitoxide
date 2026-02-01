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

/// Run a fixture script and print the path to the resulting read-only directory.
/// The directory is cached and reused across invocations.
/// `cwd` - the directory to change to before running the script (should be repository root)
/// `script_path` - path relative to tests/fixtures/
fn run_script(cwd: PathBuf, script_path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::env::set_current_dir(&cwd)?;
    let fixture_path = gix_testtools::scripted_fixture_read_only_with_args(script_path, None::<String>)?;
    // Return absolute path since caller may be in a different directory
    println!("{}", cwd.join(fixture_path).display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut args = std::env::args().skip(1);
    let scmd = args.next().expect("sub command");
    match &*scmd {
        "bash-program" | "bp" => bash_program()?,
        "mess-in-the-middle" => mess_in_the_middle(PathBuf::from(args.next().expect("path to file to mess with")))?,
        "run-script" => {
            let cwd = PathBuf::from(args.next().expect("working directory (repository root)"));
            let script = PathBuf::from(args.next().expect("path to script relative to tests/fixtures/"));
            run_script(cwd, script)?;
        }
        #[cfg(unix)]
        "umask" => umask()?,
        _ => unreachable!("Unknown subcommand: {}", scmd),
    }
    Ok(())
}
