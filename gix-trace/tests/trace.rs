use gix_trace::{coarse, debug, detail, error, event, info, span, trace, warn};

#[test]
fn in_thread_returns_the_closure_output() {
    let mut value = 41;
    let result = gix_trace::in_thread(|| {
        value += 1;
        value
    })();
    assert_eq!(result, 42, "the wrapper supports borrowed data and returns the result");
}

#[test]
fn span() {
    let _x = span!(gix_trace::Level::Coarse, "hello");
    let forty_two = span!(gix_trace::Level::Coarse, "hello", x = "value", y = 42).into_scope(|| 42);
    assert_eq!(forty_two, 42);
    let span = span!(target: "other", gix_trace::Level::Coarse, "hello", x = "value", y = 42);
    span.record("y", "hello").record("x", 36);
}

#[test]
fn coarse() {
    let _x = coarse!("hello");
    coarse!("hello", x = "value", y = 42);
    coarse!(target: "other", "hello", x = "value", y = 42).into_scope(|| {
        event!(gix_trace::event::Level::ERROR, "an error");
        event!(gix_trace::event::Level::WARN, "an info: {}", 42);
        event!(gix_trace::event::Level::INFO, answer = 42, field = "some");
        #[derive(Debug)]
        struct User {
            name: &'static str,
            email: &'static str,
        }
        let user = User {
            name: "ferris",
            email: "ferris@example.com",
        };
        event!(gix_trace::event::Level::DEBUG, user.name, user.email);
        event!(gix_trace::event::Level::TRACE, greeting = ?user, display = %user.name);

        error!("hello {}", 42);
        warn!("hello {}", 42);
        info!("hello {}", 42);
        debug!("hello {}", 42);
        trace!("hello {}", 42);
    });
}

#[test]
fn detail() {
    let _y = detail!("hello");
    detail!("hello", x = "value", y = 42);
    detail!(target: "other", "hello", x = "value", y = 42);
}
