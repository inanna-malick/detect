use slog::{o, Discard, Logger};
use std::{fs, time::SystemTime};
use tempdir::TempDir;

// Shared helper to run temporal test cases
async fn run_temporal_test(
    tmp_dir: &TempDir,
    expr: &str,
    expected_files: Vec<&str>,
    not_expected: Vec<&str>,
) {
    let mut found = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        expr.to_owned(),
        detect::RuntimeConfig::default(),
            |p| found.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();

    for file in expected_files {
        assert!(
            found.contains(&file.to_string()),
            "Expression '{}' should find '{}', but found: {:?}",
            expr,
            file,
            found
        );
    }

    for file in not_expected {
        assert!(
            !found.contains(&file.to_string()),
            "Expression '{}' should not find '{}', but found: {:?}",
            expr,
            file,
            found
        );
    }
}

#[tokio::test]
async fn test_relative_time_operations() {
    let tmp_dir = TempDir::new("detect-temporal-relative").unwrap();

    // Create files with different ages
    let files = vec![
        ("1sec.txt", 1),
        ("10secs.txt", 10),
        ("5mins.txt", 5 * 60),
        ("2hours.txt", 2 * 60 * 60),
        ("3days.txt", 3 * 24 * 60 * 60),
        ("1week.txt", 7 * 24 * 60 * 60 - 1), // Just under 7 days to pass > -7.days test
        ("30days.txt", 30 * 24 * 60 * 60),
    ];

    for (name, age_secs) in &files {
        let path = tmp_dir.path().join(name);
        std::fs::write(&path, "content").unwrap();
        let mtime = SystemTime::now() - std::time::Duration::from_secs(*age_secs);
        fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
    }

    // Test various relative time expressions
    let test_cases = vec![
        // Seconds
        (
            "modified > \"-2.seconds\"",
            vec!["1sec.txt"],
            vec!["10secs.txt"],
        ),
        (
            "modified > \"-30.seconds\"",
            vec!["1sec.txt", "10secs.txt"],
            vec!["5mins.txt"],
        ),
        // Minutes
        (
            "modified > \"-10.minutes\"",
            vec!["1sec.txt", "10secs.txt", "5mins.txt"],
            vec!["2hours.txt"],
        ),
        (
            "modified > \"-1.minute\"",
            vec!["1sec.txt", "10secs.txt"],
            vec!["5mins.txt"],
        ),
        // Hours
        (
            "modified > \"-3.hours\"",
            vec!["1sec.txt", "10secs.txt", "5mins.txt", "2hours.txt"],
            vec!["3days.txt"],
        ),
        (
            "modified > \"-1.hour\"",
            vec!["1sec.txt", "10secs.txt", "5mins.txt"],
            vec!["2hours.txt"],
        ),
        // Days
        (
            "modified > \"-5.days\"",
            vec![
                "1sec.txt",
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
            ],
            vec!["1week.txt"],
        ),
        (
            "modified > \"-7.days\"",
            vec![
                "1sec.txt",
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
                "1week.txt",
            ],
            vec!["30days.txt"],
        ),
        // Weeks
        (
            "modified > \"-2.weeks\"",
            vec![
                "1sec.txt",
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
                "1week.txt",
            ],
            vec!["30days.txt"],
        ),
        // Test with different units abbreviations
        (
            "modified > -30s",
            vec!["1sec.txt", "10secs.txt"],
            vec!["5mins.txt"],
        ),
        (
            "modified > -10m",
            vec!["1sec.txt", "10secs.txt", "5mins.txt"],
            vec!["2hours.txt"],
        ),
        (
            "modified > -3h",
            vec!["1sec.txt", "10secs.txt", "5mins.txt", "2hours.txt"],
            vec!["3days.txt"],
        ),
        (
            "modified > -5d",
            vec![
                "1sec.txt",
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
            ],
            vec!["1week.txt"],
        ),
        (
            "modified > -2w",
            vec![
                "1sec.txt",
                "10secs.txt",
                "5mins.txt",
                "2hours.txt",
                "3days.txt",
                "1week.txt",
            ],
            vec!["30days.txt"],
        ),
    ];

    for (expr, expected, not_expected) in test_cases {
        run_temporal_test(&tmp_dir, expr, expected, not_expected).await;
    }
}

