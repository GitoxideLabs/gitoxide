pub type Result<T = ()> = std::result::Result<T, gix_error::TestError>;

mod convert;
mod realpath;
mod relative_path;
mod home_dir {
    #[test]
    fn returns_existing_directory() {
        if let Some(home) = gix_path::env::home_dir() {
            assert!(
                home.is_dir(),
                "the home directory would typically exist, even though on unix we don't test for that."
            );
        }
    }
}

mod env;
mod util;
