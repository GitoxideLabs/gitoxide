use std::pin::Pin;

use gix_macros::momo;

struct Options;

#[allow(dead_code)]
fn test_open_opts_inner(_dir: impl Into<std::path::PathBuf>, _options: Options) -> Result<(), ()> {
    Ok(())
}

/// See if doc are kept
#[allow(dead_code)]
#[momo]
fn test_open_opts(directory: impl Into<std::path::PathBuf>, options: Options) -> Result<(), ()> {
    test_open_opts_inner(directory, options)
}

#[momo]
fn test_fn<E>(
    a: impl Into<String>,
    b: impl AsRef<str>,
    mut c: impl AsMut<str>,
    d: impl TryInto<String, Error = E>,
) -> Result<String, E> {
    let mut s = a.into();
    s += b.as_ref();
    s += c.as_mut();
    s += &d.try_into()?;

    Ok(s)
}

#[momo]
fn test_fn_call_style<E>(
    a: impl Into<String>,
    b: impl AsRef<str>,
    mut c: impl AsMut<str>,
    d: impl TryInto<String, Error = E>,
) -> Result<String, E> {
    let mut s = Into::into(a);
    s += AsRef::as_ref(&b);
    s += AsMut::as_mut(&mut c);
    s += &TryInto::try_into(d)?;

    Ok(s)
}

#[momo]
fn test_fn_where<A, B, C, D, E>(a: A, b: B, mut c: C, d: D) -> Result<String, E>
where
    A: Into<String>,
    B: AsRef<str>,
    C: AsMut<str>,
    D: TryInto<String, Error = E>,
{
    let mut s = a.into();
    s += b.as_ref();
    s += c.as_mut();
    s += &d.try_into()?;

    Ok(s)
}

struct TestStruct;

impl TestStruct {
    #[momo]
    fn test_method<E>(
        self,
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
    ) -> Result<String, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        Ok(s)
    }

    #[allow(clippy::needless_arbitrary_self_type)]
    #[momo]
    fn test_method2<E>(
        self: Self,
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
    ) -> Result<String, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        Ok(s)
    }

    #[momo]
    fn test_fn<E>(
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
        _e: (),
    ) -> Result<String, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        Ok(s)
    }

    #[momo]
    fn test_fn2<E>(
        _this: Self,
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
        _e: (),
        _f: (),
    ) -> Result<String, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        Ok(s)
    }

    #[momo]
    fn test_fn3<E>(
        _this: Pin<&mut Self>,
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
        _e: (),
        _f: (),
    ) -> Result<String, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        Ok(s)
    }

    #[allow(unused)]
    #[momo]
    fn test_fn_ret<E>(
        _this: Pin<&mut Self>,
        a: impl Into<String>,
        b: impl AsRef<str>,
        mut c: impl AsMut<str>,
        d: impl TryInto<String, Error = E>,
        _e: (),
        _f: (),
    ) -> Result<Self, E> {
        let mut s = a.into();
        s += b.as_ref();
        s += c.as_mut();
        s += &d.try_into()?;

        drop(s);

        Ok(Self)
    }
}

struct S(bool);
impl TryInto<String> for S {
    type Error = ();

    fn try_into(self) -> Result<String, ()> {
        if self.0 { Ok(String::from("!2345")) } else { Err(()) }
    }
}

#[allow(unused)]
#[momo]
fn test_fn_pat<E>(
    a: impl Into<String>,
    b: impl AsRef<str>,
    mut c: impl AsMut<str>,
    d: impl TryInto<String, Error = E>,
    S(_g): S,
) -> Result<(), E> {
    let mut s = a.into();
    s += b.as_ref();
    s += c.as_mut();
    s += &d.try_into()?;

    drop(s);

    Ok(())
}

#[test]
fn basic_fn() {
    assert_eq!(
        test_fn("12345", "12345", String::from("12345"), S(true)).unwrap(),
        "123451234512345!2345"
    );

    test_fn("12345", "12345", String::from("12345"), S(false)).unwrap_err();

    assert_eq!(
        test_fn_call_style("12345", "12345", String::from("12345"), S(true)).unwrap(),
        "123451234512345!2345"
    );

    test_fn_call_style("12345", "12345", String::from("12345"), S(false)).unwrap_err();

    assert_eq!(
        test_fn_where("12345", "12345", String::from("12345"), S(true)).unwrap(),
        "123451234512345!2345"
    );

    test_fn_where("12345", "12345", String::from("12345"), S(false)).unwrap_err();
}

#[test]
fn struct_method() {
    assert_eq!(
        TestStruct
            .test_method("12345", "12345", String::from("12345"), S(true))
            .unwrap(),
        "123451234512345!2345"
    );

    TestStruct
        .test_method("12345", "12345", String::from("12345"), S(false))
        .unwrap_err();

    // Test test_method2
    assert_eq!(
        TestStruct
            .test_method2("12345", "12345", String::from("12345"), S(true))
            .unwrap(),
        "123451234512345!2345"
    );

    TestStruct
        .test_method2("12345", "12345", String::from("12345"), S(false))
        .unwrap_err();
}

#[test]
fn struct_fn() {
    assert_eq!(
        TestStruct::test_fn("12345", "12345", String::from("12345"), S(true), ()).unwrap(),
        "123451234512345!2345"
    );

    TestStruct::test_fn("12345", "12345", String::from("12345"), S(false), ()).unwrap_err();

    assert_eq!(
        TestStruct::test_fn2(TestStruct, "12345", "12345", String::from("12345"), S(true), (), ()).unwrap(),
        "123451234512345!2345"
    );

    TestStruct::test_fn2(TestStruct, "12345", "12345", String::from("12345"), S(false), (), ()).unwrap_err();

    assert_eq!(
        TestStruct::test_fn3(
            Pin::new(&mut TestStruct),
            "12345",
            "12345",
            String::from("12345"),
            S(true),
            (),
            ()
        )
        .unwrap(),
        "123451234512345!2345"
    );

    TestStruct::test_fn3(
        Pin::new(&mut TestStruct),
        "12345",
        "12345",
        String::from("12345"),
        S(false),
        (),
        (),
    )
    .unwrap_err();
}

#[test]
fn ux() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/momo/ux/*.rs");
}
