use gix_attributes::{AssignmentRef, NameRef, StateRef};

#[test]
fn display() {
    assert_eq!(adisplay("hello", StateRef::Unspecified), "!hello");
    assert_eq!(adisplay("hello", StateRef::Unset), "-hello");
    assert_eq!(adisplay("hello", StateRef::Set), "hello");
    assert_eq!(adisplay("hello", StateRef::Value("value".into())), "hello=value");
}

fn adisplay(name: &str, state: StateRef<'_>) -> String {
    AssignmentRef {
        name: NameRef::try_from(bstr::BStr::new(name.as_bytes())).expect("valid name"),
        state,
    }
    .to_string()
}

#[test]
fn names_must_not_be_empty_but_may_use_the_builtin_namespace() {
    assert!(
        NameRef::try_from(bstr::BStr::new(b"")).is_err(),
        "an empty attribute name isn't valid"
    );
    assert!(
        NameRef::try_from(bstr::BStr::new(b"builtin_objectmode")).is_ok(),
        "the reserved namespace is valid for queries and pathspecs"
    );
}

#[cfg(feature = "serde")]
#[test]
fn deserialize_rejects_empty_names() {
    use serde::Deserialize;

    let deserializer = serde::de::value::StringDeserializer::<serde::de::value::Error>::new(String::new());
    gix_attributes::Name::deserialize(deserializer).expect_err("empty names must remain invalid after deserialization");
}
