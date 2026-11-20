use super::{MessageId as M, Reporter, headers};

pub(super) fn validate(data: &[u8], hash: gix_hash::Kind, out: &mut Reporter<'_>) {
    let Some(lines) = headers::lines(data, out) else { return };
    let mut lines = lines.peekable();
    let Some(tree) = lines.next().and_then(|line| line.strip_prefix(b"tree ")) else {
        out.report(M::MissingTree);
        return;
    };
    if !headers::hash(tree, hash) && out.report(M::BadTreeSha1) {
        return;
    }
    while let Some(parent) = lines.peek().and_then(|line| line.strip_prefix(b"parent ")) {
        if !headers::hash(parent, hash) && out.report(M::BadParentSha1) {
            return;
        }
        lines.next();
    }
    let mut authors = 0;
    while let Some(author) = lines.peek().and_then(|line| line.strip_prefix(b"author ")) {
        authors += 1;
        if headers::identity(author, out) {
            return;
        }
        lines.next();
    }
    if authors == 0 && out.report(M::MissingAuthor) || authors > 1 && out.report(M::MultipleAuthors) {
        return;
    }
    let Some(committer) = lines.next().and_then(|line| line.strip_prefix(b"committer ")) else {
        out.report(M::MissingCommitter);
        return;
    };
    if headers::identity(committer, out) {
        return;
    }
    if data.contains(&0) {
        out.report(M::NulInCommit);
    }
}
