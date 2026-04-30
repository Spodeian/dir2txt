use dir2txt::*;
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;


pub fn setup_test_env() -> (TempDir, std::path::PathBuf) {
    let tmp_dir = TempDir::new().expect("Failed to create temp dir");
    let root_path = tmp_dir.path().to_path_buf();
    let src = root_path.join("src");
    let assets = src.join("assets");

    fs::create_dir_all(&assets).unwrap();
    File::create(root_path.join("config.json")).unwrap()
        .write_all(b"{\"version\": \"1.0\"}").unwrap();
    // 0x00 ensures definitive binary detection
    File::create(root_path.join("data.bin")).unwrap()
        .write_all(&[0x00, 0xDE, 0xAD, 0xBE, 0xEF]).unwrap();
    File::create(src.join("main.rs")).unwrap()
        .write_all(b"fn main() {}").unwrap();
    File::create(assets.join("logo.png")).unwrap()
        .write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]).unwrap();

    (tmp_dir, root_path)
}

#[test]
fn test_slimmed_serialization() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let file_path = dir.path().join("info.txt");
    std::fs::File::create(&file_path)?.write_all(b"sensor_data: 42")?;

    let directory = Directory::from_path_slimmed(dir.path(), false)?;
    let _ = directory.load_recursive(dir.path());

    let json_output = serde_json::to_string(&directory).unwrap();
    assert!(json_output.contains("\"info.txt\":\"sensor_data: 42\""));
    assert!(!json_output.contains("\"files\":"));
    Ok(())
}

#[test]
fn test_partial_lazy_loading() {
    let (_tmp, root_path) = setup_test_env();
    let root_dir = Directory::from_path_slimmed(&root_path, true).unwrap();
    let config_file = root_dir.files.iter().find(|f| f.name == "config.json").unwrap();

    assert!(config_file.get_content().is_none());
    config_file.load_content(&root_path).unwrap();
    assert_eq!(config_file.get_content(), Some("{\"version\": \"1.0\"}"));
}

#[test]
fn test_full_pipeline_determinism() {
    let (_tmp, root_path) = setup_test_env();

    // The key change: Ingest the root_path
    let mut root_dir = Directory::from_path_slimmed(&root_path, true)
        .expect("Failed to ingest directory");

    // The load_recursive needs the path to the folder ABOVE the root_dir.name
    // Or we modify the root_dir name to empty/dot for the root.
    let base_parent = root_path.parent().unwrap();

    root_dir.load_recursive(base_parent).expect("Failed recursive load");
    root_dir.prune(false);
    root_dir.sort();

    let json_val = serde_json::to_value(&root_dir).expect("Serialization failed");

    // Debug print if it fails again to see the structure:
    // println!("{}", serde_json::to_string_pretty(&json_val).unwrap());

    assert_eq!(json_val["config.json"], "{\"version\": \"1.0\"}");
    assert_eq!(json_val["src"]["main.rs"], "fn main() {}");

    assert!(json_val.get("data.bin").is_none());
    // Since 'assets' only had 'logo.png' (binary), it should be gone:
    assert!(json_val["src"].get("assets").is_none());
}

#[test]
fn test_lazy_file_detection() {
    let (_tmp, root) = setup_test_env();
    let bin_file = LazyFile::new("data.bin".to_string());

    let bin_result = bin_file.load_content(&root);

    assert!(bin_result.is_ok());
    assert_eq!(bin_file.get_is_text(), Some(false));
    assert!(bin_file.get_content().is_none());
}
