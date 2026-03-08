//! Error types for directory traversal operations.
//!
//! This module defines the [`WalkError`] enum, which encapsulates all possible
//! failure states that can occur while walking a directory tree, including
//! standard operating system errors and logical filesystem violations like infinite loops.

use std::{fmt, io, path::PathBuf};

/// Errors that can occur during a directory traversal.
///
/// This enum categorizes failures into low-level system I/O issues and
/// high-level recursive logic violations.
#[derive(Debug)]
pub enum WalkError {
    /// A wrapper for standard library I/O errors.
    /// Triggered by permission issues, missing files, or hardware failures.
    Io(io::Error),
    /// Emitted when a symbolic link cycle is detected.
    /// This prevents the walker from entering an infinite recursion.
    LoopDetected(PathBuf),
}

impl From<io::Error> for WalkError {
    /// Converts a standard `std::io::Error` into a `WalkError`.
    ///
    /// # Arguments
    /// * `e` - The original I/O error from the standard library.
    ///
    /// # Returns
    /// A `WalkError::Io` variant containing the provided error.
    fn from(e: io::Error) -> Self {
        WalkError::Io(e)
    }
}

impl fmt::Display for WalkError {
    /// Formats the error for human-readable output.
    ///
    /// # Arguments
    /// * `f` - The formatter context.
    ///
    /// # Returns
    /// A fmt::Result indicating success or failure of the write operation.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalkError::Io(e) => write!(f, "IO error: {}", e),
            WalkError::LoopDetected(p) => {
                write!(f, "Symbolic link loop detected at {}", p.display())
            }
        }
    }
}

/// Integration with the standard Error trait.
///
/// This implementation allows [`WalkError`] to be used with error handling crates
/// (like `anyhow` or `thiserror`) and ensures compatibility with the broader
/// Rust error ecosystem.
impl std::error::Error for WalkError {}
