use super::*;
use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::PathBuf;

fn create_temp_dir(name: &str) -> PathBuf {
    let tmp = env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    tmp
}

struct PermGuard {
    locked_path: PathBuf,
    original_mode: u32,
    root: PathBuf,
}

impl PermGuard {
    fn new(locked_path: PathBuf, restricted_mode: u32, root: PathBuf) -> Self {
        let original_mode = fs::metadata(&locked_path)
            .expect("failed to read metadata before restricting permissions")
            .permissions()
            .mode();

        fs::set_permissions(&locked_path, fs::Permissions::from_mode(restricted_mode))
            .expect("failed to set restricted permissions");

        Self {
            locked_path,
            original_mode,
            root,
        }
    }
}

impl Drop for PermGuard {
    fn drop(&mut self) {
        let _ = fs::set_permissions(
            &self.locked_path,
            fs::Permissions::from_mode(self.original_mode),
        );
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn test_ignore_permission_denied_on_locked_subdir() {
    let root = create_temp_dir("walkdir_test_permission_denied");

    fs::write(root.join("accessible.txt"), b"ok").expect("failed to create accessible.txt");

    let locked = root.join("locked");
    fs::create_dir(&locked).expect("failed to create locked dir");
    fs::write(locked.join("secret.txt"), b"secret").expect("failed to create secret.txt");

    let _guard = PermGuard::new(locked.clone(), 0o000, root.clone());

    {
        let results: Vec<_> = WalkDir::new(&root)
            .expect("failed to create walker")
            .ignore_permission_denied(false)
            .collect();

        let errors: Vec<_> = results.iter().filter(|r| r.is_err()).collect();
        let entries: Vec<_> = results.iter().filter_map(|r| r.as_ref().ok()).collect();

        println!("ignore_permission_denied false");
        println!("{:#?}", errors);

        assert_eq!(
            errors.len(),
            1,
            "expected exactly one error, got: {errors:?}"
        );
        match errors[0].as_ref().unwrap_err() {
            WalkError::Io(io_err, path) => {
                assert_eq!(
                    io_err.kind(),
                    std::io::ErrorKind::PermissionDenied,
                    "expected PermissionDenied, got {io_err}"
                );
                assert_eq!(
                    path, &locked,
                    "error path should point to the locked directory"
                );
            }
            other => panic!("unexpected error variant: {other:?}"),
        }

        let names: Vec<_> = entries
            .iter()
            .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"accessible.txt".to_string()));
        assert!(names.contains(&"locked".to_string()));
        assert!(
            !names.contains(&"secret.txt".to_string()),
            "secret.txt should not be reachable"
        );
    }

    {
        let results: Vec<_> = WalkDir::new(&root)
            .expect("failed to create walker")
            .ignore_permission_denied(true)
            .collect();

        let errors: Vec<_> = results.iter().filter(|r| r.is_err()).collect();
        assert!(errors.is_empty(), "expected no errors, got: {errors:?}");

        println!("ignore_permission_denied true");
        println!("{:#?}", errors);

        let names: Vec<_> = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .map(|e| e.path().file_name().unwrap().to_string_lossy().into_owned())
            .collect();

        assert!(names.contains(&"accessible.txt".to_string()));
        assert!(names.contains(&"locked".to_string()));
        assert!(
            !names.contains(&"secret.txt".to_string()),
            "secret.txt should not be reachable"
        );
    }
}

#[test]
fn walkdir_filter_works() {
    println!("\nFilter Works:");

    let tmp = create_temp_dir("walkdir_minimal_filter");
    fs::create_dir_all(tmp.join("a")).unwrap();
    fs::create_dir_all(tmp.join("b_ignore")).unwrap();
    File::create(tmp.join("a/file1.txt")).unwrap();
    File::create(tmp.join("b_ignore/file2.txt")).unwrap();

    let walker = WalkDir::new(&tmp)
        .unwrap()
        .filter_entry(|e| !e.path().to_string_lossy().contains("ignore"));

    let mut files = Vec::new();
    for entry in walker {
        let e = entry.unwrap();
        println!("{}", e.path().display());
        files.push(e.path().to_path_buf());
    }

    assert!(files.iter().any(|p| p.ends_with("file1.txt")));
    assert!(!files.iter().any(|p| p.ends_with("file2.txt")));
}

