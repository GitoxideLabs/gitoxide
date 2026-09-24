mod path;
mod reference;
mod submodule;
mod tag;

#[test]
fn invalid_names_retain_their_classification() {
    use bstr::ByteSlice;
    use gix_error::ErrorExt;

    fn check<E: std::error::Error + Send + Sync + 'static>(err: E) -> gix_error::Exn {
        let err = err.raise();
        assert!(err.is_validation(), "invalid names classify as validation failures");
        assert!(
            err.downcast_any_ref::<E>().is_some(),
            "the concrete validation error remains available "
        );
        assert!(
            err.probable_cause().is::<E>(),
            "the concrete validation error, not its classification marker, is the probable cause"
        );
        err.erased()
    }

    insta::assert_debug_snapshot!(check(gix_validate::reference::name(b"refs//heads/main".as_bstr()).expect_err("repeated slashes are invalid")), "invalid names identify the rejected syntax", @"Reference name cannot contain repeated slashes");
    insta::assert_debug_snapshot!(check(gix_validate::tag::name(b"v1..0".as_bstr()).expect_err("repeated dots are invalid")), "invalid names identify the rejected syntax", @"A ref must not contain '..' as it may be mistaken for a range");
    insta::assert_debug_snapshot!(check(gix_validate::submodule::name(b"../module".as_bstr()).expect_err("parent components are invalid")), "invalid names identify the rejected syntax", @"Submodules names must not contains '..'");
    insta::assert_debug_snapshot!(check(
        gix_validate::path::component(b".git".as_bstr(), None, Default::default())
            .expect_err("the Git directory cannot be a worktree entry"),
    ), "invalid names identify the rejected syntax", @"The .git name may never be used");
}
