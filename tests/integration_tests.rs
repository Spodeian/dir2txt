use dir2txt::*;
use std::io::Write;

mod common;

#[test]
fn test_full_pipeline_integration() {
    let (_tmp, root_path) = common::setup_test_env();

    let mut root_dir = Directory::from_path(&root_path, true)
        .expect("Failed to ingest directory");

    root_dir.load_recursive(&root_path).expect("Failed recursive load");
    root_dir.prune(false);
    root_dir.sort();

    let json_val = serde_json::to_value(&root_dir).expect("Serialization failed");

    assert_eq!(json_val["config.json"], "{\"version\": \"1.0\"}");
    assert!(json_val.get("data.bin").is_none());

    let src = json_val.get("src").expect("src directory missing");
    assert_eq!(src["main.rs"], "fn main() {}");
    assert!(src.get("assets").is_none(), "Empty dir should be pruned");
}

#[test]
fn test_slimmed_serialization() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let file_path = dir.path().join("info.txt");
    std::fs::File::create(&file_path)?.write_all(b"sensor_data: 42")?;

    let directory = Directory::from_path(dir.path(), false)?;
    let _ = directory.load_recursive(dir.path());

    let json_output = serde_json::to_string(&directory).unwrap();
    assert!(json_output.contains("\"info.txt\":\"sensor_data: 42\""));
    assert!(!json_output.contains("\"files\":"));
    Ok(())
}

#[test]
fn test_partial_lazy_loading() {
    let (_tmp, root_path) = common::setup_test_env();
    let root_dir = Directory::from_path(&root_path, true).unwrap();
    let config_file = root_dir.files.iter().find(|f| f.name == "config.json").unwrap();

    assert!(config_file.get_content().is_none());
    config_file.load_content(&root_path).unwrap();
    assert_eq!(config_file.get_content(), Some("{\"version\": \"1.0\"}"));
}
