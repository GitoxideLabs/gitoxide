use crate::hex_to_id;

fn oid() -> gix_hash::ObjectId {
    hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
}

fn oid2() -> gix_hash::ObjectId {
    hex_to_id("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
}

#[test]
fn include() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        gix_hash::Kind::Sha256 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        _ => unimplemented!(),
    };
    assert_eq!(gix_revision::Spec::Include(oid()).to_string(), expected);
}

#[test]
fn exclude() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "^aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        gix_hash::Kind::Sha256 => "^aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        _ => unimplemented!(),
    };
    assert_eq!(gix_revision::Spec::Exclude(oid()).to_string(), expected);
}

#[test]
fn range() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa..bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        gix_hash::Kind::Sha256 => {
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa..bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
        _ => unimplemented!(),
    };
    assert_eq!(
        gix_revision::Spec::Range {
            from: oid(),
            to: oid2()
        }
        .to_string(),
        expected
    );
}

#[test]
fn merge() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        gix_hash::Kind::Sha256 => {
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa...bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
        _ => unimplemented!(),
    };
    assert_eq!(
        gix_revision::Spec::Merge {
            theirs: oid(),
            ours: oid2()
        }
        .to_string(),
        expected
    );
}

#[test]
fn include_parents() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^@",
        gix_hash::Kind::Sha256 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^@",
        _ => unimplemented!(),
    };
    assert_eq!(gix_revision::Spec::IncludeOnlyParents(oid()).to_string(), expected);
}

#[test]
fn exclude_parents() {
    let expected = match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^!",
        gix_hash::Kind::Sha256 => "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa^!",
        _ => unimplemented!(),
    };
    assert_eq!(gix_revision::Spec::ExcludeParents(oid()).to_string(), expected);
}
