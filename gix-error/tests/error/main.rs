mod error;
mod exn;

mod utils {
    use gix_error::{ErrorExt, Exn, Message, message};

    pub fn new_tree_error() -> Exn<Message> {
        let e1 = message("E1").raise();
        let e3 = e1.raise(message("E3"));

        let e9 = message("E9").raise();
        let e10 = e9.raise(message("E10"));

        let e11 = message("E11").raise();
        let e12 = e11.raise(message("E12"));

        let e5 = Exn::raise_all([e3, e10, e12], message("E5"));

        let e2 = message("E2").raise();
        let e4 = e2.raise(message("E4"));

        let e7 = message("E7").raise();
        let e8 = e7.raise(message("E8"));

        Exn::raise_all([e5, e4, e8], message("E6"))
    }

    pub fn debug_string(input: impl std::fmt::Debug) -> String {
        fixup_paths(format!("{input:?}"))
    }

    pub fn fixup_paths(input: String) -> String {
        if cfg!(windows) { input.replace('\\', "/") } else { input }
    }

    #[derive(Debug)]
    pub struct ErrorWithSource(pub &'static str, pub Message);

    impl std::fmt::Display for ErrorWithSource {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for ErrorWithSource {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.1)
        }
    }
}
pub use utils::*;
