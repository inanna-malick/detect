#[cfg(test)]
mod cli_tests {
    use std::process::Command;
    
    #[test]
    #[ignore] // Requires git repo
    fn test_git_with_simple_mode() {
        // Test that git flag works with simple mode
        let output = Command::new("cargo")
            .args(&["run", "--", "-g", "HEAD", "TODO"])
            .output()
            .expect("failed to execute");
            
        assert!(output.status.success() || output.status.code() == Some(0));
    }
    
    #[test]
    #[ignore] // Requires git repo
    fn test_git_with_expression_mode() {
        // Test that git flag works with expression mode
        let output = Command::new("cargo")
            .args(&["run", "--", "-g", "HEAD", "-e", "ext = rs"])
            .output()
            .expect("failed to execute");
            
        assert!(output.status.success() || output.status.code() == Some(0));
    }
    
    #[test]
    #[ignore] // Requires git repo
    fn test_git_with_type_flag() {
        // Test that git flag works with type shortcuts
        let output = Command::new("cargo")
            .args(&["run", "--", "-g", "HEAD", "--type", "rust"])
            .output()
            .expect("failed to execute");
            
        assert!(output.status.success() || output.status.code() == Some(0));
    }
}