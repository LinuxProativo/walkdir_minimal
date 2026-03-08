use std::collections::HashSet;
use std::fs::{self, ReadDir};
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use crate::{Entry, WalkError, WalkOptions};

struct StackEntry {
    read_dir: ReadDir,
    depth: usize,
}

pub struct WalkDir {
    root: PathBuf,
    opts: WalkOptions,
    stack: Vec<StackEntry>,
    filter: Option<Box<dyn Fn(&Entry) -> bool>>,
    detect_loops: bool,
    visited: HashSet<(u64, u64)>,
    started: bool,
    root_is_file: bool,
}

impl WalkDir {
    pub fn new(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let md = fs::symlink_metadata(&root)?;
        let root_is_file = md.is_file();

        Ok(Self {
            root,
            opts: WalkOptions::default(),
            stack: Vec::new(),
            filter: None,
            detect_loops: true,
            visited: HashSet::new(),
            started: false,
            root_is_file,
        })
    }

    pub fn follow_links(mut self, follow: bool) -> Self {
        self.opts.follow_links = follow;
        self
    }

    pub fn max_depth(mut self, depth: usize) -> Self {
        self.opts.max_depth = depth;
        self
    }

    pub fn detect_loops(mut self, detect: bool) -> Self {
        self.detect_loops = detect;
        self
    }

    pub fn ignore_errors(mut self, ignore: bool) -> Self {
        self.opts.ignore_errors = ignore;
        self
    }

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

    fn next(&mut self) -> Option<Self::Item> {
        if !self.started {
            self.started = true;
            if self.root_is_file {
                let e = Entry::new(self.root.clone(), 0);
                if self.opts.follow_links && self.detect_loops {
                    if let Ok(md) = e.metadata() {
                        self.visited.insert((md.dev(), md.ino()));
                    }
                }
                return Some(Ok(e));
            } else {
                match fs::read_dir(&self.root) {
                    Ok(rd) => {
                        self.stack.push(StackEntry {
                            read_dir: rd,
                            depth: 0,
                        });
                        if self.detect_loops {
                            if let Ok(md) = fs::metadata(&self.root) {
                                self.visited.insert((md.dev(), md.ino()));
                            }
                        }
                    }
                    Err(e) => {
                        if self.opts.ignore_errors {
                            return None;
                        }
                        return Some(Err(WalkError::Io(e)));
                    }
                }
            }
        }

        while let Some(top) = self.stack.last_mut() {
            match top.read_dir.next() {
                Some(Ok(dirent)) => {
                    let path = dirent.path();
                    let depth = top.depth + 1;

                    let ft = match dirent.file_type() {
                        Ok(ft) => ft,
                        Err(e) => {
                            if self.opts.ignore_errors {
                                continue;
                            }
                            return Some(Err(WalkError::Io(e)));
                        }
                    };

                    let entry = Entry::with_ft(path.clone(), depth, ft);

                    if let Some(ref f) = self.filter {
                        if !f(&entry) {
                            continue;
                        }
                    }

                    let is_dir_res = if self.opts.follow_links {
                        fs::metadata(&path).map(|m| m.is_dir())
                    } else {
                        Ok(ft.is_dir())
                    };

                    return match is_dir_res {
                        Ok(true) => {
                            if self.opts.follow_links && self.detect_loops {
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
                                            return Some(Err(WalkError::Io(e)));
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
                            Some(Err(WalkError::Io(e)))
                        }
                    };
                }
                Some(Err(e)) => {
                    if self.opts.ignore_errors {
                        continue;
                    }
                    return Some(Err(WalkError::Io(e)));
                }
                None => {
                    self.stack.pop();
                    continue;
                }
            }
        }

        None
    }
}
