use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    os::windows::{ffi::OsStrExt, ffi::OsStringExt, process::CommandExt},
    path::Path,
    process::Command,
};

use gix_error::{Result, bail, validation};

/// Apply Git's extra quoting without probing the executable.
pub(super) fn quote_arguments(
    cmd: Command,
    git_quoting: bool,
    bundled_shell: bool,
    env: &[(OsString, OsString)],
    inline_env: &[(String, OsString)],
) -> Result<Command> {
    let glob = |key| {
        crate::prepare::windows_env_override(key, env, inline_env)
            .map(Cow::Borrowed)
            .or_else(|| std::env::var_os(key).map(Cow::Owned))
            .as_deref()
            .is_none_or(glob_enabled)
    };
    let quote: fn(&OsStr) -> Option<OsString> = if bundled_shell {
        if !cmd.get_args().any(needs_msys_quoting) {
            return Ok(cmd);
        }
        if glob("MSYS") {
            quote_for_msys_glob
        } else {
            quote_for_msys_noglob
        }
    } else if git_quoting {
        let has_double_quote = cmd.get_args().any(|arg| arg.as_encoded_bytes().contains(&b'"'));
        let needs_quoting = cmd.get_args().any(needs_msvc_quoting);
        if !(has_double_quote || needs_quoting) || may_be_batch_file(cmd.get_program()) {
            return Ok(cmd);
        }
        // The noglob parser does not decode the backslash escapes used by the Windows C runtime.
        // Without identifying the executable, reject an argument that could lose its quote boundary.
        if has_double_quote && (!glob("MSYS") || !glob("CYGWIN")) {
            bail!(validation(
                "cannot pass literal double quotes to a Windows Git transport with globbing disabled"
            ));
        }
        if !needs_quoting {
            return Ok(cmd);
        }
        quote_for_msvc
    } else {
        return Ok(cmd);
    };

    let mut quoted = Command::new(cmd.get_program());
    for arg in cmd.get_args() {
        if let Some(arg) = quote(arg) {
            quoted.raw_arg(arg);
        } else {
            quoted.arg(arg);
        }
    }
    Ok(quoted)
}

fn may_be_batch_file(program: &OsStr) -> bool {
    // Rust uses a separate, safer encoder for batch files. In particular, raw_arg would bypass
    // its escaping of percent signs and its checks for line breaks.
    let has_suffix = |name: &OsStr, suffix: &[u8]| {
        let bytes = name.as_encoded_bytes();
        bytes
            .get(bytes.len().saturating_sub(suffix.len())..)
            .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
    };
    !has_suffix(program, b".exe")
        && std::path::absolute(Path::new(program)).map_or(true, |path| {
            has_suffix(path.as_os_str(), b".cmd") || has_suffix(path.as_os_str(), b".bat")
        })
}

fn needs_msvc_quoting(arg: &OsStr) -> bool {
    let bytes = arg.as_encoded_bytes();
    // Spaces and tabs already make Rust wrap the entire argument in double quotes.
    !bytes.iter().any(|byte| matches!(byte, b' ' | b'\t')) && bytes.iter().any(|&byte| requires_quotes(byte))
}

fn needs_msys_quoting(arg: &OsStr) -> bool {
    arg.as_encoded_bytes()
        .iter()
        .any(|&byte| byte == b'\\' || requires_quotes(byte))
}

fn requires_quotes(byte: u8) -> bool {
    // As in Git's quote_arg_msvc(), also quote characters that trigger argument expansion.
    // Include bracket and tilde expansion to preserve the SSH destination literally.
    matches!(
        byte,
        b'\n' | b'\r' | 0x0b | 0x0c | b'"' | b'\'' | b'*' | b'?' | b'[' | b'{' | b'~'
    )
}

/// The last glob, noglob or -glob setting controls Windows argument expansion.
fn glob_enabled(options: &OsStr) -> bool {
    let mut quoted = false;
    let mut glob = true;
    for option in options.as_encoded_bytes().split(|&byte| {
        if byte == b'"' {
            quoted = !quoted;
        }
        !quoted && matches!(byte, b' ' | b'\t')
    }) {
        let split = option
            .iter()
            .position(|&b| b == b'=')
            .or_else(|| option.iter().position(|&b| b == b':'));
        let (name, value) = split.map_or((option, None), |at| (&option[..at], Some(&option[at + 1..])));
        if name.eq_ignore_ascii_case(b"glob") {
            glob = value.is_none_or(|value| value.iter().any(|&byte| byte != b'"'));
        } else if name.eq_ignore_ascii_case(b"noglob") || name.eq_ignore_ascii_case(b"-glob") {
            glob = false;
        }
    }
    glob
}

const DOUBLE: u16 = b'"' as u16;
const SINGLE: u16 = b'\'' as u16;
const SLASH: u16 = b'\\' as u16;

fn quote_for_msvc(arg: &OsStr) -> Option<OsString> {
    if !needs_msvc_quoting(arg) {
        return None;
    }
    let mut quoted = vec![DOUBLE];
    let mut slashes = 0;
    for c in arg.encode_wide() {
        if c == SLASH {
            slashes += 1;
        } else {
            if c == DOUBLE {
                quoted.extend(std::iter::repeat_n(SLASH, slashes + 1));
            }
            slashes = 0;
        }
        quoted.push(c);
    }
    quoted.extend(std::iter::repeat_n(SLASH, slashes));
    quoted.push(DOUBLE);
    Some(OsString::from_wide(&quoted))
}

fn quote_for_msys_glob(arg: &OsStr) -> Option<OsString> {
    if !needs_msys_quoting(arg) {
        return None;
    }
    let mut quoted = vec![DOUBLE];
    for c in arg.encode_wide() {
        if matches!(c, SLASH | DOUBLE) {
            quoted.push(SLASH);
        }
        quoted.push(c);
    }
    quoted.push(DOUBLE);
    Some(OsString::from_wide(&quoted))
}

fn quote_for_msys_noglob(arg: &OsStr) -> Option<OsString> {
    if !needs_msys_quoting(arg) {
        return None;
    }
    let mut quoted = vec![DOUBLE];
    for c in arg.encode_wide() {
        if c == DOUBLE {
            quoted.extend([DOUBLE, SINGLE, DOUBLE, SINGLE, DOUBLE]);
        } else {
            quoted.push(c);
        }
    }
    quoted.push(DOUBLE);
    Some(OsString::from_wide(&quoted))
}
