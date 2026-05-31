//! The core recursive directory traversal implementation.
//!
//! This module provides the [`WalkDir`] iterator, which manages a manual stack
//! to perform a depth-first search (DFS) through the filesystem. It includes
//! specialized logic for symbolic link handling and loop prevention.

use crate::WalkError;
use crate::entry::Entry;
use crate::options::WalkOptions;
use std::collections::HashSet;
use std::fs::{self, ReadDir};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// Internal state for each directory level in the traversal stack.
///
/// This structure keeps track of an open directory stream and its
/// corresponding depth to manage the recursive process iteratively.
struct StackEntry {
    /// The standard library iterator for the current directory's entries.
    read_dir: ReadDir,
    /// The depth level of this directory relative to the walk root.
    depth: usize,
}

/// The primary iterator for recursive directory walking.
///
/// `WalkDir` maintains the traversal state, including the stack of open
/// directories, the set of visited nodes (for loop detection), and
/// configuration settings for the walk.
pub struct WalkDir {
    /// The starting directory or file path.
    root: PathBuf,
    /// Configuration options (links, depth, error handling).
    opts: WalkOptions,
    /// A manual stack used to avoid recursion and potential stack overflow.
    stack: Vec<StackEntry>,
    /// An optional user-provided closure to skip specific entries.
    filter: Option<Box<dyn Fn(&Entry) -> bool>>,
    /// Set of (device ID, inode ID) to identify already visited directories.
    visited: HashSet<(u64, u64)>,
    /// Holds the root entry when the root path is a file, yielded on the first call to next().
    pending_root: Option<Entry>,
    /// Holds a deferred error from read_dir on the root, resolved in next() based on ignore_errors.
    pending_error: Option<WalkError>,
}

impl WalkDir {
    /// Initializes a new directory walker starting at the given path.
    ///
    /// # Arguments
    /// * `root` - The starting path for the traversal.
    ///
    /// # Returns
    /// * `io::Result<Self>` - A configured `WalkDir` instance or an I/O error
    /// if the root path is inaccessible.
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let md = fs::symlink_metadata(&root)?;

        let (stack, pending_root, pending_error) = if md.is_file() {
            (vec![], Some(Entry::new(root.clone(), 0)), None)
        } else {
            match fs::read_dir(&root) {
                Ok(rd) => (
                    vec![StackEntry {
                        read_dir: rd,
                        depth: 0,
                    }],
                    None,
                    None,
                ),
                Err(e) => (vec![], None, Some(WalkError::Io(e, root.clone()))),
            }
        };

        Ok(Self {
            root,
            opts: WalkOptions::default(),
            stack,
            filter: None,
            visited: HashSet::new(),
            pending_root,
            pending_error,
        })
    }

    /// Sets whether the walker should follow symbolic links to directories.
    ///
    /// # Arguments
    /// * `follow` - If `true`, the walker will recurse into symlinked directories.
    ///
    /// # Returns
    /// The modified `WalkDir` instance for method chaining.
    pub fn follow_links(mut self, follow: bool) -> Self {
        self.opts.follow_links = follow;
        self
    }

    /// Sets the maximum recursion depth for the traversal.
    ///
    /// # Arguments
    /// * `depth` - The limit of subdirectory levels to descend (root is 0).
    ///
    /// # Returns
    /// The modified `WalkDir` instance.
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.opts.max_depth = depth;
        self
    }

    /// Enables or disables the symbolic link loop detection mechanism.
    ///
    /// # Arguments
    /// * `detect` - If `true`, tracks visited nodes to prevent infinite recursion.
    ///
    /// # Returns
    /// The modified `WalkDir` instance.
    pub fn detect_loops(mut self, detect: bool) -> Self {
        self.opts.detect_loops = detect;
        self
    }

    /// Configures the walker to silently ignore I/O errors and detected loops.
    ///
    /// # Arguments
    /// * `ignore` - If `true`, problematic entries are skipped instead of halting with an error.
    ///
    /// # Returns
    /// The modified `WalkDir` instance.
    pub fn ignore_errors(mut self, ignore: bool) -> Self {
        self.opts.ignore_errors = ignore;
        self
    }

    /// Enables or disables skipping entries that cannot be accessed due to permission restrictions.
    ///
    /// # Arguments
    /// * `ignore` - If `true`, permission denied errors will be silently ignored.
    ///
    /// # Returns
    /// The modified `WalkDir` instance for method chaining.
    pub fn ignore_permission_denied(mut self, ignore: bool) -> Self {
        self.opts.ignore_permission_denied = ignore;
        self
    }

    /// Registers a filter function to prune the search tree.
    ///
    /// # Arguments
    /// * `f` - A closure that returns `false` if an entry (and its children) should be skipped.
    ///
    /// # Returns
    /// The modified `WalkDir` instance.
    pub fn filter_entry<F>(mut self, f: F) -> Self
    where
        F: Fn(&Entry) -> bool + 'static,
    {
        self.filter = Some(Box::new(f));
        self
    }
}

