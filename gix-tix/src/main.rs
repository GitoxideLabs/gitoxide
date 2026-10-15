#![forbid(unsafe_code)]

use gix::Result;

fn main() -> Result<()> {
    let _interrupt = gix::interrupt::init_handler_default(1)?.auto_deregister();
    gix_tix::command::parse().run()
}

#[test]
fn main_errors_include_caller_locations() {
    use gix::error::{ErrorExt, message};

    let error = message("command failed").raise();
    assert!(
        format!("{error:?}").contains(file!()),
        "standalone binary diagnostics include captured caller locations"
    );
    assert!(
        !format!("{error:#?}").contains(file!()),
        "alternate formatting still omits caller locations"
    );
}
