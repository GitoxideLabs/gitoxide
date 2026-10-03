use gix_features::trace::{coarse, detail, span};

#[cfg(feature = "tracing")]
mod parallel;

#[test]
fn span() {
    let _x = span!(gix_features::trace::Level::Coarse, "hello");
    span!(gix_features::trace::Level::Coarse, "hello", x = "value", y = 42);
    span!(target: "other", gix_features::trace::Level::Coarse, "hello", x = "value", y = 42);

    let _x = coarse!("hello");
    coarse!("hello", x = "value", y = 42);
    coarse!(target: "other", "hello", x = "value", y = 42);

    let _y = detail!("hello");
    detail!("hello", x = "value", y = 42);
    detail!(target: "other", "hello", x = "value", y = 42);
}
