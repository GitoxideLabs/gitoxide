use std::collections::BTreeMap;

use bstr::ByteSlice;
use gix_error::{Result, ResultExt, message};
use gix_hash::ObjectId;
use gix_validate::path::SpecialComponent;

use super::{Diagnostic, MessageId as M, Options, Reporter};

#[derive(Clone, Copy, Debug, Default)]
struct Kinds {
    modules: bool,
    attributes: bool,
}

/// Deferred validation of blobs whose meaning depends on their tree entry names.
///
/// Observe every relevant tree, then call [`Self::validate()`] once all referenced
/// objects are available. Shared blobs are checked once, regardless of traversal
/// order. These checks supplement standalone structure and connectivity validation.
#[derive(Clone, Debug, Default)]
pub struct SpecialBlobs {
    blobs: BTreeMap<ObjectId, Kinds>,
}

impl SpecialBlobs {
    /// Discover `.gitmodules` and `.gitattributes` blobs and diagnose symbolic links
    /// at special names. HFS and NTFS aliases are recognized on every platform.
    /// Skipped trees suppress their own diagnostics but still discover special blobs.
    pub fn observe_tree(&mut self, tree_id: ObjectId, data: &[u8], options: &Options) -> Result<Vec<Diagnostic>> {
        let mut out = Reporter::new(tree_id, options);
        for entry in super::tree::Entries::new(data, tree_id.kind()) {
            let Ok(entry) = entry else {
                out.report(M::BadTree);
                break;
            };
            let name = entry.name.as_bstr();
            let modules = SpecialComponent::GitModules.matches(name)
                || entry.name.iter().enumerate().any(|(index, byte)| {
                    *byte == b'\\' && SpecialComponent::GitModules.matches_ntfs(entry.name[index + 1..].as_bstr())
                });
            let attributes = SpecialComponent::GitAttributes.matches(name);
            if entry.mode & 0o170000 == 0o120000 {
                for (matches, id) in [
                    (modules, M::GitModulesSymlink),
                    (attributes, M::GitAttributesSymlink),
                    (SpecialComponent::GitIgnore.matches(name), M::GitIgnoreSymlink),
                    (SpecialComponent::Mailmap.matches(name), M::MailmapSymlink),
                ] {
                    if matches {
                        out.report(id);
                    }
                }
            } else if modules || attributes {
                let kinds = self.blobs.entry(entry.object_id.to_owned()).or_default();
                kinds.modules |= modules;
                kinds.attributes |= attributes;
            }
        }
        Ok(out.finish())
    }

    /// Resolve and validate the special blobs discovered by [`Self::observe_tree()`].
    /// Missing objects and unexpected kinds produce diagnostics; object-store errors
    /// retain their original causes. Includes in `.gitmodules` are never followed.
    pub fn validate(&self, objects: &impl gix_object::Find, options: &Options) -> Result<Vec<Diagnostic>> {
        let mut diagnostics = Vec::new();
        let mut buffer = Vec::new();
        for (&blob_id, kinds) in &self.blobs {
            if options.is_skipped(&blob_id) {
                continue;
            }
            let mut out = Reporter::new(blob_id, options);
            let object = objects
                .try_find(&blob_id, &mut buffer)
                .or_raise(|| message!("Could not read special blob {blob_id}"))?;
            match object {
                None => {
                    if kinds.modules {
                        out.report(M::GitModulesMissing);
                    }
                    if kinds.attributes {
                        out.report(M::GitAttributesMissing);
                    }
                }
                Some(object) if object.kind != gix_object::Kind::Blob => {
                    if kinds.modules {
                        out.report(M::GitModulesBlob);
                    }
                    if kinds.attributes {
                        out.report(M::GitAttributesBlob);
                    }
                }
                Some(object) => {
                    if kinds.modules {
                        validate_modules(object.data, &mut out);
                    }
                    if kinds.attributes {
                        validate_attributes(object.data, &mut out);
                    }
                }
            }
            diagnostics.extend(out.finish());
        }
        Ok(diagnostics)
    }
}

fn validate_attributes(data: &[u8], out: &mut Reporter<'_>) {
    // These are Git's ATTR_MAX_FILE_SIZE and ATTR_MAX_LINE_LENGTH limits. Scan
    // bytes rather than C strings so a NUL cannot hide an oversized later line.
    if data.len() > 100 * 1024 * 1024 {
        out.report(M::GitAttributesLarge);
    } else if data.split(|byte| *byte == b'\n').any(|line| line.len() >= 2048) {
        out.report(M::GitAttributesLineLength);
    }
}

fn validate_modules(data: &[u8], out: &mut Reporter<'_>) {
    if data.len() > u32::MAX as usize {
        out.report(M::GitModulesLarge);
        return;
    }
    let (events, error) = gix_config::parse::Events::from_bytes_partial(data, None);
    let config = gix_config::File::from_parse_events_no_includes(events, gix_config::file::Metadata::api());
    for section in config.sections() {
        let header = section.header();
        if !header.name().eq_ignore_ascii_case(b"submodule") || section.value_names().next().is_none() {
            continue;
        }
        let Some(name) = header.subsection_name() else {
            continue;
        };
        if gix_validate::submodule::name(name).is_err() {
            out.report(M::GitModulesName);
        }
        for value in section.values("url") {
            if !submodule_url_is_safe(value.as_ref()) {
                out.report(M::GitModulesUrl);
            }
        }
        if section.values("path").iter().any(|value| value.starts_with(b"-")) {
            out.report(M::GitModulesPath);
        }
        if section.values("update").iter().any(|value| value.starts_with(b"!")) {
            out.report(M::GitModulesUpdate);
        }
    }
    if error.is_some() {
        out.report(M::GitModulesParse);
    }
}

