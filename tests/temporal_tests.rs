use slog::{o, Discard, Logger};
use std::{fs, time::SystemTime};
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
    fs::File::open(&new_file)
        .unwrap()
        .set_modified(now)
        .unwrap();

    let mut recent_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified > \"-1.seconds\"".to_owned(),
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
    fs::File::open(&old_file)
        .unwrap()
        .set_modified(thirty_days_ago)
        .unwrap();

    let mut recent_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified > \"-7.days\"".to_owned(),
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
    fs::File::open(&yesterday_file)
        .unwrap()
        .set_modified(yesterday)
        .unwrap();

    let mut today_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified >= \"today\"".to_owned(),
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
    fs::File::open(&old_file)
        .unwrap()
        .set_modified(old_time)
        .unwrap();

    let mut new_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified > \"2021-01-01\"".to_owned(),
        |p| new_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(new_files.contains(&"new.txt".to_string()));
    assert!(!new_files.contains(&"old.txt".to_string()));
}

// ===== Additional temporal tests for gaps =====

#[tokio::test]
async fn test_created_time_selector() {
    let tmp_dir = TempDir::new("detect-temporal-created").unwrap();
    let old_file = tmp_dir.path().join("old.txt");
    let new_file = tmp_dir.path().join("new.txt");

    // Create files
    std::fs::write(&old_file, "old").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::fs::write(&new_file, "new").unwrap();

    // Note: Setting creation time is OS-specific and not always possible
    // We'll test that the selector at least works without errors
    let mut files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "created > \"-1.hour\"".to_owned(),
        |p| files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    // Should find at least some files (can't reliably set ctime on all platforms)
    // Just verify the query runs without error
}

#[tokio::test]
async fn test_accessed_time_selector() {
    let tmp_dir = TempDir::new("detect-temporal-accessed").unwrap();
    let file = tmp_dir.path().join("test.txt");

    // Create and read file
    std::fs::write(&file, "content").unwrap();
    let _ = std::fs::read_to_string(&file).unwrap();

    let mut files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "accessed > \"-1.minute\"".to_owned(),
        |p| files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(files.contains(&"test.txt".to_string()));
}

#[tokio::test]
async fn test_temporal_equality_operator() {
    let tmp_dir = TempDir::new("detect-temporal-eq").unwrap();
    let today_file = tmp_dir.path().join("today.txt");
    let old_file = tmp_dir.path().join("old.txt");

    // Create files
    std::fs::write(&today_file, "today").unwrap();
    std::fs::write(&old_file, "old").unwrap();

    // Set old file to a week ago
    let week_ago = SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 60 * 60);
    fs::File::open(&old_file)
        .unwrap()
        .set_modified(week_ago)
        .unwrap();

    // Test equality with "today"
    let mut today_matches = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified == \"today\"".to_owned(),
        |p| today_matches.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(today_matches.contains(&"today.txt".to_string()));
    assert!(!today_matches.contains(&"old.txt".to_string()));
}

#[tokio::test]
async fn test_temporal_midnight_boundary() {
    let tmp_dir = TempDir::new("detect-temporal-midnight").unwrap();
    let before_midnight = tmp_dir.path().join("before.txt");
    let after_midnight = tmp_dir.path().join("after.txt");

    // Create files
    std::fs::write(&before_midnight, "before").unwrap();
    std::fs::write(&after_midnight, "after").unwrap();

    // Set one file to 23:59:59 yesterday, one to 00:00:01 today
    use chrono::{Local, NaiveTime};

    let now = Local::now();
    let today_start = now
        .date_naive()
        .and_time(NaiveTime::from_hms_opt(0, 0, 1).unwrap());
    let yesterday_end = now
        .date_naive()
        .pred_opt()
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());

    // Convert to SystemTime
    let today_start_systime: SystemTime = today_start.and_local_timezone(Local).unwrap().into();
    let yesterday_end_systime: SystemTime = yesterday_end.and_local_timezone(Local).unwrap().into();

    fs::File::open(&after_midnight)
        .unwrap()
        .set_modified(today_start_systime)
        .unwrap();
    fs::File::open(&before_midnight)
        .unwrap()
        .set_modified(yesterday_end_systime)
        .unwrap();

    // Test "today" boundary
    let mut today_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified >= \"today\"".to_owned(),
        |p| today_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(today_files.contains(&"after.txt".to_string()));
    assert!(!today_files.contains(&"before.txt".to_string()));
}

