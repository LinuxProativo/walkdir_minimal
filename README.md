<p align="center">
  <img src="logo.png" width="300">
</p>

<h1 align="center">WalkDir Minimal - A lightweight, POSIX-only directory walker</h1> 
<h3 align="center">A minimal, 100% safe Rust directory walker for POSIX systems that prioritizes determinism and zero dependencies.</h3>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-POSIX-FCC624?&logo=linux&style=flat-square" alt="Platform">
  <a href="https://github.com/LinuxProativo/ALPack/actions/workflows/rust.yml" style="text-decoration:none;"><img src="https://img.shields.io/github/actions/workflow/status/LinuxProativo/walkdir_minimal/rust.yml?label=Test&style=flat-square&logo=github" alt="Build Status"></a>
  <img src="https://img.shields.io/badge/RustC-1.85+-orange?style=flat-square&logo=rust" alt="MSRV">
  <img src="https://img.shields.io/github/languages/code-size/LinuxProativo/walkdir_minimal?style=flat-square&logo=rust&label=Code Size" alt="Code Size">
</p>

## 🔍 Overview

`walkdir_minimal` is a **lightweight, POSIX-only directory walker** written in 
**100% safe Rust**, designed for **maximum portability**, **robust error handling**,
and **predictable iteration order** across UNIX-like systems.

