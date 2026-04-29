use dir2txt::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::tempdir;

#[test]
fn test_empty_file_is_valid_text() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("empty.txt");
    File::create(&path)?; // 0-byte file

    let file = LazyFile::new("empty.txt".to_string());
    file.load_content(dir.path())?;

    // Most inspectors treat 0-byte files as empty UTF-8 text
    assert_eq!(file.get_is_text(), Some(true));
    assert_eq!(file.get_content(), Some(""));
    Ok(())
}

#[test]
fn test_large_file_buffer_overflow() -> std::io::Result<()> {
    let dir = tempdir()?;
    let path = dir.path().join("large.txt");

    // Create a file larger than the 1024-byte detection buffer
    let mut big_data = vec![b'a'; 2048];
    big_data.extend_from_slice(b"end_of_file");
    File::create(&path)?.write_all(&big_data)?;

    let file = LazyFile::new("large.txt".to_string());
    file.load_content(dir.path())?;

    assert!(file.get_content().unwrap().contains("end_of_file"));
    assert_eq!(file.get_content().unwrap().len(), 2048 + 11);
    Ok(())
}

#[test]
fn test_nested_empty_directories_pruning() -> std::io::Result<()> {
    let dir = tempdir()?;
    // path: root/level1/level2/level3 (all empty)
    let deep_path = dir.path().join("level1/level2/level3");
    fs::create_dir_all(&deep_path)?;

    let mut root_dir = Directory::from_path(dir.path(), true)?;

    // This should return false because the entire tree is empty
    let stays = root_dir.prune(false);

    assert!(!stays, "The root directory should be pruned if it contains nothing");
    Ok(())
}
