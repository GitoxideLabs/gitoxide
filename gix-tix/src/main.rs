#![forbid(unsafe_code)]

use gix::Result;

fn main() -> Result<()> {
    gix_tix::command::parse().run()
}

#[test]
fn main_errors_include_caller_locations() {
    use gix::error::{ErrorExt, message};

    let error = message("command failed").raise();
    let source = file!().replace('\\', "/");
    assert!(
        format!("{error:?}").contains(&source),
        "standalone binary diagnostics include captured caller locations"
    );
    assert!(
        !format!("{error:#?}").contains(&source),
        "alternate formatting still omits caller locations"
    );
}
