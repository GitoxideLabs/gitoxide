fn requires_find<T: gix_object::Find>(_: &T) {}

fn inspect(repo: &gix::Repository) -> gix::Result<()> {
    requires_find(repo);
    let _: &gix::OdbHandle = &repo.objects;
    let _: gix::ObjectId = repo.head_id()?.detach();
    Ok(())
}

fn main() {
    let _inspect = inspect;
}
