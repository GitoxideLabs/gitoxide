use std::{
    io::{Read, Write},
    sync::{Arc, atomic::AtomicBool},
};

use gix::{
    Result,
    bstr::BString,
    credentials::program::cache::{self, DaemonCommand},
    error::ResultExt,
};

use super::options::credential::{Cache, CacheDaemon};

pub(super) fn launcher(config: &[BString]) -> Result<DaemonCommand> {
    let mut args = Vec::new();
    for value in config {
        args.push("-c".into());
        args.push(gix::path::from_bstr(value)?.into_owned().into_os_string());
    }
    args.push("credential-cache--daemon".into());
    Ok(DaemonCommand {
        executable: std::env::current_exe().or_error()?,
        args,
    })
}

pub(super) fn client(options: Cache, launcher: &DaemonCommand) -> Result<()> {
    let socket = match options.socket {
        Some(socket) => socket,
        None => cache::default_socket()?,
    };
    let mut input = Vec::new();
    if matches!(options.action.as_str(), "get" | "store" | "erase") {
        std::io::stdin().read_to_end(&mut input).or_error()?;
    }
    let response = cache::request(&socket, &options.action, options.timeout, &input, Some(launcher))?;
    std::io::stdout().write_all(&response).or_error()
}

pub(super) fn daemon(options: CacheDaemon, config: &gix::config::File) -> Result<()> {
    let ignore_sighup = config.boolean("credentialCache.ignoreSighup")?.unwrap_or(false);
    let interrupt = Arc::new(AtomicBool::new(false));
    // The general CLI handler includes SIGHUP, so install this daemon's policy before readiness.
    let mut signals = Vec::new();
    for signal in signal_hook::consts::TERM_SIGNALS {
        signals.push(signal_hook::flag::register(*signal, Arc::clone(&interrupt)).or_error()?);
    }
    signals.push(
        signal_hook::flag::register(
            signal_hook::consts::SIGHUP,
            if ignore_sighup {
                Arc::new(AtomicBool::new(false))
            } else {
                Arc::clone(&interrupt)
            },
        )
        .or_error()?,
    );
    let result =
        cache::Daemon::bind(&options.socket).and_then(|daemon| daemon.run_with_stdio(options.debug, &interrupt));
    for signal in signals {
        signal_hook::low_level::unregister(signal);
    }
    result
}
