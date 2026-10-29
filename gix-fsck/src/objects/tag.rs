use bstr::ByteSlice;

use super::{MessageId as M, Reporter, headers};

pub(super) fn validate(data: &[u8], hash: gix_hash::Kind, out: &mut Reporter<'_>) {
    let Some(lines) = headers::lines(data, out) else { return };
    let mut lines = lines.peekable();
    let Some(object) = lines.next().and_then(|line| line.strip_prefix(b"object ")) else {
        out.report(M::MissingObject);
        return;
    };
    if !headers::hash(object, hash) && out.report(M::BadObjectSha1) {
        return;
    }
    let Some(kind) = lines.next().and_then(|line| line.strip_prefix(b"type ")) else {
        out.report(M::MissingTypeEntry);
        return;
    };
    if gix_object::Kind::from_bytes(kind).is_err() && out.report(M::BadType) {
        return;
    }
    let Some(name) = lines.next().and_then(|line| line.strip_prefix(b"tag ")) else {
        out.report(M::MissingTagEntry);
        return;
    };
    let mut full_name = b"refs/tags/".to_vec();
    full_name.extend_from_slice(name);
    if gix_validate::reference::name(full_name.as_bstr()).is_err() && out.report(M::BadTagName) {
        return;
    }
    if let Some(tagger) = lines.peek().and_then(|line| line.strip_prefix(b"tagger ")) {
        if headers::identity(tagger, out) {
            return;
        }
        lines.next();
    } else if out.report(M::MissingTaggerEntry) {
        return;
    }
    if lines
        .peek()
        .is_some_and(|line| line.starts_with(b"gpgsig ") || line.starts_with(b"gpgsig-sha256 "))
    {
        lines.next();
        while lines.peek().is_some_and(|line| line.starts_with(b" ")) {
            lines.next();
        }
    }
    if lines.next().is_some() {
        out.report(M::ExtraHeaderEntry);
    }
}
