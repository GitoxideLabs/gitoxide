use gix_hash::ObjectId;

pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

pub use gix_testtools::Result;

mod file;
mod fullname;
mod partialname {
    use gix_ref::PartialName;

    #[test]
    fn join() -> crate::Result {
        let pn = PartialName::try_from("no-trailing-slash")?;
        assert_eq!(pn.join("name".into())?.as_ref().as_bstr(), "no-trailing-slash/name");

        let err = PartialName::try_from("trailing-slash/").unwrap_err();
        assert!(
            matches!(
                err,
                gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::EndsWithSlash)
            ),
            "thanks to this there is no worry about dealing with this case"
        );

        let pn = PartialName::try_from("prefix")?;
        let err = pn.join("/slash-in-name".into()).unwrap_err();
        assert!(
            matches!(
                err,
                gix_validate::reference::name::Error::Tag(gix_validate::tag::name::Error::RepeatedSlash)
            ),
            "validation post-join assures the returned type is valid"
        );
        Ok(())
    }
}
mod namespace;
mod packed;
mod pseudo_refs;
mod reference;
mod store;
mod transaction;
