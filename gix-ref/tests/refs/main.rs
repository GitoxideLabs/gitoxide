use gix_hash::ObjectId;
pub use gix_testtools::{
    scripted_fixture_read_only, scripted_fixture_read_only_with_args, scripted_fixture_writable,
    scripted_fixture_writable_with_args,
};

pub fn hex_to_id(hex: &str) -> ObjectId {
    match fixture_hash_kind() {
        gix_hash::Kind::Sha1 => ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => {
            ObjectId::from_hex(translate_sha1_to_fixture_sha256(hex).as_bytes()).expect("64 bytes hex")
        }
        _ => unreachable!("tests only support known hash kinds"),
    }
}

pub fn sha1_hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

pub fn fixture_hash_kind() -> gix_hash::Kind {
    gix_testtools::object_hash()
}

fn translate_sha1_to_fixture_sha256(hex: &str) -> String {
    match hex {
        "0000000000000000000000000000000000000000" => {
            "0000000000000000000000000000000000000000000000000000000000000000".into()
        }
        "134385f6d781b7e97062102c6a483440bfda2a03" => {
            "5c4c31e0551f0d1fb410b7b9366604b050ea3388b96885063f10ba4c3e2dedd0".into()
        }
        "4c3f4cce493d7beb45012e478021b5f65295e5a3" => {
            "2c309d047b92197ef711ba55ab652c42d36750d6571a3e024a7325e324be3033".into()
        }
        "9902e3c3e8f0c569b4ab295ddf473e6de763e1e7" => {
            "bbaf9640a7404a15394dae2606c5090cb44a722be2167d9d78485779aaf4e065".into()
        }
        "17d78c64cef6c33a10a604573fd2c429e477fd63" => {
            "e47e1df5636110feefb5b858c346dbd1c0feebfc37651a238ec5a6300ed2f666".into()
        }
        "9556057aee5abb06912922e9f26c46386a816822" => {
            "9a3e230fc8479e41397b78b9295510e38be525ec05a08c1ceb797547dc93ed4c".into()
        }
        "d3ba65e5e3be5cdd7210da9998307a4762999cc5" => {
            "8aa62135237b610c0e58159f0a0d7a763371ed72dc046dfda6baf1a30ab8511a".into()
        }
        "b3109a7e51fc593f85b145a76c70ddd1d133fafd" => {
            "1ce70f5e127ba939d70e3b1643213b1dbb0dfedc3079f57a73b2d18cd6cc8a02".into()
        }
        "02a7a22d90d7c02fb494ed25551850b868e634f0" => {
            "c87659e8e5d86a499a88a3869342d1367b918aab9675577571fbc2d2ea2a24b8".into()
        }
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391" => gix_hash::Kind::Sha256.empty_blob().to_string(),
        other => {
            let oid = ObjectId::from_hex(other.as_bytes()).expect("40 bytes hex");
            let mut hasher = gix_hash::hasher(gix_hash::Kind::Sha256);
            hasher.update(oid.as_bytes());
            hasher.try_finalize().expect("sha256 hashing works").to_string()
        }
    }
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
mod transaction;
