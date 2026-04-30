use dir2txt::*;
use tempfile::tempdir;


#[test]
#[cfg(unix)] // Permission tests are most reliable on Unix
fn test_permission_denied_handling() -> std::io::Result<()> {
    use std::os::fs::PermissionsExt;

    let dir = tempdir()?;
    let secret_path = dir.path().join("secret.txt");
    fs::write(&secret_path, "can't see me")?;

    // Remove read permissions
    let mut perms = fs::metadata(&secret_path)?.permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&secret_path, perms)?;

    let file = LazyFile::new("secret.txt".to_string());
    let result = file.load_content(dir.path());

    // Should return an IO error, not panic
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::PermissionDenied);

    // Clean up permissions so tempdir can delete itself
    let mut restore = fs::metadata(&secret_path)?.permissions();
    restore.set_mode(0o644);
    fs::set_permissions(&secret_path, restore)?;
    Ok(())
}

#[test]
fn test_missing_file_during_lazy_load() {
    let dir = tempdir().unwrap();
    let file = LazyFile::new("ghost.txt".to_string());

    // We didn't actually create ghost.txt on disk
    let result = file.load_content(dir.path());

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn test_lazy_file_uninitialized_error() {
    let file = LazyFile::new("test.txt".to_string());
    // Serialization should fail because content is not loaded
    let result = serde_json::to_string(&file);
    assert!(result.is_err());
}
