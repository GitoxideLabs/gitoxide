use crate::Result;
use crate::file::{named_store_at, store};

#[test]
fn set_packed_buffer_mmap_threshold() -> Result {
    let mut store = store()?;
    let prev = store.set_packed_buffer_mmap_threshold(0);
    if cfg!(windows) {
        assert_eq!(
            prev,
            u64::MAX,
            "on windows mmap are deactivated as otherwise we can't change packed-refs while it's mapped"
        );
    } else {
        assert_eq!(prev, 32 * 1024, "the default is the value that Git uses as well");
    }
    assert_eq!(
        store.set_packed_buffer_mmap_threshold(0),
        0,
        "it actually sets the value"
    );
    Ok(())
}

#[test]
fn is_pristine() -> Result {
    let store = named_store_at("make_pristine.sh", "untouched")?;
    assert_eq!(store.is_pristine("refs/heads/main".try_into()?), Some(true));
    assert_eq!(store.is_pristine("refs/heads/other".try_into()?), Some(false));

    let store = named_store_at("make_pristine.sh", "changed-headref")?;
    assert_eq!(store.is_pristine("refs/heads/other".try_into()?), Some(true));
    assert_eq!(store.is_pristine("refs/heads/main".try_into()?), Some(false));

    let store = named_store_at("make_pristine.sh", "detached")?;
    assert_eq!(store.is_pristine("refs/heads/main".try_into()?), Some(false));

    let store = named_store_at("make_pristine.sh", "invalid-loose-ref")?;
    assert_eq!(store.is_pristine("refs/heads/main".try_into()?), Some(true));

    Ok(())
}
