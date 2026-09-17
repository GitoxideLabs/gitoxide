mod path;
mod reference;
mod submodule;
mod tag;

#[test]
fn invalid_names_retain_their_classification() {
    use bstr::ByteSlice;
    use gix_error::ErrorExt;

    fn check<E: std::error::Error + Send + Sync + 'static>(err: E) {
        let err = err.raise();
        assert!(err.is_validation(), "invalid names classify as validation failures");
        let err = err.into_error();
        assert!(err.is_validation(), "conversion preserves the classification");
        assert!(
            err.downcast_any_ref::<E>().is_some(),
            "the concrete validation error remains available"
        );
    }

    check(gix_validate::reference::name(b"refs//heads/main".as_bstr()).expect_err("repeated slashes are invalid"));
    check(gix_validate::tag::name(b"v1..0".as_bstr()).expect_err("repeated dots are invalid"));
    check(gix_validate::submodule::name(b"../module".as_bstr()).expect_err("parent components are invalid"));
    check(
        gix_validate::path::component(b".git".as_bstr(), None, Default::default())
            .expect_err("the Git directory cannot be a worktree entry"),
    );
}
