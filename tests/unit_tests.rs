use dir2txt::*;


#[test]
fn test_lazy_file_manual_init() {
    let file = LazyFile::new("test.txt".to_string());
    file.set_content(Some("Hello World".to_string())).unwrap();

    let json = serde_json::to_string(&file).unwrap();
    assert_eq!(json, "\"Hello World\"");
}

#[test]
fn test_directory_flattened_serialization() {
    let mut root = Directory::new("root".to_string());

    let f1 = LazyFile::new("a.txt".to_string());
    f1.set_content(Some("content a".to_string())).unwrap();

    let mut sub = Directory::new("sub".to_string());
    let f2 = LazyFile::new("b.txt".to_string());
    f2.set_content(Some("content b".to_string())).unwrap();

    sub.files.push(f2);
    root.files.push(f1);
    root.directories.push(sub);

    let json = serde_json::to_string(&root).unwrap();
    // Expecting: {"a.txt":"content a","sub":{"b.txt":"content b"}}
    assert!(json.contains("\"a.txt\":\"content a\""));
    assert!(json.contains("\"sub\":{"));
}

#[test]
fn test_sorting_determinism() {
    let mut dir = Directory::new("root".to_string());
    dir.files.push(LazyFile::new("z.txt".to_string()));
    dir.files.push(LazyFile::new("a.txt".to_string()));
    dir.directories.push(Directory::new("beta".to_string()));
    dir.directories.push(Directory::new("alpha".to_string()));

    dir.sort();

    assert_eq!(dir.files[0].name, "a.txt");
    assert_eq!(dir.files[1].name, "z.txt");
    assert_eq!(dir.directories[0].name, "alpha");
    assert_eq!(dir.directories[1].name, "beta");
}

#[test]
fn test_empty_directory_serialization_fails() {
    let dir = Directory::new("empty".to_string());
    let result = serde_json::to_string(&dir);
    assert!(result.is_err(), "Empty directory should not serialize to avoid loss of context");
}
