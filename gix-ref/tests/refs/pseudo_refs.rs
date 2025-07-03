use crate::file::store_at;

#[test]
fn pseudo_refs_iterate_valid_pseudorefs() -> crate::Result {
    let store = store_at("make_pref_repository.sh")?;

    let prefs = store
        .iter_pseudo_refs()?
        .map(Result::unwrap)
        .map(|r: gix_ref::Reference| r.name)
        .collect::<Vec<_>>();

    let expected_prefs = vec!["FETCH_HEAD", "HEAD", "JIRI_HEAD"];

    assert_eq!(
        prefs.iter().map(gix_ref::FullName::as_bstr).collect::<Vec<_>>(),
        expected_prefs
    );

    Ok(())
}
