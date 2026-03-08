//! Configuration options for the directory walker.
//!
//! This module defines the [`Entry`] struct, which represents a single filesystem
//! node discovered during traversal. It is designed to provide efficient access
//! to file information by leveraging cached metadata whenever possible.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Represents a single directory entry found during the walk.
///
/// Contains the path, the recursion depth level, and an optional cached 
/// file type to optimize performance on POSIX systems.
#[derive(Debug, Clone)]
pub struct Entry {
    /// The full path to the filesystem entry.
    path: PathBuf,
    /// The depth of this entry relative to the starting directory.
    depth: usize,
    /// Cached file type information to avoid redundant syscalls.
    cached_ft: Option<fs::FileType>,
}

impl Entry {
    /// Creates a new `Entry` instance without cached file type information.
    ///
    /// # Arguments
    /// * `path` - The path of the discovered file or directory.
    /// * `depth` - The depth level of the entry (root is 0).
    ///
    /// # Returns
    /// A new `Entry` instance.
    pub fn new(path: PathBuf, depth: usize) -> Self {
        Self {
            path,
            depth,
            cached_ft: None,
        }
    }

    /// Creates a new `Entry` instance with pre-discovered file type information.
    ///
    /// This is an internal optimization used by the walker to pass information
    /// already obtained via `ReadDir`.
    ///
    /// # Arguments
    /// * `path` - The path of the discovered file or directory.
    /// * `depth` - The depth level of the entry.
    /// * `ft` - The pre-fetched `fs::FileType`.
    ///
    /// # Returns
    /// A new `Entry` instance with a populated cache.
    pub fn with_ft(path: PathBuf, depth: usize, ft: fs::FileType) -> Self {
        Self {
            path,
            depth,
            cached_ft: Some(ft),
        }
    }

    /// Returns the file type of this entry.
    ///
    /// # Performance
    /// If the file type was cached during the directory iteration (common in POSIX),
    /// this method returns immediately. Otherwise, it performs a `symlink_metadata` syscall.
    ///
    /// # Returns
    /// * `io::Result<fs::FileType>` - The file type information or an I/O error.
    pub fn file_type(&self) -> io::Result<fs::FileType> {
        if let Some(ft) = self.cached_ft {
            return Ok(ft);
        }
        fs::symlink_metadata(&self.path).map(|m| m.file_type())
    }

    /// Returns a reference to the entry's path.
    ///
    /// # Returns
    /// A reference to the underlying [`Path`].
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the depth of this entry relative to the root traversal directory.
    ///
    /// # Returns
    /// The depth level as a `usize`.
    pub fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the metadata for the file or directory.
    ///
    /// If the entry is a symbolic link, this method will follow it and return
    /// the metadata of the target file.
    ///
    /// # Returns
    /// * `io::Result<fs::Metadata>` - The metadata or an I/O error if the path is inaccessible.
    pub fn metadata(&self) -> io::Result<fs::Metadata> {
        fs::metadata(&self.path)
    }

    /// Returns the metadata for the entry without following symbolic links.
    ///
    /// # Returns
    /// * `io::Result<fs::Metadata>` - The symlink metadata or an I/O error.
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata> {
        fs::symlink_metadata(&self.path)
    }
}
