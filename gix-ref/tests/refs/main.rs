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
            matches!(err, gix_validate::reference::name::Error::EndsWithSlash),
            "thanks to this there is no worry about dealing with this case"
        );

        let pn = PartialName::try_from("prefix")?;
        let err = pn.join("/slash-in-name".into()).unwrap_err();
        assert!(
            matches!(err, gix_validate::reference::name::Error::RepeatedSlash),
            "validation post-join assures the returned type is valid"
        );
        Ok(())
    }

    #[test]
    fn display() {
        let partial_name = PartialName::try_from("heads/main").unwrap();
        assert_eq!(format!("{partial_name}"), "heads/main");

        let partial_name_ref = partial_name.as_ref();
        assert_eq!(format!("{partial_name_ref}"), "heads/main");
    }
}
mod namespace;
mod packed;
mod reference;
mod store;
mod traits;
mod transaction;
