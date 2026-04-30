# dir2txt

`dir2txt` is a Rust-based utility and library designed to aggregate filesystem hierarchies into a single, structured text representation. It is optimized for scenarios requiring deterministic, high-fidelity context—such as feeding codebases into LLMs, generating system snapshots, or creating documentation manifests.

## ⚙️ Core Architecture

The project is built on a **Lazy-Loading, Heuristic-First** architecture. Unlike naive crawlers that read every file into memory, `dir2txt` follows a multi-stage ingestion pipeline to ensure performance on large-scale projects.

### The Ingestion Pipeline

1.  **Skeleton Mapping:** The directory tree is initially mapped without file I/O using metadata only.
2.  **Text Validation (1KB Probe):** The first 1024 bytes of each file are inspected for UTF-8 validity.
3.  **Deferred Loading:** Only files confirmed as text are read into memory.
4.  **Tree Pruning:** A recursive bottom-up pass removes binary files and collapses any directory branches that become empty as a result.
5.  **Deterministic Canonicalization:** All entries are sorted alphabetically to ensure identical outputs across different filesystem states.

## 🚀 Performance Features

* **Heuristic Binary Filtering:** Uses the `content-inspector` crate to detect non-text files (images, executables, compiled objects) before they can bloat memory.
* **Zero-Footprint Handling:** Employs `OnceLock` for thread-safe, single-assignment storage, ensuring content is never loaded twice.
* **Flattened Serialization:** A custom `Serialize` implementation skips struct metadata, producing a direct Key-Value map of paths to content.
* **Resource Resilience:** Explicitly handles `EINTR` (interrupted) system calls and provides graceful error handling for permission-denied paths.

## 📦 Technical Specifications

### Library Usage

```rust
use dir2txt::Directory;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new("./project_root");

    // Ingest tree structure
    let mut root = Directory::from_path(path, true)?;

    // Process and refine
    root.load_recursive(path)?; // 1KB probe + selective load
    root.prune(false);          // Remove binary nodes
    root.sort();                // Alpha-sort for determinism

    // Serialize to map-style JSON
    let output = serde_json::to_string(&root).unwrap();
    Ok(())
}
```

### Data Layout
The output is serialized as a nested map, where directory names are keys to child maps, and file names are keys to their string content.

```json
{
  "config.toml": "[settings]\nversion = 1",
  "src": {
    "main.rs": "fn main() { ... }",
    "lib.rs": "mod stuff;\nmod more;"
  },
  "tests": {
    "unit_tests.rs": "#[test]\nfn test() { ... }"
  }
}
```
