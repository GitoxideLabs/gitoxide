pub use tracing::field;

#[doc(hidden)]
pub use tracing as __tracing;

/// An entered span which will exit on drop.
pub struct Span {
    inner: Option<tracing::span::EnteredSpan>,
}

impl Clone for Span {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.as_ref().map(|s| tracing::Span::clone(s).entered()),
        }
    }
}

impl Span {
    /// Create a disabled span.
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    /// Wrap an entered span from the `tracing` crate.
    pub fn from_entered(span: tracing::span::EnteredSpan) -> Self {
        Self { inner: Some(span) }
    }

    /// Record a single `field` to take `value`.
    ///
    /// ### Panics
    ///
    /// If the field name wasn't mentioned when the span was created.
    pub fn record<V>(&self, field: &str, value: V) -> &Self
    where
        V: field::Value,
    {
        if let Some(inner) = &self.inner {
            inner.record(field, value);
        }
        self
    }
}

#[doc(hidden)]
impl crate::Level {
    pub const fn into_tracing_level(self) -> tracing::Level {
        match self {
            crate::Level::Coarse => tracing::Level::INFO,
            crate::Level::Detail => tracing::Level::DEBUG,
        }
    }
}

/// A macro to create a span.
#[macro_export]
macro_rules! span {
    (target: $target:expr, $lvl:expr, $name:expr, $($fields:tt)*) => {{
        if $lvl > $crate::MAX_LEVEL {
            $crate::Span::disabled()
        } else {
            $crate::Span::from_entered(
                $crate::__tracing::span!(target: $target, $lvl.into_tracing_level(), $name, $($fields)*).entered()
            )
        }
    }};
    (target: $target:expr, $lvl:expr, $name:expr) => {
        $crate::span!(target: $target, $lvl, $name,)
    };
    ($lvl:expr, $name:expr, $($fields:tt)*) => {
        $crate::span!(
            target: module_path!(),
            $lvl,
            $name,
            $($fields)*
        )
    };
    ($lvl:expr, $name:expr) => {
        $crate::span!(
            target: module_path!(),
            $lvl,
            $name,
        )
    };
}

/// Create an event with the given level.
#[macro_export]
macro_rules! event {
    (target: $target:expr, $lvl:expr, { $($fields:tt)* } )=> (
        $crate::__tracing::event!(target: $target, $lvl, $($fields)*)
    );
    (target: $target:expr, $lvl:expr, { $($fields:tt)* }, $($arg:tt)+ ) => (
        $crate::event!(
            target: $target,
            $lvl,
            { message = format_args!($($arg)+), $($fields)* }
        )
    );
    (target: $target:expr, $lvl:expr, $($k:ident).+ = $($fields:tt)* ) => (
        $crate::event!(target: $target, $lvl, { $($k).+ = $($fields)* })
    );
    (target: $target:expr, $lvl:expr, $($arg:tt)+ ) => (
        $crate::event!(target: $target, $lvl, { $($arg)+ })
    );
    ( $lvl:expr, { $($fields:tt)* }, $($arg:tt)+ ) => (
        $crate::event!(
            target: module_path!(),
            $lvl,
            { message = format_args!($($arg)+), $($fields)* }
        )
    );
    ($lvl:expr, $($k:ident).+ = $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $lvl,
            { $($k).+ = $($field)*}
        )
    );
    ($lvl:expr, $($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $lvl,
            { $($k).+, $($field)*}
        )
    );
    ($lvl:expr, ?$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $lvl,
            { ?$($k).+, $($field)*}
        )
    );
    ($lvl:expr, %$($k:ident).+, $($field:tt)*) => (
        $crate::event!(
            target: module_path!(),
            $lvl,
            { %$($k).+, $($field)*}
        )
    );
    ($lvl:expr, ?$($k:ident).+) => (
        $crate::event!($lvl, ?$($k).+,)
    );
    ($lvl:expr, %$($k:ident).+) => (
        $crate::event!($lvl, %$($k).+,)
    );
    ($lvl:expr, $($k:ident).+) => (
        $crate::event!($lvl, $($k).+,)
    );
    ( $lvl:expr, $($arg:tt)+ ) => (
        $crate::event!(target: module_path!(), $lvl, { $($arg)+ })
    );
}
