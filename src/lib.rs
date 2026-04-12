//! # walkdir_minimal
//!
//! A fast, minimalist, and POSIX-compliant recursive directory traverser with zero external dependencies.
//!
//! This crate provides an efficient way to iterate over directories, featuring:
//! - **Cycle Detection**: Prevents infinite recursion from circular symbolic links using device and inode IDs.
//! - **Performance**: Caches file types during traversal to avoid redundant and expensive syscalls.
//! - **Safety**: 100% safe Rust implementation with no `unsafe` blocks.
//! - **Flexibility**: Configurable recursion depth and optional error handling (ignore or report).
//!
//! # Security and Reliability
//! This library is designed with POSIX environments in mind. It uses standard Unix metadata 
//! to track visited directories, ensuring that even complex filesystem structures with 
//! nested symbolic links are handled safely without memory leaks or infinite loops.

mod entry;
mod error;
mod options;
mod walker;

/// Defines the error types that can occur during traversal, 
/// such as I/O failures or detected symbolic link loops.
pub use error::WalkError;

/// The main entry point for the library. An iterator that recursively 
/// yields directory entries based on the provided configuration.
pub use walker::WalkDir;

#[cfg(test)]
mod tests;
