use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};

use bstr::ByteSlice;
use gix_error::{ErrorExt, Result, ResultExt, bail, message, validation};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_encode};

use crate::{Program, helper::Action, program::Kind, protocol::Context};

const COMPONENT: &AsciiSet = &NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');
const PATH_COMPONENT: &AsciiSet = &COMPONENT
    .remove(b'!')
    .remove(b'*')
    .remove(b'\'')
    .remove(b'(')
    .remove(b')')
    .remove(b';')
    .remove(b':')
    .remove(b'@')
    .remove(b'&')
    .remove(b'=')
    .remove(b'+')
    .remove(b'$')
    .remove(b',')
    .remove(b'/')
    .remove(b'?')
    .remove(b'#')
    .remove(b'[')
    .remove(b']');

pub(crate) fn invoke(
    helper: &Program,
    action: &Action,
    environment: &[(std::ffi::OsString, std::ffi::OsString)],
) -> Option<Result<Option<Vec<u8>>>> {
    let Kind::ExternalName { name_and_args } = &helper.kind else {
        return None;
    };
    if !super::builtin::is_static(name_and_args) {
        return None;
    }
    let parsed = gix_command::parse::command_line(name_and_args.as_bstr()).ok()?;
    if !parsed.env.is_empty() || !(parsed.command == "store" || cfg!(windows) && parsed.command == "store.exe") {
        return None;
    }
    Some((|| {
        let mut file = None;
        let mut args = parsed.args.iter();
        while let Some(arg) = args.next() {
            if arg == "--file" {
                file =
                    Some(PathBuf::from(args.next().ok_or_else(|| {
                        validation("Credential store --file needs a path").raise()
                    })?));
            } else if let Some(path) = gix_path::os_str_into_bstr(arg)?.strip_prefix(b"--file=") {
                file = Some(gix_path::from_bstr(path.as_bstr())?.into_owned());
            } else if arg == "--no-file" {
                file = None;
            } else if arg != "--" || args.len() != 0 {
                bail!(validation("Unsupported credential store option"));
            }
        }
        let files = match file {
            Some(file) => vec![file],
            None => {
                let env_var = |name: &str| {
                    environment
                        .iter()
                        .rev()
                        .find(|(key, _)| key == name)
                        .map(|(_, value)| value.clone())
                        .or_else(|| gix_path::env::var(name))
                };
                let mut files = Vec::new();
                if let Some(home) = env_var("HOME") {
                    files.push(PathBuf::from(home).join(".git-credentials"));
                }
                if let Some(xdg) = gix_path::env::xdg_config("credentials", &mut |name| {
                    env_var(name).filter(|value| name == "HOME" || !value.is_empty())
                }) {
                    files.push(xdg);
                }
                if files.is_empty() {
                    bail!(validation("Credential store needs a home directory or --file"));
                }
                files
            }
        };
        let (context, has_credential) = input_context(action)?;
        match action {
            Action::Get(_) => {
                for file in &files {
                    for line in lines(file, true)?.into_iter().flatten() {
                        let line = line.or_raise(|| message("Could not read credential store"))?;
                        if let Some(entry) = parse_url(&line).filter(|entry| matches(&context, entry, false)) {
                            let mut output = Vec::new();
                            Context {
                                username: entry.username,
                                password: entry.password,
                                options: context.options,
                                ..Default::default()
                            }
                            .write_to(&mut output)
                            .map_err(|_| {
                                validation("Stored credential cannot be represented by the helper protocol").raise()
                            })?;
                            return Ok(Some(output));
                        }
                    }
                }
                Ok(Some(Vec::new()))
            }
            Action::Store(_) => {
                if context.protocol.is_some()
                    && (context.host.is_some() || context.path.is_some())
                    && context.username.is_some()
                    && context.password.is_some()
                {
                    let file = files.iter().find(|path| path.exists()).unwrap_or(&files[0]);
                    rewrite(
                        file,
                        Some(&context),
                        Some(encode_url(&context)),
                        false,
                        helper.store_lock_timeout,
                    )?;
                }
                Ok(None)
            }
            Action::Erase(_) => {
                if context.protocol.is_some()
                    || context.host.is_some()
                    || context.path.is_some()
                    || context.username.is_some()
                {
                    for file in &files {
                        if file.exists() {
                            // This file format never stores opaque tokens, so token-qualified erase matches nothing.
                            rewrite(
                                file,
                                (!has_credential).then_some(&context),
                                None,
                                true,
                                helper.store_lock_timeout,
                            )?;
                        }
                    }
                }
                Ok(None)
            }
        }
    })())
}

