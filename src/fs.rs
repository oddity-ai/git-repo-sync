use path_slash::PathExt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    /// Relative path.
    pub path: std::path::PathBuf,

    /// File size in bytes.
    pub size: u64,
}

impl File {
    pub fn new(path: std::path::PathBuf, size: u64) -> Self {
        File { path, size }
    }
}

impl std::fmt::Display for File {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} ({} bytes)", self.path.to_slash_lossy(), self.size)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directory {
    /// Relative path.
    pub path: std::path::PathBuf,
}

impl Directory {
    pub fn new(path: std::path::PathBuf) -> Self {
        Directory { path }
    }
}

impl std::fmt::Display for Directory {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.path.to_slash_lossy())
    }
}
