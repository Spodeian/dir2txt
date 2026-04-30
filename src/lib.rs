use std::{io::Read, sync::OnceLock};
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Serialize, Serializer};
use content_inspector::{inspect, ContentType};

/// Reads bytes from a source into a buffer until the buffer is full or EOF is reached.
///
/// Unlike [`std::io::Read::read_exact`], this function handles cases where the source
/// has fewer bytes remaining than the buffer size without returning an error.
///
/// # Errors
/// Returns an [`std::io::Error`] if a physical read failure occurs. It transparently
/// handles and retries on [`std::io::ErrorKind::Interrupted`].
fn read_to_fill_or_eof<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<usize, std::io::Error> {
    let mut total = 0;
    while total < buf.len() {
        // Attempt to read into the remaining slice
        match reader.read(&mut buf[total..]) {
            Ok(0) => break, // EOF reached
            Ok(n) => total += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {} // Retry on EINTR
            Err(e) => Err(e)?,
        }
    }

    Ok(total)
}

/// A lazily-loaded file representation that defers content reading until explicitly requested.
///
/// Uses `OnceLock` to ensure thread-safe, single-assignment initialization of content
/// and file-type detection. This is optimized for high-volume directory crawling where
/// many files (like binaries) should be skipped before loading into memory.
#[derive(Clone, Debug, Default)]
pub struct LazyFile {
    /// The base name of the file.
    pub name: String,
    /// Cached detection of whether the file is valid UTF-8 text.
    is_text: OnceLock<bool>,
    /// Cached string content of the file.
    content: OnceLock<Option<String>>,
}

impl LazyFile {
    /// Creates a new `LazyFile` instance. Initialization of data remains deferred.
    pub fn new(name: String) -> Self {
        Self {
            name,
            is_text: OnceLock::new(),
            content: OnceLock::new(),
        }
    }

    /// Computes the full path of the file given a parent directory.
    pub fn path<P: AsRef<Path>>(&self, parent_path: P) -> PathBuf {
        parent_path.as_ref().join(&self.name)
    }

    /// Reads the first 1024 bytes to determine if the file is UTF-8 text.
    ///
    /// The detection uses a heuristic based on the presence of null bytes or invalid
    /// UTF-8 sequences in the initial buffer.
    ///
    /// # Return
    /// Returns a tuple containing the number of bytes read and the buffer used,
    /// allowing for zero-copy reuse of the header for small files.
    ///
    /// # Errors
    /// Returns an error if the `is_text` state was previously initialized to a
    /// different value, ensuring internal consistency.
    pub fn set_is_text<R: Read>(&self, reader: &mut R) -> Result<(usize, [u8; 1024]), std::io::Error> {
        let mut buf = [0u8; 1024];
        let bytes_read = read_to_fill_or_eof(reader, buf.as_mut_slice())?;
        let detection = inspect(&buf[..bytes_read]);
        let is_text = detection == ContentType::UTF_8;

        if self.is_text.get_or_init(|| is_text).to_owned() == is_text {
            Ok((bytes_read, buf))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "is_text state mismatch"))
        }
    }

    /// Returns the cached text status, or `None` if detection hasn't occurred.
    pub fn get_is_text(&self) -> Option<bool>    {
        self.is_text.get().copied()
    }

    /// Loads the file content into memory.
    ///
    /// This method is idempotent; if content is already loaded, it returns `Ok(())`.
    /// Binary files are skipped gracefully
    pub fn load_content<P: AsRef<Path>>(&self, parent_path: P) -> Result<(), std::io::Error> {
        if self.content.get().is_some() {
            return Ok(());
        }

        if let Some(false) = self.get_is_text() {
            if self.content.get().is_none() {
                let _ = self.set_content(None);
            }

            return Ok(());
        }

        let full_path = self.path(parent_path);
        let mut file = fs::File::open(&full_path)?;

        let (bytes_read, buf) = self.set_is_text(&mut file)?;

        match self.get_is_text() {
            Some(true) => {
                let mut reader = std::io::BufReader::new(&mut file);
                let mut full_buf: Vec<u8> = buf[..bytes_read].to_vec();
                reader.read_to_end(&mut full_buf)?;

                self.set_content(Some(String::from_utf8_lossy(&full_buf).to_string()))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
                Ok(())
            },
            Some(false) => {
                if self.content.get().is_none() {
                    let _ = self.set_content(None);
                }

                Ok(())
            },
            None => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to update is_text"))
            },
        }
    }

    /// Manually sets the content of the file.
    ///
    /// # Errors
    /// Returns an error if the content or the `is_text` status has already been initialized.
    pub fn set_content(&self, content: Option<String>) -> Result<(), &'static str> {
        let is_some = content.is_some();
        if *self.is_text.get_or_init(|| is_some) != is_some {
            return Err("is_text already set differently");
        } else {
            self.content.set(content).map_err(|_| "Content already set")?;
        }

        Ok(())
    }

    /// Returns a reference to the loaded content if available.
    pub fn get_content(&self) -> Option<&str> {
        self.content.get().and_then(|opt| opt.as_deref())
    }

    /// Returns the content if initialized, otherwise initializes it via the provided closure.
    pub fn get_or_init_content(&self, content_fn: impl FnOnce() -> Option<String>) -> Option<&str> {
        self.content.get_or_init(content_fn).as_deref()
    }

    /// Checks if the content is currently loaded in memory.
    pub fn is_text_ready(&self) -> bool {
        self.get_content().is_some()
    }

    /// Evaluates if the file should be kept during a pruning pass.
    ///
    /// If detection hasn't run, it uses the `default` value.
    pub fn prune(&self, default: bool) -> bool {
        *self.is_text.get().unwrap_or(&default)
    }
}

