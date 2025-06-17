use std::{fs, time::SystemTime};
use slog::{o, Discard, Logger};
use tempdir::TempDir;

#[tokio::test]
async fn test_modified_time_relative() {
    let tmp_dir = TempDir::new("detect-temporal").unwrap();
    let old_file = tmp_dir.path().join("old.txt");
    let new_file = tmp_dir.path().join("new.txt");
    
    // Create an old file
    std::fs::write(&old_file, "old content").unwrap();
    
    // Sleep briefly
    std::thread::sleep(std::time::Duration::from_millis(100));
    
    // Create a new file
    std::fs::write(&new_file, "new content").unwrap();
    
    // Touch the new file to ensure its mtime is recent
    let now = SystemTime::now();
    fs::File::open(&new_file).unwrap().set_modified(now).unwrap();
    
    let mut recent_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "@modified > \"-1.seconds\"".to_owned(),
        |p| recent_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    
    assert!(recent_files.contains(&"new.txt".to_string()));
}

#[tokio::test]
async fn test_modified_time_days() {
    let tmp_dir = TempDir::new("detect-temporal-days").unwrap();
    let recent_file = tmp_dir.path().join("recent.txt");
    let old_file = tmp_dir.path().join("old.txt");
    
    // Create files
    std::fs::write(&recent_file, "recent").unwrap();
    std::fs::write(&old_file, "old").unwrap();
    
    // Set old file's mtime to 30 days ago
    let thirty_days_ago = SystemTime::now() - std::time::Duration::from_secs(30 * 24 * 60 * 60);
    fs::File::open(&old_file).unwrap().set_modified(thirty_days_ago).unwrap();
    
    let mut recent_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "@modified > \"-7.days\"".to_owned(),
        |p| recent_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    
    assert!(recent_files.contains(&"recent.txt".to_string()));
    assert!(!recent_files.contains(&"old.txt".to_string()));
}

#[tokio::test]
async fn test_temporal_keywords() {
    let tmp_dir = TempDir::new("detect-temporal-keywords").unwrap();
    let today_file = tmp_dir.path().join("today.txt");
    let yesterday_file = tmp_dir.path().join("yesterday.txt");
    
    // Create files
    std::fs::write(&today_file, "today").unwrap();
    std::fs::write(&yesterday_file, "yesterday").unwrap();
    
    // Set yesterday file's mtime to yesterday
    let yesterday = SystemTime::now() - std::time::Duration::from_secs(24 * 60 * 60);
    fs::File::open(&yesterday_file).unwrap().set_modified(yesterday).unwrap();
    
    let mut today_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "@modified >= \"today\"".to_owned(),
        |p| today_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    
    assert!(today_files.contains(&"today.txt".to_string()));
    assert!(!today_files.contains(&"yesterday.txt".to_string()));
}

#[tokio::test]
async fn test_absolute_date() {
    let tmp_dir = TempDir::new("detect-temporal-absolute").unwrap();
    let old_file = tmp_dir.path().join("old.txt");
    let new_file = tmp_dir.path().join("new.txt");
    
    // Create files
    std::fs::write(&old_file, "old").unwrap();
    std::fs::write(&new_file, "new").unwrap();
    
    // Set old file's mtime to 2020
    let old_time = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1577836800); // 2020-01-01
    fs::File::open(&old_file).unwrap().set_modified(old_time).unwrap();
    
    let mut new_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "@modified > \"2021-01-01\"".to_owned(),
        |p| new_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    
    assert!(new_files.contains(&"new.txt".to_string()));
    assert!(!new_files.contains(&"old.txt".to_string()));
}