fn input_context(action: &Action) -> Result<(Context, bool)> {
    // Store/erase payloads have already been encoded under the caller's protocol policy.
    let options = action.context().map_or(
        crate::protocol::ContextOptions {
            protect_protocol: false,
        },
        |ctx| ctx.options,
    );
    let mut input = Vec::new();
    action
        .send(&mut input)
        .map_err(|_| validation("Invalid credential store input").raise())?;
    Context::from_bytes(&input, options).map_err(|_| validation("Invalid credential store input").raise())?;
    let mut context = Context {
        options,
        ..Default::default()
    };
    let mut has_credential = false;
    let mut tail = input.as_slice();
    let mut offset = 0;
    for line in input.split_inclusive(|byte| *byte == b'\n') {
        offset += line.len();
        let line = line.strip_suffix(b"\n").unwrap_or(line);
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.is_empty() {
            break;
        }
        if let Some(url) = line.strip_prefix(b"url=") {
            context = parse_url(url).ok_or_else(|| validation("Invalid credential store URL").raise())?;
            context.options = options;
            tail = &input[offset..];
            has_credential = false;
        } else if line.starts_with(b"credential=") {
            has_credential = true;
        }
    }
    let fields =
        Context::from_bytes(tail, options).map_err(|_| validation("Invalid credential store input").raise())?;
    for (target, value) in [
        (&mut context.protocol, fields.protocol),
        (&mut context.host, fields.host),
        (&mut context.username, fields.username),
        (&mut context.password, fields.password),
    ] {
        if value.is_some() {
            *target = value;
        }
    }
    if fields.path.is_some() {
        context.path = fields.path;
    }
    context
        .write_to(std::io::sink())
        .map_err(|_| validation("Invalid credential store input").raise())?;
    Ok((context, has_credential))
}

fn parse_url(input: &[u8]) -> Option<Context> {
    let parsed = super::builtin::parse_url(input)?;
    Some(Context {
        protocol: Some(String::from_utf8(parsed.protocol).ok()?),
        host: Some(String::from_utf8(parsed.host).ok()?),
        username: parsed.username.map(String::from_utf8).transpose().ok()?,
        password: parsed.password.map(String::from_utf8).transpose().ok()?,
        path: parsed.path.map(Into::into),
        ..Default::default()
    })
}

fn matches(query: &Context, entry: &Context, password: bool) -> bool {
    entry.username.is_some()
        && entry.password.is_some()
        && [
            (&query.protocol, &entry.protocol),
            (&query.host, &entry.host),
            (&query.username, &entry.username),
        ]
        .iter()
        .all(|(query, entry)| query.as_ref().is_none_or(|query| Some(query) == entry.as_ref()))
        && query.path.as_ref().is_none_or(|path| Some(path) == entry.path.as_ref())
        && (!password
            || query
                .password
                .as_ref()
                .is_none_or(|password| Some(password) == entry.password.as_ref()))
}

fn encode_url(context: &Context) -> Vec<u8> {
    let mut output = format!(
        "{}://{}:{}@",
        context.protocol.as_deref().expect("store requires protocol"),
        encoded(
            context.username.as_deref().expect("store requires username").as_bytes(),
            COMPONENT
        ),
        encoded(
            context.password.as_deref().expect("store requires password").as_bytes(),
            COMPONENT
        )
    );
    if let Some(host) = &context.host {
        output.push_str(&encoded(host.as_bytes(), COMPONENT));
    }
    if let Some(path) = &context.path {
        output.push('/');
        output.push_str(&encoded(path, PATH_COMPONENT));
    }
    output.into_bytes()
}

fn encoded(value: &[u8], set: &'static AsciiSet) -> String {
    percent_encode(value, set)
        .map(|part| {
            if part.starts_with('%') {
                part.to_ascii_lowercase()
            } else {
                part.to_owned()
            }
        })
        .collect()
}

fn lines(path: &Path, ignore_unreadable: bool) -> Result<Option<std::io::Split<BufReader<std::fs::File>>>> {
    match std::fs::File::open(path) {
        Ok(file) => Ok(Some(BufReader::new(file).split(b'\n'))),
        Err(err)
            if err.kind() == std::io::ErrorKind::NotFound
                || ignore_unreadable && err.kind() == std::io::ErrorKind::PermissionDenied =>
        {
            Ok(None)
        }
        Err(err) => Err(err).or_raise(|| message("Could not open credential store")),
    }
}

fn rewrite(
    path: &Path,
    context: Option<&Context>,
    extra: Option<Vec<u8>>,
    password: bool,
    timeout: gix_lock::acquire::Fail,
) -> Result {
    let mut lock = gix_lock::File::acquire(path, timeout, None, -0o600, Some(&gix_lock::acquire::resolve_symlink))
        .or_raise(|| message("Could not lock credential store"))?;
    let write_error = || message("Could not write credential store");
    if let Some(extra) = extra {
        lock.write_all(&extra).or_raise(write_error)?;
        lock.write_all(b"\n").or_raise(write_error)?;
    }
    for line in lines(&lock.resource_path(), false)?.into_iter().flatten() {
        let line = line.or_raise(|| message("Could not read credential store"))?;
        if !context.is_some_and(|context| parse_url(&line).is_some_and(|entry| matches(context, &entry, password))) {
            lock.write_all(&line).or_raise(write_error)?;
            lock.write_all(b"\n").or_raise(write_error)?;
        }
    }
    lock.commit()
        .map_err(|err| err.error)
        .or_raise(|| message("Could not commit credential store"))?;
    Ok(())
}
