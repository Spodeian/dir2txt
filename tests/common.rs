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