#[test]
fn walkdir_follow_symlinks() {
    println!("\nFollow symlinks:");

    let tmp = create_temp_dir("walkdir_minimal_symlinks");
    fs::create_dir_all(tmp.join("target")).unwrap();
    File::create(tmp.join("target/file.txt")).unwrap();

    let link_path = tmp.join("link_to_target");
    symlink(tmp.join("target"), &link_path).unwrap();

    let walker = WalkDir::new(&tmp)
        .unwrap()
        .follow_links(true)
        .detect_loops(false);

    let mut paths = Vec::new();
    for entry in walker {
        let e = entry.unwrap();
        println!("{}", e.path().display());
        paths.push(e.path().to_path_buf());
    }

    assert!(paths.iter().any(|p| p.ends_with("file.txt")));
}

#[test]
fn walkdir_loop_detection() {
    println!("\nLoop detection:");

    let tmp = create_temp_dir("walkdir_minimal_loops");
    fs::create_dir_all(tmp.join("a")).unwrap();
    fs::create_dir_all(tmp.join("a/b")).unwrap();

    symlink(tmp.join("a"), tmp.join("a/b/link_back")).unwrap();

    let walker = WalkDir::new(&tmp)
        .unwrap()
        .follow_links(true)
        .detect_loops(true);

    let mut visited = 0;
    let mut loop_detected = false;
    for entry in walker {
        if let Err(WalkError::LoopDetected(_)) = entry {
            println!("Loop detected");
            loop_detected = true;
            break;
        }
        println!("visited {}", entry.unwrap().path().display());
        visited += 1;
    }

    assert!(visited < 10);
    assert!(
        loop_detected,
        "Deveria ter detectado um loop infinito de links simbólicos"
    );
}

#[test]
fn walkdir_handles_large_dir() {
    println!("\nHandle Large Dir:");

    let tmp = create_temp_dir("walkdir_minimal_large");
    for i in 0..50 {
        let mut f = File::create(tmp.join(format!("file_{i}.txt"))).unwrap();
        writeln!(f, "conteúdo {i}").unwrap();
    }

    let count = WalkDir::new(&tmp).unwrap().count();
    println!("Found {} files", count);
    assert_eq!(count, 50);
}

#[test]
fn walkdir_ignores_broken_symlinks() {
    println!("\nBroken and valid symlinks test:");

    let tmp = create_temp_dir("walkdir_minimal_broken_link");
    let real_file = tmp.join("file.txt");
    let real_dir = tmp.join("dir");
    fs::write(&real_file, "hello").unwrap();
    fs::create_dir(&real_dir).unwrap();
    let valid_file_link = tmp.join("link_to_file");
    let _ = symlink(&real_file, &valid_file_link);
    let valid_dir_link = tmp.join("link_to_dir");
    let _ = symlink(&real_dir, &valid_dir_link);
    let broken_link = tmp.join("broken_link");
    let _ = symlink("/nonexistent/path", &broken_link);

    let walker = WalkDir::new(&tmp).unwrap().follow_links(true);

    let mut found_valid_links = 0;
    let mut found_broken_links = 0;

    for item in walker {
        match item {
            Ok(e) => {
                let path = e.path();
                println!("Found: {}", path.display());
                assert_ne!(path, &broken_link);

                if path == valid_file_link || path == valid_dir_link {
                    found_valid_links += 1;
                }
            }
            Err(err) => match err {
                WalkError::Io(io_err, ..) => {
                    println!("IO error: {:?}", io_err);
                    found_broken_links += 1;
                }
                _ => {}
            },
        }
    }

    println!("Valid links: {found_valid_links}, Broken links: {found_broken_links}");
}

#[test]
fn walkdir_follow_symlinks_no_loop_detection() {
    println!("\nFollow symlinks no loop detection:");

    let tmp = env::temp_dir().join("walkdir_minimal_symlink_noloop_test");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();

    let dir_a = tmp.join("a");
    let dir_b = tmp.join("b");
    fs::create_dir(&dir_a).unwrap();
    fs::create_dir(&dir_b).unwrap();
    File::create(dir_a.join("file_a.txt")).unwrap();

    symlink(&dir_a, dir_b.join("link_to_a")).unwrap();
    symlink(&dir_b, dir_a.join("link_to_b")).unwrap();

    let walker = WalkDir::new(&tmp)
        .unwrap()
        .follow_links(true)
        .detect_loops(false)
        .max_depth(5);

    let mut count = 0;
    for entry in walker {
        let e = entry.unwrap();
        println!("Visited: {}", e.path().display());
        count += 1;
    }

    assert!(
        count > 2,
        "Expected to visit multiple paths when following symbolic links"
    );
}
