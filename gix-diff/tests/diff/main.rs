use gix_testtools::Result;
pub use gix_testtools::{normalize_debug_snapshot, scripted_fixture_read_only, scripted_fixture_read_only_with_args};
use std::collections::HashMap;

fn hex_to_id(hex_sha1: &str, hex_sha256: &str) -> gix_hash::ObjectId {
    match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::ObjectId::from_hex(hex_sha1.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => gix_hash::ObjectId::from_hex(hex_sha256.as_bytes()).expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}

fn fixture_hash_kind() -> gix_hash::Kind {
    gix_testtools::object_hash()
}

fn open_odb(objects_dir: impl Into<std::path::PathBuf>) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at_opts(
        objects_dir,
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash: fixture_hash_kind(),
            ..Default::default()
        },
    )
}

fn normalize_patch_snapshot(input: &str) -> String {
    fn normalize_hex_token<'a>(token: &'a str, seen: &mut HashMap<&'a str, usize>, next_id: &mut usize) -> String {
        if token == "0000000" || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return token.to_owned();
        }
        let normalized = *seen.entry(token).or_insert_with(|| {
            let current = *next_id;
            *next_id += 1;
            current
        });
        format!("Oid({normalized})")
    }

    let mut seen = HashMap::<&str, usize>::new();
    let mut next_id = 1usize;

    input
        .lines()
        .map(|line| {
            if let Some(commit) = line.strip_prefix("commit ") {
                return format!("commit {}", normalize_hex_token(commit, &mut seen, &mut next_id));
            }

            if let Some(index) = line.strip_prefix("index ") {
                let (range, mode) = index
                    .split_once(' ')
                    .map_or((index, None), |(range, mode)| (range, Some(mode)));
                if let Some((lhs, rhs)) = range.split_once("..") {
                    let lhs = normalize_hex_token(lhs, &mut seen, &mut next_id);
                    let rhs = normalize_hex_token(rhs, &mut seen, &mut next_id);
                    return match mode {
                        Some(mode) => format!("index {lhs}..{rhs} {mode}"),
                        None => format!("index {lhs}..{rhs}"),
                    };
                }
            }

            line.to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}

fn assert_hash_agnostic_patch_eq(actual: &str, expected: &str) {
    pretty_assertions::assert_eq!(normalize_patch_snapshot(expected), normalize_patch_snapshot(actual));
}

mod blob;
mod index;
mod rewrites;
mod tree;
mod tree_with_rewrites;

mod util {
    use std::collections::HashMap;

    use gix_hash::oid;
    use gix_object::{bstr::BString, find::Error};

    #[derive(Default)]
    pub struct ObjectDb {
        data_by_id: HashMap<gix_hash::ObjectId, BString>,
    }

    impl gix_object::FindHeader for ObjectDb {
        fn try_header(&self, id: &oid) -> Result<Option<gix_object::Header>, Error> {
            match self.data_by_id.get(&id.to_owned()) {
                Some(data) => Ok(Some(gix_object::Header {
                    kind: gix_object::Kind::Blob,
                    size: data.len() as u64,
                })),
                None => Ok(None),
            }
        }
    }

    impl gix_object::Find for ObjectDb {
        fn try_find<'a>(&self, id: &oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>, Error> {
            match self.data_by_id.get(&id.to_owned()) {
                Some(data) => {
                    buffer.clear();
                    buffer.extend_from_slice(data);
                    Ok(Some(gix_object::Data {
                        kind: gix_object::Kind::Blob,
                        object_hash: id.kind(),
                        data: buffer.as_slice(),
                    }))
                }
                None => Ok(None),
            }
        }
    }

    impl ObjectDb {
        /// Insert `data` and return its hash. That can be used to find it again.
        pub fn insert(&mut self, data: &str) -> Result<gix_hash::ObjectId, Error> {
            let id = gix_object::compute_hash(super::fixture_hash_kind(), gix_object::Kind::Blob, data.as_bytes())?;
            self.data_by_id.insert(id, data.into());
            Ok(id)
        }
    }
}
