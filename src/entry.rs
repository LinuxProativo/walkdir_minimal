use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Entry {
    path: PathBuf,
    depth: usize,
    cached_ft: Option<fs::FileType>,
}

impl Entry {
    pub fn new(path: PathBuf, depth: usize) -> Self {
        Self {
            path,
            depth,
            cached_ft: None,
        }
    }

    pub fn with_ft(path: PathBuf, depth: usize, ft: fs::FileType) -> Self {
        Self {
            path,
            depth,
            cached_ft: Some(ft),
        }
    }

    pub fn file_type(&self) -> io::Result<fs::FileType> {
        if let Some(ft) = self.cached_ft {
            return Ok(ft);
        }
        fs::symlink_metadata(&self.path).map(|m| m.file_type())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn depth(&self) -> usize {
        self.depth
    }

    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(&self.path)
    }

    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(&self.path)
    }
}
