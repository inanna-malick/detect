use slog::{o, Discard, Logger};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempdir::TempDir;

#[cfg(unix)]
#[tokio::test]
async fn test_permission_denied_file() {
    let tmp_dir = TempDir::new("detect-perms").unwrap();

    // Create a normal file
    let normal_file = tmp_dir.path().join("normal.txt");
    fs::write(&normal_file, "normal content").unwrap();

    // Create a file without read permissions
    let protected_file = tmp_dir.path().join("protected.txt");
    fs::write(&protected_file, "secret content").unwrap();

    // Remove read permissions from the file
    let mut perms = fs::metadata(&protected_file).unwrap().permissions();
    perms.set_mode(0o000); // No permissions
    fs::set_permissions(&protected_file, perms).unwrap();

    // Try to search for content - should skip the unreadable file
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "contents contains content".to_owned(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await;

    // The walk might fail when trying to read protected file content
    // But we should at least find the normal file
    if result.is_ok() {
        assert!(found.contains(&"normal.txt".to_string()));
        assert!(!found.contains(&"protected.txt".to_string()));
    }

    // Restore permissions for cleanup
    let mut perms = fs::metadata(&protected_file).unwrap().permissions();
    perms.set_mode(0o644);
    fs::set_permissions(&protected_file, perms).unwrap();
}

#[tokio::test]
async fn test_broken_symlink() {
    let tmp_dir = TempDir::new("detect-symlink").unwrap();

    // Create a symlink to a non-existent file
    let target = tmp_dir.path().join("nonexistent.txt");
    let link = tmp_dir.path().join("broken_link.txt");

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &link).unwrap();

    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, &link).unwrap();

    // Search for all files - should handle broken symlink gracefully
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.name ~= .*".to_owned(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Broken symlink test failed: {:?}",
        result.err()
    );
    // The behavior with broken symlinks may vary by implementation
    // Just verify it doesn't crash
}

#[tokio::test]
async fn test_file_disappears_during_traversal() {
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    let tmp_dir = TempDir::new("detect-disappear").unwrap();

    // Create several files
    for i in 0..10 {
        fs::write(tmp_dir.path().join(format!("file{}.txt", i)), "content").unwrap();
    }

    // Create a file that we'll delete during traversal
    let vanishing_file = tmp_dir.path().join("vanishing.txt");
    fs::write(&vanishing_file, "now you see me").unwrap();

    let files_processed = Arc::new(Mutex::new(0));
    let files_processed_clone = Arc::clone(&files_processed);
    let vanishing_file_clone = vanishing_file.clone();

    // Delete the file after we've processed a few files
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_millis(10));
            let count = *files_processed_clone.lock().unwrap();
            if count > 3 {
                // Try to remove the file if it still exists
                let _ = fs::remove_file(&vanishing_file_clone);
                break;
            }
        }
    });

    // Search for all txt files
    let mut found = Vec::new();
    let files_processed_search = Arc::clone(&files_processed);
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.suffix == txt".to_owned(),
        |p| {
            found.push(p.file_name().unwrap().to_string_lossy().to_string());
            *files_processed_search.lock().unwrap() += 1;
        },
    )
    .await;

    // Should complete successfully even if a file disappeared
    assert!(
        result.is_ok(),
        "File disappears test failed: {:?}",
        result.err()
    );

    // We should have found at least some files
    assert!(!found.is_empty());
}

#[tokio::test]
async fn test_very_long_path() {
    let tmp_dir = TempDir::new("detect-longpath").unwrap();

    // Create a deeply nested directory structure
    let mut current_path = tmp_dir.path().to_path_buf();

    // Create nested directories with long names
    for i in 0..20 {
        let long_name = format!(
            "very_long_directory_name_number_{}_with_lots_of_characters",
            i
        );
        let next_path = current_path.join(long_name);

        // Stop before we hit filesystem limits
        if next_path.to_string_lossy().len() > 200 {
            break;
        }

        fs::create_dir(&next_path).unwrap();
        current_path = next_path;
    }

    // Create a file at the end
    let deep_file = current_path.join("deep.txt");
    fs::write(&deep_file, "deep content").unwrap();

    // Search for the file
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.name == deep.txt".to_owned(),
        |p| found.push(p.to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Very long path test failed: {:?}",
        result.err()
    );
    assert_eq!(found.len(), 1);
}

#[tokio::test]
async fn test_special_characters_in_filenames() {
    let tmp_dir = TempDir::new("detect-special").unwrap();

    // Create files with various special characters
    let special_names = vec![
        "file with spaces.txt",
        "file'with'quotes.txt",
        "file\"with\"doublequotes.txt",
        "file|with|pipes.txt",
        "file&with&ampersands.txt",
        "file;with;semicolons.txt",
        "file(with)parens.txt",
        "file[with]brackets.txt",
        "file{with}braces.txt",
        "file`with`backticks.txt",
        "file$with$dollars.txt",
    ];

    for name in &special_names {
        // Some characters might not be allowed on all filesystems
        match fs::write(tmp_dir.path().join(name), "content") {
            Ok(_) => {}
            Err(_) => continue, // Skip files that can't be created
        }
    }

    // Search for all txt files
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.suffix == txt".to_owned(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Special characters test failed: {:?}",
        result.err()
    );

    // Should find at least some of the files
    assert!(!found.is_empty());

    // Test searching for a specific special filename
    let mut found_specific = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        r#"path.name == "file with spaces.txt""#.to_owned(),
        |p| found_specific.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Special characters specific test failed: {:?}",
        result.err()
    );
    if found.iter().any(|f| f == "file with spaces.txt") {
        assert_eq!(found_specific.len(), 1);
    }
}

#[tokio::test]
async fn test_empty_directory() {
    let tmp_dir = TempDir::new("detect-empty").unwrap();

    // Create an empty subdirectory
    let empty_dir = tmp_dir.path().join("empty");
    fs::create_dir(&empty_dir).unwrap();

    // Search in the empty directory
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        &empty_dir,
        false,
        "path.name ~= .*".to_owned(),
        |p| found.push(p.to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Empty directory test failed: {:?}",
        result.err()
    );
    // Should find the directory itself but no files
    assert!(found.is_empty() || found.len() == 1);
}

#[tokio::test]
async fn test_circular_symlinks() {
    let tmp_dir = TempDir::new("detect-circular").unwrap();

    // Create circular symlinks
    let link1 = tmp_dir.path().join("link1");
    let link2 = tmp_dir.path().join("link2");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&link2, &link1).unwrap();
        std::os::unix::fs::symlink(&link1, &link2).unwrap();
    }

    // Also create a normal file
    fs::write(tmp_dir.path().join("normal.txt"), "content").unwrap();

    // Search should handle circular symlinks gracefully
    let mut found = Vec::new();
    let result = detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.name ~= .*".to_owned(),
        |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await;

    assert!(
        result.is_ok(),
        "Circular symlinks test failed: {:?}",
        result.err()
    );
    // Should find the normal file at least
    assert!(found.contains(&"normal.txt".to_string()));
}
