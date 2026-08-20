use crate::{
    FullName, FullNameRef, Namespace, PartialName, PartialNameRef, Reference,
    bstr::{BStr, BString, ByteSlice},
    file, packed,
};

macro_rules! impl_partial_eq {
    ($left_type:ty, $right_type:ty, $left:ident => $left_bytes:expr, $right:ident => $right_bytes:expr) => {
        impl PartialEq<$right_type> for $left_type {
            fn eq(&self, other: &$right_type) -> bool {
                let $left = self;
                let $right = other;
                $left_bytes == $right_bytes
            }
        }
    };
}

macro_rules! impl_partial_eq_pair {
    ($left_type:ty, $right_type:ty, $left:ident => $left_bytes:expr, $right:ident => $right_bytes:expr) => {
        impl_partial_eq!($left_type, $right_type, $left => $left_bytes, $right => $right_bytes);
        impl_partial_eq!($right_type, $left_type, $right => $right_bytes, $left => $left_bytes);
    };
}

macro_rules! impl_partial_eq_bytes {
    ($type:ty, $value:ident => $bytes:expr) => {
        impl_partial_eq_pair!($type, str, $value => $bytes.as_bytes(), other => other.as_bytes());
        impl_partial_eq_pair!($type, &str, $value => $bytes.as_bytes(), other => other.as_bytes());
        impl_partial_eq_pair!($type, String, $value => $bytes.as_bytes(), other => other.as_bytes());
        impl_partial_eq_pair!($type, BStr, $value => $bytes.as_bytes(), other => other.as_bytes());
        impl_partial_eq_pair!($type, &BStr, $value => $bytes.as_bytes(), other => other.as_bytes());
        impl_partial_eq_pair!($type, BString, $value => $bytes.as_bytes(), other => other.as_bytes());
    };
}

macro_rules! impl_partial_eq_reference {
    ($type:ty, $value:ident => $name:expr) => {
        impl_partial_eq!($type, str, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!($type, &str, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!($type, String, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!($type, BStr, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!($type, &BStr, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!($type, BString, $value => $name.as_bytes(), other => other.as_bytes());
        impl_partial_eq!(
            $type,
            FullName,
            $value => $name.as_bytes(),
            other => other.as_bstr().as_bytes()
        );
        impl_partial_eq!(
            $type,
            FullNameRef,
            $value => $name.as_bytes(),
            other => other.as_bstr().as_bytes()
        );
        impl_partial_eq!(
            $type,
            &FullNameRef,
            $value => $name.as_bytes(),
            other => other.as_bstr().as_bytes()
        );
    };
}

impl_partial_eq_bytes!(FullName, value => value.as_bstr());
impl_partial_eq_bytes!(FullNameRef, value => value.as_bstr());
impl_partial_eq_bytes!(PartialName, value => value.as_ref().as_bstr());
impl_partial_eq_bytes!(PartialNameRef, value => value.as_bstr());
impl_partial_eq_bytes!(Namespace, value => value.as_bstr());

impl_partial_eq_pair!(
    &FullNameRef,
    String,
    name => name.as_bstr().as_bytes(),
    text => text.as_bytes()
);
impl_partial_eq_pair!(
    &FullNameRef,
    BString,
    name => name.as_bstr().as_bytes(),
    text => text.as_bytes()
);
impl_partial_eq_pair!(
    &PartialNameRef,
    String,
    name => name.as_bstr().as_bytes(),
    text => text.as_bytes()
);
impl_partial_eq_pair!(
    &PartialNameRef,
    BString,
    name => name.as_bstr().as_bytes(),
    text => text.as_bytes()
);

impl_partial_eq_pair!(
    FullName,
    FullNameRef,
    owned => owned.as_bstr().as_bytes(),
    borrowed => borrowed.as_bstr().as_bytes()
);
impl_partial_eq_pair!(
    FullName,
    &FullNameRef,
    owned => owned.as_bstr().as_bytes(),
    borrowed => borrowed.as_bstr().as_bytes()
);
impl_partial_eq_pair!(
    PartialName,
    PartialNameRef,
    owned => owned.as_ref().as_bstr().as_bytes(),
    borrowed => borrowed.as_bstr().as_bytes()
);
impl_partial_eq_pair!(
    PartialName,
    &PartialNameRef,
    owned => owned.as_ref().as_bstr().as_bytes(),
    borrowed => borrowed.as_bstr().as_bytes()
);

// Keep these comparisons one-way: same-type reference equality is structural, so reverse
// implementations would let references with different targets form a non-transitive chain through their name.
impl_partial_eq_reference!(Reference, value => value.name.as_bstr());
impl_partial_eq_reference!(file::loose::Reference, value => value.name.as_bstr());
impl_partial_eq_reference!(packed::Reference<'_>, value => value.name.as_bstr());
