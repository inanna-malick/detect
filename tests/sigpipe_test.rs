use std::process::{Command, Stdio};

/// Test that detect handles broken pipes gracefully when used with head/tail etc.
/// This test verifies that we exit with code 0 rather than panicking.
#[test]
#[cfg(unix)] // SIGPIPE is Unix-specific
fn test_sigpipe_handling_with_head() {
    // Create a command that will generate more output than head will consume
    let mut child = Command::new("./target/release/detect")
        .args(&["size >= 0", "."]) // This should match many files
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start detect");

    // Pipe the output to head -1 which will close the pipe after reading one line
    let detect_stdout = child.stdout.take().expect("Failed to get stdout");

    let head_output = Command::new("head")
        .args(&["-1"])
        .stdin(detect_stdout)
        .output()
        .expect("Failed to run head");

    // Wait for detect to finish
    let detect_exit = child.wait().expect("Failed to wait for detect");

    // The key test: detect should exit with code 0, not panic
    assert!(
        detect_exit.success(),
        "detect should exit successfully when pipe is broken, got exit code: {:?}",
        detect_exit.code()
    );

    // head should have successfully processed one line
    assert!(
        head_output.status.success(),
        "head should exit successfully"
    );

    // head should have produced exactly one line of output
    let output_lines: Vec<_> = head_output.stdout.split(|&b| b == b'\n').collect();
    assert!(
        output_lines.len() >= 1 && !output_lines[0].is_empty(),
        "head should produce at least one non-empty line"
    );
}

/// Test normal operation (no broken pipe) still works correctly
#[test]
fn test_normal_output_still_works() {
    let output = Command::new("./target/release/detect")
        .args(&["name == Cargo.toml", "."])
        .output()
        .expect("Failed to run detect");

    assert!(
        output.status.success(),
        "detect should succeed normally: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Cargo.toml"),
        "Should find Cargo.toml in output"
    );
}