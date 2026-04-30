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
    assert_eq!(file.get_content().unwrap().chars().count(), 2048 + 11);
    Ok(())
}

#[test]
fn test_nested_empty_directories_pruning() -> std::io::Result<()> {
    let dir = tempdir()?;
    // path: root/level1/level2/level3 (all empty)
    let deep_path = dir.path().join("level1/level2/level3");
    fs::create_dir_all(&deep_path)?;

    let mut root_dir = Directory::from_path_slimmed(dir.path(), true)?;

    // This should return false because the entire tree is empty
    let stays = root_dir.prune(false);

    assert!(!stays, "The root directory should be pruned if it contains nothing");
    Ok(())
}

#[test]
fn test_recursive_binary_pruning_collapse() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let root_path = dir.path();

    // Structure: root/sub/only_binary.dat
    let sub_path = root_path.join("sub");
    fs::create_dir(&sub_path)?;
    let bin_path = sub_path.join("only_binary.dat");
    fs::File::create(bin_path)?.write_all(&[0x00, 0x80, 0xFF])?;

    // Ingest lazily
    let mut tree = Directory::from_path_slimmed(root_path, true)?;

    // Trigger detection and pruning
    tree.load_recursive(root_path.parent().unwrap())?;
    let stays = tree.prune(false);

    // Assertions: sub was only binary, so sub is pruned.
    // Since root then becomes empty, root should also be pruned.
    assert!(!stays, "The tree should collapse if all leaf nodes are binary");
    assert!(tree.is_empty());
    Ok(())
}

#[test]
fn test_pruning_logic() -> std::io::Result<()> {
    let dir = tempdir()?;
    fs::File::create(dir.path().join("text.txt"))?.write_all(b"Valid text")?;
    fs::File::create(dir.path().join("binary.dat"))?.write_all(&[0, 159, 146, 150])?;

    let mut directory = Directory::from_path_slimmed(dir.path(), false)?;
    let _ = directory.load_recursive(dir.path());
    directory.prune(false);

    assert_eq!(directory.files.len(), 1);
    assert_eq!(directory.files[0].name, "text.txt");
    Ok(())
}