impl Iterator for WalkDir {
    type Item = Result<Entry, WalkError>;

    /// Advances the iterator to produce the next directory entry.
    ///
    /// # Safety and Performance
    /// This implementation uses an iterative approach with a manual stack to
    /// prevent `StackOverflowError`. On POSIX systems, it identifies directories
    /// uniquely via `st_dev` and `st_ino` to safely handle symbolic link cycles.
    ///
    /// It also leverages cached file types from directory entries to minimize
    /// redundant metadata syscalls.
    ///
    /// # Returns
    /// * `Some(Ok(Entry))` - The next file or directory entry found.
    /// * `Some(Err(WalkError))` - An error encountered (unless `ignore_errors` is true).
    /// * `None` - When the traversal is finished.
    fn next(&mut self) -> Option<Self::Item> {
        // If root is a file, yield it once and finish.
        if let Some(e) = self.pending_root.take() {
            return Some(Ok(e));
        }

        // Deliver any deferred error from opening the root directory.
        if let Some(err) = self.pending_error.take() {
            if self.opts.ignore_errors {
                return None;
            }
            return Some(Err(err));
        }

        // Main iteration loop processing the stack.
        while let Some(top) = self.stack.last_mut() {
            match top.read_dir.next() {
                Some(Ok(dirent)) => {
                    let path = dirent.path();
                    let depth = top.depth + 1;

                    // Efficiently get file type from dirent (minimal syscalls).
                    let ft = match dirent.file_type() {
                        Ok(ft) => ft,
                        Err(e) => {
                            if self.opts.ignore_errors {
                                continue;
                            }
                            return Some(Err(WalkError::Io(e, path)));
                        }
                    };

                    let entry = Entry::with_ft(path.clone(), depth, ft);

                    // Apply user-defined filter.
                    if let Some(ref f) = self.filter {
                        if !f(&entry) {
                            continue;
                        }
                    }

                    // Determine if we should treat this as a directory (follow links or not).
                    let is_dir_res = if self.opts.follow_links {
                        fs::metadata(&path).map(|m| m.is_dir())
                    } else {
                        Ok(ft.is_dir())
                    };

                    return match is_dir_res {
                        Ok(true) => {
                            // POSIX loop detection: uses device and inode IDs.
                            if self.opts.follow_links && self.opts.detect_loops {
                                if let Ok(md) = fs::metadata(&path) {
                                    let id = (md.dev(), md.ino());
                                    if self.visited.contains(&id) {
                                        if self.opts.ignore_errors {
                                            continue;
                                        }
                                        return Some(Err(WalkError::LoopDetected(path)));
                                    }
                                    self.visited.insert(id);
                                }
                            }
                            // Push new directory to stack if within depth limits.
                            if depth <= self.opts.max_depth {
                                match fs::read_dir(&path) {
                                    Ok(rd) => {
                                        self.stack.push(StackEntry {
                                            read_dir: rd,
                                            depth,
                                        });
                                    }
                                    Err(e) => {
                                        if !self.opts.ignore_errors {
                                            return Some(Err(WalkError::Io(e, path)));
                                        }
                                    }
                                }
                            }
                            Some(Ok(entry))
                        }
                        Ok(false) => Some(Ok(entry)),
                        Err(e) => {
                            if self.opts.ignore_errors {
                                continue;
                            }
                            Some(Err(WalkError::Io(e, path)))
                        }
                    };
                }
                Some(Err(e)) => {
                    if self.opts.ignore_errors {
                        continue;
                    }
                    return Some(Err(WalkError::Io(e, self.root.clone())));
                }
                None => {
                    // Directory stream exhausted, pop from stack.
                    self.stack.pop();
                    continue;
                }
            }
        }
        None
    }
}
