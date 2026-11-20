use crate::Trust;

impl Trust {
    /// Derive `Full` trust if `path` is owned by the user executing the current process, or `Reduced` trust otherwise.
    pub fn from_path_ownership(path: &std::path::Path) -> std::io::Result<Self> {
        Ok(if crate::identity::is_path_owned_by_current_user(path)? {
            Trust::Full
        } else {
            Trust::Reduced
        })
    }
}
