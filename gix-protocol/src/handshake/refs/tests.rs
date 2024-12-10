use gix_transport::client;

use crate::handshake::{refs, refs::shared::InternalRef};

#[test]
fn extract_symbolic_references_from_capabilities() -> Result<(), client::Error> {
    let caps = client::Capabilities::from_bytes(
        b"\0unrelated symref=HEAD:refs/heads/main symref=ANOTHER:refs/heads/foo symref=MISSING_NAMESPACE_TARGET:(null) agent=git/2.28.0",
    )?
        .0;
    let out = refs::shared::from_capabilities(caps.iter()).expect("a working example");

    assert_eq!(
        out,
        vec![
            InternalRef::SymbolicForLookup {
                path: "HEAD".into(),
                target: Some("refs/heads/main".into())
            },
            InternalRef::SymbolicForLookup {
                path: "ANOTHER".into(),
                target: Some("refs/heads/foo".into())
            },
            InternalRef::SymbolicForLookup {
                path: "MISSING_NAMESPACE_TARGET".into(),
                target: None
            }
        ]
    );
    Ok(())
}
