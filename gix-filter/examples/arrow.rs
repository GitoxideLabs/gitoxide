use std::{
    io::{Read, Write, stdin, stdout},
    time::Duration,
};

use bstr::{ByteSlice, ByteVec};
use gix_filter::driver::process;

static PREFIX: &str = "➡";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args();
    let sub_command = args.nth(1).ok_or("Need sub-command")?;
    let next_arg = args.next(); // possibly %f
    let needs_failure = next_arg.as_deref().is_some_and(|file| file.ends_with("fail"));
    if needs_failure {
        panic!("failure requested for {sub_command}");
    }

    match sub_command.as_str() {
        "process" => {
            let disallow_delay = next_arg.as_deref() == Some("disallow-delay");
            let mut srv = gix_filter::driver::process::Server::handshake(
                stdin(),
                stdout(),
                "git-filter",
                &mut |versions| versions.contains(&2).then_some(2),
                if disallow_delay {
                    &["clean", "smudge"]
                } else {
                    &["clean", "smudge", "delay"]
                },
            )?;

            let mut next_smudge_aborts = false;
            let mut next_smudge_fails_permanently = false; // a test validates that we don't actually hang
            let mut delayed = Vec::new();
            while let Some(mut request) = srv.next_request()? {
                let needs_failure = request
                    .meta
                    .iter()
                    .find_map(|(key, value)| (key == "pathname").then_some(value))
                    .is_some_and(|path| path.ends_with(b"fail"));
                let pathname = request
                    .meta
                    .iter()
                    .find_map(|(key, value)| (key == "pathname").then(|| value.clone()));
                if needs_failure {
                    panic!("process failure requested: {:?}", request.meta);
                }
                let can_delay = request
                    .meta
                    .iter()
                    .any(|(key, value)| key == "can-delay" && value == "1");
                match request.command.as_str() {
                    "clean" => {
                        let mut buf = Vec::new();
                        request.as_read().read_to_end(&mut buf)?;
                        request.write_status(if can_delay {
                            process::Status::delayed()
                        } else {
                            process::Status::success()
                        })?;

                        let lines = if let Some(delayed_lines) = buf
                            .is_empty()
                            .then(|| {
                                delayed
                                    .iter()
                                    .position(|(cmd, path, _)| {
                                        *cmd == request.command.as_str() && Some(path) == pathname.as_ref()
                                    })
                                    .map(|pos| delayed.remove(pos).2)
                            })
                            .flatten()
                        {
                            delayed_lines
                        } else {
                            let mut lines = Vec::new();
                            for mut line in buf.lines_with_terminator() {
                                if line.starts_with(PREFIX.as_bytes()) {
                                    line = &line[PREFIX.len()..];
                                }
                                lines.push_str(line);
                            }
                            lines
                        };
                        if can_delay {
                            delayed.push(("clean", pathname.expect("needed for delayed operation"), lines));
                        } else {
                            request.as_write().write_all(&lines)?;
                            request.write_status(process::Status::Previous)?;
                        }
                    }
                    "smudge" => {
                        let mut buf = Vec::new();
                        request.as_read().read_to_end(&mut buf)?;
                        let status = if next_smudge_aborts {
                            next_smudge_aborts = false;
                            process::Status::abort()
                        } else if next_smudge_fails_permanently {
                            process::Status::exit()
                        } else if can_delay {
                            process::Status::delayed()
                        } else {
                            process::Status::success()
                        };
                        request.write_status(status)?;

                        let lines = if let Some(delayed_lines) = buf
                            .is_empty()
                            .then(|| {
                                delayed
                                    .iter()
                                    .position(|(cmd, path, _)| {
                                        *cmd == request.command.as_str() && Some(path) == pathname.as_ref()
                                    })
                                    .map(|pos| delayed.remove(pos).2)
                            })
                            .flatten()
                        {
                            delayed_lines
                        } else {
                            let mut lines = Vec::new();
                            for line in buf.lines_with_terminator() {
                                if !line.starts_with(PREFIX.as_bytes()) {
                                    lines.push_str(PREFIX.as_bytes());
                                }
                                lines.push_str(line);
                            }
                            lines
                        };

                        if can_delay {
                            delayed.push(("smudge", pathname.expect("needed for delayed operation"), lines));
                        } else {
                            request.as_write().write_all(&lines)?;
                            request.write_status(process::Status::Previous)?;
                        }
                    }
                    "list_available_blobs" => {
                        {
                            let mut out = request.as_write();
                            let mut last_cmd = None;
                            let mut buf = Vec::<u8>::new();
                            for (cmd, path, _) in &delayed {
                                if last_cmd.get_or_insert(*cmd) != cmd {
                                    panic!("the API doesn't support mixing cmds as paths might not be unique anymore")
                                }
                                buf.clear();
                                buf.push_str("pathname=");
                                buf.extend_from_slice(path);
                                out.write_all(&buf)?;
                            }
                        }
                        request.write_status(process::Status::success())?;
                    }
                    "wait-1-s" => {
                        std::io::copy(&mut request.as_read(), &mut std::io::sink())?;
                        request.write_status(process::Status::success())?;
                        std::thread::sleep(Duration::from_secs(1));
                    }
                    "next-smudge-aborts" => {
                        std::io::copy(&mut request.as_read(), &mut std::io::sink())?;
                        request.write_status(process::Status::success())?;
                        next_smudge_aborts = true;
                    }
                    "next-invocation-returns-strange-status-and-smudge-fails-permanently" => {
                        std::io::copy(&mut request.as_read(), &mut std::io::sink())?;
                        request.write_status(process::Status::success())?;
                        next_smudge_fails_permanently = true;
                    }
                    unknown => panic!("Unknown capability requested: {unknown}"),
                }
            }
        }
        // simple filters actually don't support streaming - they have to first read all input, then produce all output,
        // but can't mix reading stdin and write to stdout at the same time as `git` (or `gitoxide`) don't read the output while
        // writing the input.
        "clean" => {
            let mut stdin = stdin().lock();
            let mut stdout = stdout().lock();
            let mut buf = Vec::new();
            std::io::copy(&mut stdin, &mut buf)?;
            for mut line in buf.lines_with_terminator() {
                if line.starts_with(PREFIX.as_bytes()) {
                    line = &line[PREFIX.len()..];
                }
                stdout.write_all(line).map(|_| true)?;
            }
        }
        "smudge" => {
            let mut stdin = stdin().lock();
            let mut stdout = stdout().lock();
            let mut buf = Vec::new();
            std::io::copy(&mut stdin, &mut buf)?;
            for line in buf.lines_with_terminator() {
                if !line.starts_with(PREFIX.as_bytes()) {
                    stdout.write_all(PREFIX.as_bytes())?;
                }
                stdout.write_all(line).map(|_| true)?;
            }
        }
        unknown => panic!("Unknown sub-command: {unknown}"),
    }
    Ok(())
}
