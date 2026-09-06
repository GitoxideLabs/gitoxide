mod builtin_driver;
mod pipeline;
mod platform;

mod util {
    use gix_object::Write;

    pub type ObjectDb = gix_odb::memory::Proxy<gix_object::find::Never>;

    pub fn object_db() -> ObjectDb {
        gix_odb::memory::Proxy::new(gix_object::find::Never, gix_hash::Kind::Sha1)
    }

    /// Insert `data` and return its hash. That can be used to find it again.
    pub fn insert(db: &ObjectDb, data: &str) -> Result<gix_hash::ObjectId, gix_error::Exn> {
        db.write_buf(gix_object::Kind::Blob, data.as_bytes())
    }
}
