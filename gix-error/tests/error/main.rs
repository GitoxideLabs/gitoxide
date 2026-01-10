mod error;
mod exn;

mod utils {
    use gix_error::{ErrorExt, Exn};

    pub fn new_tree_error() -> Exn<Error> {
        let e1 = Error("E1").raise();
        let e3 = e1.raise(Error("E3"));

        let e9 = Error("E9").raise();
        let e10 = e9.raise(Error("E10"));

        let e11 = Error("E11").raise();
        let e12 = e11.raise(Error("E12"));

        let e5 = Exn::from_iter([e3, e10, e12], Error("E5"));

        let e2 = Error("E2").raise();
        let e4 = e2.raise(Error("E4"));

        let e7 = Error("E7").raise();
        let e8 = e7.raise(Error("E8"));

        Exn::from_iter([e5, e4, e8], Error("E6"))
    }

    #[derive(Debug)]
    pub struct Error(pub &'static str);

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for Error {}

    pub fn debug_string(input: impl std::fmt::Debug) -> String {
        if cfg!(windows) {
            let out = format!("{input:?}");
            out.replace('\\', "/")
        } else {
            format!("{input:?}")
        }
    }

    #[derive(Debug)]
    pub struct ErrorWithSource(pub &'static str, pub Error);

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