impl Serialize for LazyFile {
    /// Serializes the file content as a string.
    ///
    /// # Errors
    /// Fails if the file is binary or has not been loaded, as these states
    /// cannot be represented in the final flattened output.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(content) = self.get_content() {
            serializer.serialize_str(content)
        } else if let Some(&false) = self.is_text.get() {
            Err(serde::ser::Error::custom("File is not text."))
        } else {
            Err(serde::ser::Error::custom("Not initialized."))
        }
    }
}

/// A recursive directory structure containing files and subdirectories.
#[derive(Clone, Debug, Default)]
pub struct Directory {
    /// The name of the directory.
    pub name: String,
    /// Collection of files within this directory.
    pub files: Vec<LazyFile>,
    /// Collection of nested subdirectories.
    pub directories: Vec<Directory>,
}

impl Directory {
    /// Creates a new empty `Directory`.
    pub fn new(name: String) -> Self {
        Self {
            name,
            files: Vec::new(),
            directories: Vec::new(),
        }
    }

    /// Computes the full path of this directory relative to a parent.
    pub fn path<P: AsRef<Path>>(&self, parent_path: P) -> PathBuf {
        parent_path.as_ref().join(&self.name)
    }

    /// Ingests a filesystem path into and creates a slimmed tree structure
    /// representing the directory and its contents, but ignoring non-utf8
    /// files and empty directories.
    ///
    /// If `lazy` is true, file contents are not loaded during ingestion.
    pub fn from_path_slimmed<P: AsRef<Path>>(path: P, lazy: bool) -> std::io::Result<Self> {
        let path = path.as_ref();
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "root".to_string());

        let mut current_dir = Directory::new(name);

        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let Ok(entry) = entry else { continue };
                let entry_type = entry.file_type()?;

                if entry_type.is_dir() {
                    let dir = Self::from_path_slimmed(entry.path(), lazy)?;
                    if !dir.is_empty() { current_dir.directories.push(dir) }
                } else if entry_type.is_file() {
                    let file = LazyFile::new(entry.file_name().to_string_lossy().into_owned());

                    if !lazy {
                        // Tell the file to load itself using the current path context
                        file.load_content(path)?;
                    } else {
                        let mut reader = fs::File::open(entry.path())?;
                        file.set_is_text(&mut reader)?;
                    }

                    if file.get_is_text().unwrap_or(true) {
                        current_dir.files.push(file);
                    }
                }
            }
        } else {
            return Err(std::io::Error::new(std::io::ErrorKind::NotFound, "Path is not a directory"));
        }

        Ok(current_dir)
    }

    /// Recursively triggers `load_content` for all files in the tree.
    pub fn load_recursive<P: AsRef<Path>>(&self, parent_path: P) -> std::io::Result<()> {
        let path = self.path(parent_path.as_ref());
        for file in &self.files {
            file.load_content(&path)?;
        }
        // Recurse into subdirectories
        for dir in &self.directories {
            dir.load_recursive(&path)?;
        }

        Ok(())
    }

    /// Loads content for files only in the immediate directory level.
    pub fn load_local_files<P: AsRef<Path>>(&self, parent_path: P) -> std::io::Result<()> {
        let path = self.path(parent_path);
        for file in &self.files {
            file.load_content(&path)?;
        }

        Ok(())
    }

    /// Removes files and subdirectories based on text detection results.
    ///
    /// This performs a **recursive bottom-up collapse**:
    /// 1. Files identified as non-text (binary) are removed.
    /// 2. Subdirectories are pruned.
    /// 3. If a subdirectory becomes empty after its own pruning, it is also removed.
    ///
    /// Returns `true` if this directory still contains at least one text file
    /// or a non-empty subdirectory.
    pub fn prune(&mut self, default: bool) -> bool {
        self.files.retain(|f| f.prune(default));
        self.directories.retain_mut(|dir| dir.prune(default));
        !self.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.directories.is_empty() && self.files.is_empty()
    }

    /// Alphabetically sorts files and subdirectories.
    ///
    /// This ensures that the generated output (e.g., JSON or Text) is **deterministic**,
    /// which is essential for version control diffs and reproducible analysis.
    pub fn sort(&mut self) {
        self.files.sort_by(|a, b| a.name.cmp(&b.name));
        self.directories.sort_by(|a, b| a.name.cmp(&b.name));
        for dir in &mut self.directories {
            dir.sort();
        }
    }
}

impl Serialize for Directory {
    /// Serializes the directory tree into a flattened JSON map.
    ///
    /// # Layout
    /// Instead of a fixed schema, this produces a dynamic map where:
    /// - Keys are file or directory names.
    /// - Values are either the file content string or a nested map.
    ///
    /// # Errors
    /// Serialization fails if the directory is empty, as an empty map provides
    /// no context for downstream consumers.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let len = self.files.len() + self.directories.len();
        if len == 0 {
            return Err(serde::ser::Error::custom(format!(
                "Directory '{}' is empty and cannot be serialized.",
                self.name
            )));
        }

        let mut map = serializer.serialize_map(Some(len))?;

        for file in &self.files {
            map.serialize_entry(&file.name, file)?;
        }

        for dir in &self.directories {
            map.serialize_entry(&dir.name, dir)?;
        }

        map.end()
    }
}
