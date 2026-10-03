#![cfg(feature = "forest")]

// Keep global initialization in its own test executable: the subscriber cannot be reset.
#[test]
fn repeated_initialization_returns_errors() -> Result<(), Box<dyn std::error::Error>> {
    gix_trace::forest::init()?;
    assert!(
        gix_trace::forest::init().is_err(),
        "installing a second global forest subscriber returns an error instead of panicking"
    );
    assert!(
        gix_trace::forest::test_init().is_err(),
        "the test initializer also leaves the existing global subscriber installed"
    );
    Ok(())
}
