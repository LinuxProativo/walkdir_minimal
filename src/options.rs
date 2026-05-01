//! Configuration options for the directory walker.
//!
//! This module defines the [`WalkOptions`] struct, which centralizes all 
//! tunable parameters for the traversal logic, such as recursion limits 
//! and error handling policies.

/// Configuration structure to customize the behavior of the directory walker.
///
/// This struct allows the user to define how the walker should handle 
/// symbolic links, how deep the recursion should go, and whether it 
/// should halt on errors.
#[derive(Clone, Debug)]
pub struct WalkOptions {
    /// Specifies whether the walker should follow symbolic links to directories.
    /// If set to `true`, the walker will recurse into linked directories.
    pub follow_links: bool,
    /// The maximum number of directory levels to descend. A depth of 0 only visits the root path.
    pub max_depth: usize,
    /// If set to `true`, I/O errors and symbolic link loops encountered 
    /// during iteration will be silently skipped.
    pub ignore_errors: bool,
    /// Toggle for the symbolic link loop detection mechanism.
    pub detect_loops: bool,
}

impl Default for WalkOptions {
    /// Provides the default configuration for directory traversal.
    ///
    /// # Returns
    ///
    /// Returns a `WalkOptions` instance with the following defaults:
    /// * `follow_links`: `false` - Does not follow symbolic links to prevent accidental loops.
    /// * `max_depth`: `512` - A conservative limit to prevent stack exhaustion in extremely deep trees.
    /// * `ignore_errors`: `false` - All errors are reported to the user by default for maximum safety.
    /// * `detect_loops`: `true` - Loop detection is enabled by default to prevent infinite recursion.
    fn default() -> Self {
        Self {
            follow_links: false,
            max_depth: 512,
            ignore_errors: false,
            detect_loops: true,
        }
    }
}
