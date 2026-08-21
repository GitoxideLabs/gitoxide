use gix_ref::{FullName, Target};

#[test]
fn strip_namespace() {
    let ns = gix_ref::namespace::expand("ns").unwrap();
    let mut r = gix_ref::Reference {
        name: {
            let mut n: FullName = "refs/heads/main".try_into().unwrap();
            n.prefix_namespace(&ns);
            n
        },
        target: Target::Symbolic({
            let mut n: FullName = "refs/tags/foo".try_into().unwrap();
            n.prefix_namespace(&ns);
            n
        }),
        peeled: None,
    };
    r.strip_namespace(&ns);
    assert_eq!(r, "refs/heads/main", "name is stripped");
    assert!(
        matches!(r.target, Target::Symbolic(n) if n == "refs/tags/foo"),
        "and the symbolic target as well"
    );
}