#[tokio::test]
async fn test_temporal_all_time_units() {
    let tmp_dir = TempDir::new("detect-temporal-units").unwrap();

    // Create files with different ages
    let files = vec![
        ("10secs.txt", 10),
        ("5mins.txt", 5 * 60),
        ("2hours.txt", 2 * 60 * 60),
        ("3days.txt", 3 * 24 * 60 * 60),
        ("1week.txt", 7 * 24 * 60 * 60),
    ];

    for (name, age_secs) in &files {
        let path = tmp_dir.path().join(name);
        std::fs::write(&path, "content").unwrap();
        let mtime = SystemTime::now() - std::time::Duration::from_secs(*age_secs);
        fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
    }

    // Test each unit type
    let test_cases = vec![
        ("modified > \"-30.seconds\"", vec!["10secs.txt"]),
        (
            "modified > \"-10.minutes\"",
            vec!["10secs.txt", "5mins.txt"],
        ),
        (
            "modified > \"-3.hours\"",
            vec!["10secs.txt", "5mins.txt", "2hours.txt"],
        ),
        (
            "modified > \"-5.days\"",
            vec!["10secs.txt", "5mins.txt", "2hours.txt", "3days.txt"],
        ),
        (
            "modified > \"-2.weeks\"",
            vec![
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
                "1week.txt",
            ],
        ),
    ];

    for (expr, expected) in test_cases {
        let mut found = Vec::new();
        detect::parse_and_run_fs(
            Logger::root(Discard, o!()),
            tmp_dir.path(),
            false,
            expr.to_owned(),
            |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
        )
        .await
        .unwrap();

        for file in expected {
            assert!(
                found.contains(&file.to_string()),
                "Expression '{}' should find '{}', but found: {:?}",
                expr,
                file,
                found
            );
        }
    }
}

#[tokio::test]
async fn test_temporal_combined_with_other_predicates() {
    let tmp_dir = TempDir::new("detect-temporal-combined").unwrap();

    // Create various files
    let files = vec![
        ("old.rs", "rust", 10 * 24 * 60 * 60),  // 10 days old
        ("new.rs", "rust", 60),                 // 1 minute old
        ("old.txt", "text", 10 * 24 * 60 * 60), // 10 days old
        ("new.txt", "text", 60),                // 1 minute old
    ];

    for (name, content, age_secs) in files {
        let path = tmp_dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        let mtime = SystemTime::now() - std::time::Duration::from_secs(age_secs);
        fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
    }

    // Find recent Rust files
    let mut recent_rust = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "path.extension == rs && modified > \"-1.day\"".to_owned(),
        |p| recent_rust.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(recent_rust.contains(&"new.rs".to_string()));
    assert!(!recent_rust.contains(&"old.rs".to_string()));
    assert!(!recent_rust.contains(&"new.txt".to_string()));
    assert!(!recent_rust.contains(&"old.txt".to_string()));
}

#[tokio::test]
async fn test_temporal_multiple_selectors() {
    let tmp_dir = TempDir::new("detect-temporal-multiple").unwrap();
    let file = tmp_dir.path().join("test.txt");

    // Create a file
    std::fs::write(&file, "content").unwrap();

    // Complex query with multiple temporal selectors
    // This tests that different temporal selectors can be used together
    let mut matches = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "modified > \"-1.hour\" && accessed > \"-1.hour\"".to_owned(),
        |p| matches.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    assert!(matches.contains(&"test.txt".to_string()));
}