fn submodule_url_is_safe(url: &[u8]) -> bool {
    if url.starts_with(b"-") {
        return false;
    }
    let mut relative = url;
    let mut parents = 0;
    while let Some((length, parent)) = dot_component(relative) {
        relative = &relative[length..];
        parents += usize::from(parent);
    }
    if relative.len() != url.len() || url.starts_with(b"git://") {
        return !(decode_percent(url, false).is_some_and(|decoded| decoded.contains(&b'\n'))
            || parents > 0 && matches!(relative.first(), Some(b':' | b'/')));
    }
    for scheme in [b"http".as_slice(), b"https", b"ftp", b"ftps"] {
        if let Some(rest) = url.strip_prefix(scheme) {
            if let Some(curl_url) = rest.strip_prefix(b"::") {
                return curl_url_is_safe(curl_url);
            }
            if rest.starts_with(b"://") {
                return curl_url_is_safe(url);
            }
        }
    }
    true
}

fn dot_component(input: &[u8]) -> Option<(usize, bool)> {
    if input.starts_with(b"..") && matches!(input.get(2), Some(b'/' | b'\\')) {
        Some((3, true))
    } else if input.starts_with(b".") && matches!(input.get(1), Some(b'/' | b'\\')) {
        Some((2, false))
    } else {
        None
    }
}

// Fsck checks Git's URL normalization grammar, which is intentionally more
// permissive than transport parsing (including arbitrary schemes passed to curl
// helpers). Validate that grammar without changing or interpreting the URL.
fn curl_url_is_safe(url: &[u8]) -> bool {
    let Some(scheme_end) = url.find(b"://") else {
        return false;
    };
    let scheme = &url[..scheme_end];
    if !scheme.first().is_some_and(u8::is_ascii_alphabetic)
        || !scheme
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
    {
        return false;
    }
    let rest = &url[scheme_end + 3..];
    let authority_end = rest
        .iter()
        .position(|byte| matches!(byte, b'/' | b'?' | b'#'))
        .unwrap_or(rest.len());
    let mut authority = &rest[..authority_end];
    if let Some(at) = authority.find_byte(b'@') {
        if decode_percent(&authority[..at], true).is_none_or(|value| value.contains(&b'\n')) {
            return false;
        }
        authority = &authority[at + 1..];
    }
    let port_separator = authority
        .iter()
        .rposition(|byte| matches!(byte, b':' | b']'))
        .filter(|index| authority[*index] == b':');
    let host = &authority[..port_separator.unwrap_or(authority.len())];
    if (host.is_empty() && !scheme.eq_ignore_ascii_case(b"file"))
        || !host
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'[' | b':' | b']'))
    {
        return false;
    }
    if let Some(separator) = port_separator {
        let port = &authority[separator + 1..];
        if !port.is_empty()
            && (host.is_empty()
                || !port.iter().all(u8::is_ascii_digit)
                || port
                    .iter()
                    .try_fold(0u16, |value, byte| {
                        value.checked_mul(10)?.checked_add(u16::from(byte - b'0'))
                    })
                    .is_none_or(|value| value == 0))
        {
            return false;
        }
    }
    let rest = &rest[authority_end..];
    let path_end = rest
        .iter()
        .position(|byte| matches!(byte, b'?' | b'#'))
        .unwrap_or(rest.len());
    if decode_percent(&rest[path_end..], true).is_none_or(|value| value.contains(&b'\n')) {
        return false;
    }
    let path = rest[..path_end].strip_prefix(b"/").unwrap_or(&rest[..path_end]);
    let mut segments = Vec::new();
    for segment in path.split(|byte| *byte == b'/') {
        let Some(decoded) = decode_percent(segment, true) else {
            return false;
        };
        if decoded == b".." {
            if segments.pop().is_none() {
                return false;
            }
        } else if decoded != b"." {
            segments.push(decoded.contains(&b'\n'));
        }
    }
    !segments.into_iter().any(|has_newline| has_newline)
}

fn decode_percent(input: &[u8], strict: bool) -> Option<Vec<u8>> {
    fn hex(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }
    let mut decoded = Vec::with_capacity(input.len());
    let mut cursor = 0;
    while cursor < input.len() {
        if input[cursor] == b'%' {
            let byte = input
                .get(cursor + 1..cursor + 3)
                .and_then(|digits| Some(hex(digits[0])? * 16 + hex(digits[1])?));
            if let Some(byte) = byte {
                decoded.push(byte);
                cursor += 3;
                continue;
            }
            if strict {
                return None;
            }
        }
        decoded.push(input[cursor]);
        cursor += 1;
    }
    Some(decoded)
}
