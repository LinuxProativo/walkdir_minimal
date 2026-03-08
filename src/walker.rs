use std::collections::HashSet;
use std::fs::{self, ReadDir};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::io;

use crate::{Entry, WalkOptions, WalkError};

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
                        let dev = md.dev();
                        let ino = md.ino();
                        self.visited.insert((dev, ino));
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
                                let dev = md.dev();
                                let ino = md.ino();
                                self.visited.insert((dev, ino));
                            }
                        }
                    }
                    Err(e) => return Some(Err(WalkError::Io(e))),
                }
            }
        }

        while let Some(top) = self.stack.last_mut() {
            match top.read_dir.next() {
                Some(Ok(dirent)) => {
                    let path = dirent.path();
                    let depth = top.depth + 1;
                    let entry = Entry::new(path.clone(), depth);

                    if let Some(ref f) = self.filter {
                        if !f(&entry) {
                            continue;
                        }
                    }

                    let is_dir_res = if self.opts.follow_links {
                        fs::metadata(&path).map(|m| m.is_dir())
                    } else {
                        fs::symlink_metadata(&path).map(|m| m.is_dir())
                    };

                    return match is_dir_res {
                        Ok(true) => {
                            if self.opts.follow_links && self.detect_loops {
                                if let Ok(md) = fs::metadata(&path) {
                                    let dev = md.dev();
                                    let ino = md.ino();
                                    if self.visited.contains(&(dev, ino)) {
                                        return Some(Err(WalkError::LoopDetected(path)));
                                    } else {
                                        self.visited.insert((dev, ino));
                                    }
                                }
                            }
                            if depth <= self.opts.max_depth {
                                match fs::read_dir(&path) {
                                    Ok(rd) => {
                                        self.stack.push(StackEntry { read_dir: rd, depth });
                                    }
                                    Err(e) => {
                                        return Some(Err(WalkError::Io(e)));
                                    }
                                }
                            }
                            Some(Ok(entry))
                        }
                        Ok(false) => Some(Ok(entry)),
                        Err(e) => {
                            Some(Err(WalkError::Io(e)))
                        }
                    };
                }
                Some(Err(e)) => {
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