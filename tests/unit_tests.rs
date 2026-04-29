use dir2txt::*;
use tempfile::tempdir;
use std::fs::{self, File};
use std::io::Write;

mod common;

#[test]
fn test_lazy_file_detection() {
    let (_tmp, root) = common::setup_test_env();
    let bin_file = LazyFile::new("data.bin".to_string());

    let bin_result = bin_file.load_content(&root);

    assert!(bin_result.is_ok());
    assert_eq!(bin_file.get_is_text(), Some(false));
    assert!(bin_file.get_content().is_none());
}

#[test]
fn test_directory_recursion_and_sorting() -> std::io::Result<()> {
    let dir = tempdir()?;
    let sub_dir_path = dir.path().join("b_subdir");
    fs::create_dir(&sub_dir_path)?;

    File::create(dir.path().join("z_file.txt"))?.write_all(b"content")?;
    File::create(dir.path().join("a_file.txt"))?.write_all(b"content")?;
    File::create(sub_dir_path.join("sub_file.txt"))?.write_all(b"sub content")?;

    let mut directory = Directory::from_path(dir.path(), true)?;
    directory.sort();

    assert_eq!(directory.files[0].name, "a_file.txt");
    assert_eq!(directory.directories[0].name, "b_subdir");
    Ok(())
}

#[test]
fn test_pruning_logic() -> std::io::Result<()> {
    let dir = tempdir()?;
    File::create(dir.path().join("text.txt"))?.write_all(b"Valid text")?;
    File::create(dir.path().join("binary.dat"))?.write_all(&[0, 159, 146, 150])?;

    let mut directory = Directory::from_path(dir.path(), false)?;
    let _ = directory.load_recursive(dir.path());
    directory.prune(false);

    assert_eq!(directory.files.len(), 1);
    assert_eq!(directory.files[0].name, "text.txt");
    Ok(())
}
