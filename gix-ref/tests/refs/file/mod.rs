use gix_ref::file;

// TODO: when ready, add a new test entry point with a feature toggle to switch this to `gix_ref::Store`.
//       That way all tests can run against the new general store to validate its truly working.
//       The same can be done when RefTable is available, and its corresponding tests.
pub type Store = file::Store;

fn store() -> crate::Result<Store> {
    store_at("make_ref_repository.sh")
}

pub fn store_with_packed_refs() -> crate::Result<Store> {
    store_at("make_packed_ref_repository.sh")
}

pub fn store_at(name: &str) -> crate::Result<Store> {
    named_store_at(name, "")
}

pub fn named_store_at(script_name: &str, name: &str) -> crate::Result<Store> {
    let path = crate::scripted_fixture_read_only(script_name)?;
    Ok(Store::at(path.join(name).join(".git"), store_options()))
}

pub fn store_at_with_args(name: &str, args: impl IntoIterator<Item = impl Into<String>>) -> crate::Result<Store> {
    let path = crate::scripted_fixture_read_only_with_args(name, args)?;
    Ok(Store::at(path.join(".git"), store_options()))
}

fn store_writable(name: &str) -> crate::Result<(gix_testtools::tempfile::TempDir, Store)> {
    let dir = crate::scripted_fixture_writable(name)?;
    let git_dir = dir.path().join(".git");
    Ok((dir, Store::at(git_dir, store_options())))
}

pub fn store_options() -> gix_ref::store::init::Options {
    gix_ref::store::init::Options {
        object_hash: crate::fixture_hash_kind(),
        ..Default::default()
    }
}

pub fn odb_at(objects_dir: impl Into<std::path::PathBuf>) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at_opts(
        objects_dir,
        Vec::new(),
        gix_odb::store::init::Options {
            object_hash: crate::fixture_hash_kind(),
            ..Default::default()
        },
    )
}

struct EmptyCommit;
impl gix_object::Find for EmptyCommit {
    fn try_find<'a>(
        &self,
        id: &gix_hash::oid,
        _buffer: &'a mut Vec<u8>,
    ) -> Result<Option<gix_object::Data<'a>>, gix_object::find::Error> {
        Ok(Some(gix_object::Data {
            kind: gix_object::Kind::Commit,
            object_hash: id.kind(),
            data: &[],
        }))
    }
}

mod log;
mod reference;
mod store;
pub(crate) mod transaction;
mod worktree;
