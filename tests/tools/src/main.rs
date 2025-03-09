use std::{fs, io, io::prelude::*, path::PathBuf};

fn bash_program() -> io::Result<()> {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        eprintln!("warning: `bash-program` subcommand not meant for scripting, format may change");
    }
    println!("{:?}", gix_testtools::bash_program());
    Ok(())
}

fn env() -> io::Result<()> {
    fn repr(text: &std::ffi::OsStr) -> String {
        text.to_str()
            .filter(|s| !s.contains('"'))
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("{text:?}"))
    }
    for (name, value) in std::env::vars_os() {
        println!("{}={}", repr(&name), repr(&value));
    }
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let scmd = args.next().expect("sub command");
    match &*scmd {
        "bash-program" | "bp" => bash_program()?,
        "env" => env()?,
        "mess-in-the-middle" => mess_in_the_middle(PathBuf::from(args.next().expect("path to file to mess with")))?,
        #[cfg(unix)]
        "umask" => umask()?,
        _ => unreachable!("Unknown subcommand: {}", scmd),
    }
    Ok(())
}