Unlike the popular [`walkdir`](https://crates.io/crates/walkdir) crate, which
offers extensive configurability and Windows support, `walkdir_minimal` aims to
provide a **clean, dependency-free** and **fully deterministic** implementation
that follows the UNIX filesystem model precisely — no abstractions, no hidden
buffering, no non-POSIX extensions.

## ✨ Key Features

* 🧱 **POSIX-only**: Works on Linux, FreeBSD, OpenBSD, NetBSD, and Solaris.
* ⚙️ **No dependencies**: Implemented using only `std::fs`, `std::path`, and 
minimal data structures.
* 🦦 **Lightweight and predictable**: The walker uses a manual stack (no recursion), 
allowing predictable memory and performance behavior.
* 🦉 **Configurable options** via `WalkOptions`:
  * `follow_links`: whether to follow symbolic links to directories.
  * `max_depth`: optional limit on traversal depth.

* 🧠 **Cycle detection**: Detects and prevents infinite loops caused by symbolic 
links that form cycles.
* 🚫 **Graceful handling of I/O errors**: Broken symlinks, permission-denied 
directories, and other errors are returned as `Err(WalkError::Io)`.
* 🦦 **Filtering**: Supports entry-level filtering with a user-provided closure.
* 🧫 **Deterministic**: The order of traversal follows the order provided by the 
filesystem’s `readdir(3)` implementation — consistent across runs on the same system.
* 🧪 **Minimal yet robust**: Designed for projects that require reliable, 
low-level control rather than high-level abstraction.


## 🪶 Design Philosophy

`walkdir_minimal` is built under the following principles:

1. **POSIX compliance first** — all filesystem operations map directly to their POSIX
equivalents (`lstat`, `stat`, `opendir`, `readdir`, etc., via Rust’s `std::fs`).
2. **Deterministic behavior** — the iterator never hides errors, skips entries
silently, or spawns threads.
3. **No allocations beyond what’s necessary** — uses `Vec` for the manual stack
and `HashSet` for visited inode/device pairs (loop detection).
4. **No recursion** — prevents stack overflows and maintains stable memory usage
even for deeply nested trees.
5. **Minimalism** — the crate is intentionally limited to features that can be
reasoned about and verified easily.
6. **Transparency** — the API surfaces raw I/O results instead of silently
ignoring or swallowing them.

`walkdir_minimal` embodies **clarity over complexity**. Its goal is not to compete
with feature-rich crates, but to provide a **clean reference implementation**
of a POSIX-only directory walker.

## ⚖️ Comparison with `walkdir`

| Feature        | `walkdir`                      | `walkdir_minimal`                     |
| -------------- | ------------------------------ | ------------------------------------- |
| Cross-platform | ✅ (Windows, macOS, Linux)     | ❌ POSIX only                         |
| Dependencies   | Many (e.g., same-file, winapi) | ❌ None                               |
| Error handling | Complex iterator states        | Simple `Result<Entry, WalkError>`     |
| Loop detection | Optional, platform-specific    | Deterministic `(dev, ino)` hashing    |
| Symbolic links | Optional follow                | Optional follow                       |
| Custom sorting | Supported                      | Not supported (filesystem order only) |
| Performance    | Optimized for general use      | Optimized for predictability          |
| Safety         | 100% safe Rust                 | 100% safe Rust                       |
| Recursion      | Implicit                       | Manual stack                       |
| Binary size    | Larger                         | Tiny                             |
| Filter API      | Supported (`filter_entry`)    | Supported                      |
| Error type       | `walkdir::Error`             | `WalkError`                  |
| Metadata caching | Yes                          | No (on-demand)             |
| Thread safety    | Yes                          | No (intentionally minimal) |

## 🦉 Example Usage

```rust
use walkdir_minimal::{WalkDir, WalkError};

fn main() -> Result<(), WalkError> {
    for entry in WalkDir::new(".")?.follow_links(false) {
        match entry {
            Ok(e) => println!("{}", e.path().display()),
            Err(err) => eprintln!("Error: {}", err),
        }
    }
    Ok(())
}
```

Output example:

```
.
./src
./src/lib.rs
./src/entry.rs
./src/error.rs
./src/walkdir.rs
```

## 📦 WalkOptions

```rust
#[derive(Clone, Debug)]
pub struct WalkOptions {
    pub follow_links: bool,
    pub max_depth: usize,
}
```

* **`follow_links`** — When `true`, symbolic links to directories are followed.
* **`max_depth`** — Optional limit to recursion depth. `None` means unlimited.

  * The root is always depth `0`.
  * Files or subdirectories at one level below are depth `1`, and so on.

## 🔗 Entry API

```rust
pub struct Entry {
    path: PathBuf,
    depth: usize,
}

impl Entry {
    pub fn path(&self) -> &Path;
    pub fn depth(&self) -> usize;
    pub fn metadata(&self) -> io::Result<fs::Metadata>;
    pub fn symlink_metadata(&self) -> io::Result<fs::Metadata>;
    pub fn file_type(&self) -> io::Result<fs::FileType>;
}
```

* `metadata()` calls `fs::metadata`, following symlinks.
* `symlink_metadata()` calls `fs::symlink_metadata`, **not** following symlinks.
* `file_type()` reports the symbolic link type correctly.

## 🦉 Error Handling

```rust
pub enum WalkError {
    Io(io::Error),
    LoopDetected(PathBuf),
}
```

* **`Io(io::Error)`** — Covers all I/O-related errors, including:

  * Broken symbolic links (`ENOENT`)
  * Permission-denied directories (`EACCES`)
  * Filesystem read errors
* **`LoopDetected(PathBuf)`** — Reported when a cyclic symbolic link is 
detected (only if loop detection is enabled).

## ⚙️ Default Behavior Summary

| Case                            | Behavior                                                 |
| ------------------------------- | -------------------------------------------------------- |
| **Broken symlink**              | Yields `Err(WalkError::Io)`                              |
| **Permission denied directory** | Yields `Err(WalkError::Io)` and continues                |
| **Loop via symlink**            | Yields `Err(WalkError::LoopDetected)` if detection is on |
| **Regular file as root**        | Returns file directly, no traversal                      |
| **Unreadable entry**            | Returns `Err(WalkError::Io)`                             |
| **Exceeds `max_depth`**         | Skips entry silently (depth-guarded)                     |

## 🔍 Technical Details

### 📌 Core Design

`walkdir_minimal` implements a depth-first directory traversal without relying on
any external dependencies, using only POSIX APIs available through Rust’s standard
library. The iterator is built around a manual stack-based traversal that mimics
recursion, avoiding stack overflows for deeply nested directories.

* **Stack-based iteration:** Uses an internal vector of `StackEntry` structs,
each holding an active `ReadDir` handle and its depth.
* **Loop detection:** Uses a `HashSet<(dev, ino)>` to detect and skip cyclic
symlinks, preventing infinite recursion.
* **Filter callbacks:** Optional user-provided closures (`filter_entry`) allow
pruning of the traversal tree dynamically.
* **Error resilience:** Each I/O operation is wrapped in `Result`, and errors
are surfaced as `WalkError` variants (`Io`, `LoopDetected`).

### 📌 Error Handling Philosophy

`walkdir_minimal` follows a **fail-soft** philosophy:

* Broken symlinks are returned as `Ok(Entry)` unless metadata is explicitly requested.
* Directories without permission to read (`EACCES`) return an `Err(WalkError::Io)`,
allowing iteration to continue with the next entry.
* Files disappearing mid-iteration yield `Err(WalkError::Io)` gracefully.

This mirrors `walkdir`’s behavior but keeps it predictable and minimal.

### 📌 Metadata Access

`Entry` deliberately does **not** cache metadata by default. This ensures:

* Minimal memory overhead.
* Consistent behavior with file system changes.
* Full control for users who may wish to query `metadata()` or
`symlink_metadata()` selectively.

```rust
let entry = Entry::new(path, depth);
if let Ok(meta) = entry.metadata() {
    println!("File size: {} bytes", meta.len());
}
```

### 📌 Platform Scope

`walkdir_minimal` targets **POSIX systems only** — this includes:

* GNU/Linux
* *BSD family (FreeBSD, OpenBSD, NetBSD, DragonFly)
* Solaris and Illumos

It relies on `MetadataExt` for device/inode access, which is non-portable
to Windows. No attempt is made to support non-POSIX environments.

### 📌 Performance Characteristics

* Single `ReadDir` handle open at a time per stack frame.
* Minimal heap allocations aside from the stack and visited set.
* No synchronization primitives — designed for **single-threaded deterministic traversal**.
* Filtering and loop detection incur negligible overhead for typical file trees.

### 📌 Safety & Reliability

* No unsafe code.
* Uses standard library types exclusively (`HashSet`, `Vec`, `ReadDir`, etc.).
* All system calls are wrapped in safe Rust abstractions.
* Loop detection ensures full reliability even on pathological file systems.

## 📚 Practical Use Cases

* Static analysis tools.
* POSIX-friendly installers and archivers.
* File packers and dependency scanners.
* System recovery tools that must run without external crates.

Example: skipping hidden files and following symlinks safely:

```rust
use walkdir_minimal::WalkDir;

let iter = WalkDir::new("/usr")
    .unwrap()
    .follow_links(true)
    .filter_entry(|e| !e.path().file_name().map(|n| n.to_string_lossy().starts_with('.')).unwrap_or(false));

for entry in iter {
    match entry {
        Ok(e) => println!("{}", e.path().display()),
        Err(err) => eprintln!("Error: {}", err),
    }
}
```

## 💡 Why Choose `walkdir_minimal`?

* Ideal for **small binaries**, **system utilities**, and **initramfs tools**.
* Zero build dependencies (fast compile times).
* Deterministic and predictable traversal order.
* Designed to be readable and hackable.

## 🧩 Implementation Notes

* Loop detection uses `(dev, ino)` pairs to identify unique directories.
* When `follow_links` is disabled, symlink loops are naturally impossible.
* `max_depth` limits traversal, excluding deeper entries.
* The iterator yields entries as soon as they are discovered — no preloading
or buffering.

## 🤝 Contributing

Contributions are very welcome! Whether it’s fixing a bug, improving
documentation, or adding new features that align with the minimalist and
POSIX-only philosophy, your input is appreciated.

Please follow these guidelines when contributing:

1. **Keep it minimal** — avoid adding dependencies or non-POSIX abstractions.
2. **Preserve safety** — no `unsafe` code will be accepted.
3. **Document behavior clearly** — especially for error cases and edge conditions.
4. **Add tests** — every feature or bug fix should include a minimal test.

If you find a bug or have a suggestion for improvement:

* Open an issue describing the behavior or proposal clearly.
* Include steps to reproduce (for bugs) or examples (for feature requests).

Pull requests should target the `main` branch and include clear commit messages.

## 🧪 Testing

`walkdir_minimal` includes tests for common scenarios:

* Regular files and nested directories.
* Symbolic link loops and detection.
* Permission-denied directories.
* Broken symbolic links.
* Filtered traversals and depth limits.

Run the test suite with:

```bash
cargo test
```

## 🧱 Project Structure

```
walkdir_minimal/
├── src/
│   ├── lib.rs           # Main crate entry
│   ├── entry.rs         # Defines the Entry type
│   ├── error.rs         # WalkError and error utilities
│   ├── options.rs       # WalkOptions definition
│   ├── tests.rs         # Unit and integration tests
│   └── walkdir.rs       # Core iterator implementation
├── README.md            # Project documentation
├── LICENSE              # License file (MIT)
└── Cargo.toml           # Package metadata
```

## 📜 MIT License

This repository has scripts that were created to be free software.
Therefore, they can be distributed and/or modified within the terms of the ***MIT License***.

> ### See the [LICENSE](LICENSE) file for details.

## 📬 Contact & Support

* 📧 **Email:** [m10ferrari1200@gmail.com](mailto:m10ferrari1200@gmail.com)
* 📧 **Email:** [contatolinuxdicaspro@gmail.com](mailto:contatolinuxdicaspro@gmail.com)