#[tokio::test]
async fn test_absolute_dates_and_keywords() {
    let tmp_dir = TempDir::new("detect-temporal-absolute").unwrap();

    // Create files with specific dates
    let today_file = tmp_dir.path().join("today.txt");
    let yesterday_file = tmp_dir.path().join("yesterday.txt");
    let week_old_file = tmp_dir.path().join("week_old.txt");
    let year_2020_file = tmp_dir.path().join("year_2020.txt");
    let year_2023_file = tmp_dir.path().join("year_2023.txt");

    // Create files
    std::fs::write(&today_file, "today").unwrap();
    std::fs::write(&yesterday_file, "yesterday").unwrap();
    std::fs::write(&week_old_file, "week").unwrap();
    std::fs::write(&year_2020_file, "2020").unwrap();
    std::fs::write(&year_2023_file, "2023").unwrap();

    // Set modification times
    let now = SystemTime::now();
    let yesterday = now - std::time::Duration::from_secs(24 * 60 * 60);
    let week_ago = now - std::time::Duration::from_secs(7 * 24 * 60 * 60);
    let year_2020 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1577836800); // 2020-01-01
    let year_2023 = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1672531200); // 2023-01-01

    fs::File::open(&today_file)
        .unwrap()
        .set_modified(now)
        .unwrap();
    fs::File::open(&yesterday_file)
        .unwrap()
        .set_modified(yesterday)
        .unwrap();
    fs::File::open(&week_old_file)
        .unwrap()
        .set_modified(week_ago)
        .unwrap();
    fs::File::open(&year_2020_file)
        .unwrap()
        .set_modified(year_2020)
        .unwrap();
    fs::File::open(&year_2023_file)
        .unwrap()
        .set_modified(year_2023)
        .unwrap();

    // Test keyword-based queries
    run_temporal_test(
        &tmp_dir,
        "modified >= today",
        vec!["today.txt"],
        vec!["yesterday.txt", "week_old.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified == today",
        vec!["today.txt"],
        vec!["yesterday.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified >= yesterday",
        vec!["today.txt", "yesterday.txt"],
        vec!["week_old.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified < today",
        vec![
            "yesterday.txt",
            "week_old.txt",
            "year_2020.txt",
            "year_2023.txt",
        ],
        vec!["today.txt"],
    )
    .await;

    // Test absolute date queries (quoted and unquoted)
    run_temporal_test(
        &tmp_dir,
        "modified > \"2021-01-01\"",
        vec![
            "today.txt",
            "yesterday.txt",
            "week_old.txt",
            "year_2023.txt",
        ],
        vec!["year_2020.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified > 2021-01-01",
        vec![
            "today.txt",
            "yesterday.txt",
            "week_old.txt",
            "year_2023.txt",
        ],
        vec!["year_2020.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified < 2022-01-01",
        vec!["year_2020.txt"],
        vec!["year_2023.txt", "today.txt"],
    )
    .await;

    // Test midnight boundary
    use chrono::{Local, NaiveTime};

    let before_midnight = tmp_dir.path().join("before_midnight.txt");
    let after_midnight = tmp_dir.path().join("after_midnight.txt");

    std::fs::write(&before_midnight, "before").unwrap();
    std::fs::write(&after_midnight, "after").unwrap();

    let now = Local::now();
    let today_start = now
        .date_naive()
        .and_time(NaiveTime::from_hms_opt(0, 0, 1).unwrap());
    let yesterday_end = now
        .date_naive()
        .pred_opt()
        .unwrap()
        .and_time(NaiveTime::from_hms_opt(23, 59, 59).unwrap());

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

    run_temporal_test(
        &tmp_dir,
        "modified >= today",
        vec!["today.txt", "after_midnight.txt"],
        vec!["before_midnight.txt"],
    )
    .await;
}

#[tokio::test]
async fn test_time_selectors() {
    let tmp_dir = TempDir::new("detect-temporal-selectors").unwrap();

    // Create test files
    let test_file = tmp_dir.path().join("test.txt");
    let old_file = tmp_dir.path().join("old.txt");

    std::fs::write(&test_file, "content").unwrap();
    std::fs::write(&old_file, "old").unwrap();

    // Set old file to be old
    let week_ago = SystemTime::now() - std::time::Duration::from_secs(7 * 24 * 60 * 60);
    fs::File::open(&old_file)
        .unwrap()
        .set_modified(week_ago)
        .unwrap();

    // Test modified selector (already tested above, but verify syntax variants)
    run_temporal_test(
        &tmp_dir,
        "modified > -1hour",
        vec!["test.txt"],
        vec!["old.txt"],
    )
    .await;
    run_temporal_test(
        &tmp_dir,
        "modified > -1hour",
        vec!["test.txt"],
        vec!["old.txt"],
    )
    .await;

    // Test created selector (created - creation time is OS-specific, just verify it runs)
    let mut created_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "created > -1hour".to_owned(),
        detect::RuntimeConfig::default(),
            |p| created_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    // Just verify it doesn't crash - actual results are OS-dependent

    // Test accessed selector
    // Read the test file to update access time
    let _ = std::fs::read_to_string(&test_file).unwrap();

    run_temporal_test(&tmp_dir, "accessed > -1minute", vec!["test.txt"], vec![]).await;
    run_temporal_test(&tmp_dir, "accessed > -1minute", vec!["test.txt"], vec![]).await;

    // Test created time variants
    let mut created_files = Vec::new();
    detect::parse_and_run_fs(
        Logger::root(Discard, o!()),
        tmp_dir.path(),
        false,
        "created > -1hour".to_owned(),
        detect::RuntimeConfig::default(),
            |p| created_files.push(p.file_name().unwrap().to_string_lossy().to_string()),
    )
    .await
    .unwrap();
    // Just verify syntax works

    // Test equality operators
    run_temporal_test(
        &tmp_dir,
        "modified == today",
        vec!["test.txt"],
        vec!["old.txt"],
    )
    .await;
    run_temporal_test(&tmp_dir, "modified != yesterday", vec!["test.txt"], vec![]).await;
}

#[tokio::test]
async fn test_temporal_combined_queries() {
    let tmp_dir = TempDir::new("detect-temporal-combined").unwrap();

    // Create various files
    let files = vec![
        ("old.rs", "rust code", 10 * 24 * 60 * 60), // 10 days old
        ("new.rs", "rust code", 60),                // 1 minute old
        ("old.txt", "text content", 10 * 24 * 60 * 60), // 10 days old
        ("new.txt", "text content", 60),            // 1 minute old
        ("old_todo.rs", "// TODO: fix", 10 * 24 * 60 * 60), // 10 days old
        ("new_todo.rs", "// TODO: implement", 60),  // 1 minute old
    ];

    for (name, content, age_secs) in &files {
        let path = tmp_dir.path().join(name);
        std::fs::write(&path, content).unwrap();
        let mtime = SystemTime::now() - std::time::Duration::from_secs(*age_secs);
        fs::File::open(&path).unwrap().set_modified(mtime).unwrap();
    }

    // Test temporal + extension
    run_temporal_test(
        &tmp_dir,
        "ext == rs && modified > -1day",
        vec!["new.rs", "new_todo.rs"],
        vec!["old.rs", "new.txt", "old.txt", "old_todo.rs"],
    )
    .await;

    // Test temporal + content
    run_temporal_test(
        &tmp_dir,
        r#"content contains "TODO" && modified > -1day"#,
        vec!["new_todo.rs"],
        vec!["old_todo.rs", "new.rs", "new.txt"],
    )
    .await;

    // Test multiple temporal selectors
    run_temporal_test(
        &tmp_dir,
        "modified > -1hour && accessed > -1hour",
        vec!["new.rs", "new.txt", "new_todo.rs"],
        vec!["old.rs", "old.txt", "old_todo.rs"],
    )
    .await;

    // Test temporal with size (all our test files are small)
    run_temporal_test(
        &tmp_dir,
        "size < 100 && modified > -1day",
        vec!["new.rs", "new.txt", "new_todo.rs"],
        vec!["old.rs", "old.txt", "old_todo.rs"],
    )
    .await;

    // Test temporal with negation
    run_temporal_test(
        &tmp_dir,
        r#"!(basename contains "old") && modified > -1day"#,
        vec!["new.rs", "new.txt", "new_todo.rs"],
        vec!["old.rs", "old.txt", "old_todo.rs"],
    )
    .await;
}

#[tokio::test]
async fn test_greater_less_or_equal_operators() {
    let tmp_dir = TempDir::new("detect-temporal-gte-lte").unwrap();

    // Create files with precise timestamps
    // Using slightly offset times to avoid boundary issues
    let now = SystemTime::now();
    let thirty_mins_ago = now - std::time::Duration::from_secs(30 * 60);
    let ninety_mins_ago = now - std::time::Duration::from_secs(90 * 60);
    let two_and_half_hours_ago = now - std::time::Duration::from_secs(150 * 60);
    let four_hours_ago = now - std::time::Duration::from_secs(4 * 60 * 60);

    let recent = tmp_dir.path().join("recent.txt");
    let thirty_mins = tmp_dir.path().join("thirty_mins.txt");
    let ninety_mins = tmp_dir.path().join("ninety_mins.txt");
    let two_half_hours = tmp_dir.path().join("two_half_hours.txt");
    let four_hours = tmp_dir.path().join("four_hours.txt");

    std::fs::write(&recent, "recent").unwrap();
    std::fs::write(&thirty_mins, "thirty_mins").unwrap();
    std::fs::write(&ninety_mins, "ninety_mins").unwrap();
    std::fs::write(&two_half_hours, "two_half_hours").unwrap();
    std::fs::write(&four_hours, "four_hours").unwrap();

    fs::File::open(&recent).unwrap().set_modified(now).unwrap();
    fs::File::open(&thirty_mins)
        .unwrap()
        .set_modified(thirty_mins_ago)
        .unwrap();
    fs::File::open(&ninety_mins)
        .unwrap()
        .set_modified(ninety_mins_ago)
        .unwrap();
    fs::File::open(&two_half_hours)
        .unwrap()
        .set_modified(two_and_half_hours_ago)
        .unwrap();
    fs::File::open(&four_hours)
        .unwrap()
        .set_modified(four_hours_ago)
        .unwrap();

    // Test >= operator (after or equal) - should include files at or after the threshold
    run_temporal_test(
        &tmp_dir,
        "modified >= -1hour",
        vec!["recent.txt", "thirty_mins.txt"],
        vec!["ninety_mins.txt", "two_half_hours.txt", "four_hours.txt"],
    )
    .await;

    run_temporal_test(
        &tmp_dir,
        "modified >= -2hours",
        vec!["recent.txt", "thirty_mins.txt", "ninety_mins.txt"],
        vec!["two_half_hours.txt", "four_hours.txt"],
    )
    .await;

    // Test <= operator (before or equal) - should include files at or before the threshold
    run_temporal_test(
        &tmp_dir,
        "modified <= -1hour",
        vec!["ninety_mins.txt", "two_half_hours.txt", "four_hours.txt"],
        vec!["recent.txt", "thirty_mins.txt"],
    )
    .await;

    run_temporal_test(
        &tmp_dir,
        "modified <= -3hours",
        vec!["four_hours.txt"],
        vec![
            "recent.txt",
            "thirty_mins.txt",
            "ninety_mins.txt",
            "two_half_hours.txt",
        ],
    )
    .await;

    // Test combining >= and <= to create a time range
    run_temporal_test(
        &tmp_dir,
        "modified >= -3hours AND modified <= -1hour",
        vec!["ninety_mins.txt", "two_half_hours.txt"],
        vec!["recent.txt", "thirty_mins.txt", "four_hours.txt"],
    )
    .await;
}
