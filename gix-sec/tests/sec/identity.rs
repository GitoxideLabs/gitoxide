#[test]
fn is_path_owned_by_current_user() -> crate::Result {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("file");
    std::fs::write(&file, [])?;
    assert!(gix_sec::identity::is_path_owned_by_current_user(&file)?);
    assert!(gix_sec::identity::is_path_owned_by_current_user(dir.path())?);
    Ok(())
}

#[test]
fn is_path_owned_by_current_user_nonexistent() {
    let nonexistent = std::path::Path::new("/this/path/does/not/exist");
    let result = gix_sec::identity::is_path_owned_by_current_user(nonexistent);
    assert!(result.is_err(), "Should fail for nonexistent paths");
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
#[cfg(windows)]
fn windows_home() -> crate::Result {
    let home = gix_path::env::home_dir().expect("home dir is available");
    assert!(gix_sec::identity::is_path_owned_by_current_user(&home)?);
    Ok(())
}

#[test]
fn test_trust_behavior_scenarios() {
    // Test 1: Current directory (should work)
    let current_dir = std::env::current_dir().unwrap();
    assert!(current_dir.exists(), "Current directory should exist: {:?}", current_dir);
    
    let ownership_result = gix_sec::identity::is_path_owned_by_current_user(&current_dir);
    assert!(ownership_result.is_ok(), "Should be able to check ownership of current directory: {:?}", ownership_result);
    

}
