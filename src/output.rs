use std::io::{self, Write};

/// Safe output writer that handles BrokenPipe errors gracefully
/// Follows Unix convention: exit code 0 on SIGPIPE/BrokenPipe
pub struct SafeOutput<W: Write> {
    writer: W,
}

impl<W: Write> SafeOutput<W> {
    pub fn new(writer: W) -> Self {
        SafeOutput { writer }
    }

    /// Write a line safely, handling BrokenPipe gracefully
    /// Returns Ok(()) for successful writes, Err for other I/O errors
    /// On BrokenPipe: exits process with code 0 (Unix convention)
    pub fn writeln(&mut self, content: &str) -> io::Result<()> {
        match writeln!(self.writer, "{}", content) {
            Ok(_) => {
                // Flush to ensure immediate output and catch BrokenPipe early
                if let Err(e) = self.writer.flush() {
                    if e.kind() == io::ErrorKind::BrokenPipe {
                        // Unix convention: exit 0 on SIGPIPE/BrokenPipe
                        std::process::exit(0);
                    }
                    return Err(e);
                }
                Ok(())
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::BrokenPipe {
                    // Unix convention: exit 0 on SIGPIPE/BrokenPipe
                    std::process::exit(0);
                }
                Err(e)
            }
        }
    }

    /// Get a reference to the underlying writer
    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    /// Get a mutable reference to the underlying writer
    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }
}

/// Convenience function for creating SafeOutput with stdout
pub fn safe_stdout() -> SafeOutput<io::Stdout> {
    SafeOutput::new(io::stdout())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_normal_output() {
        let mut buffer = Vec::new();
        {
            let mut safe_output = SafeOutput::new(Cursor::new(&mut buffer));
            safe_output
                .writeln("test line 1")
                .expect("write should succeed");
            safe_output
                .writeln("test line 2")
                .expect("write should succeed");
        }

        let output = String::from_utf8(buffer).expect("valid UTF-8");
        assert_eq!(output, "test line 1\ntest line 2\n");
    }

    #[test]
    fn test_empty_output() {
        let mut buffer = Vec::new();
        {
            let mut safe_output = SafeOutput::new(Cursor::new(&mut buffer));
            safe_output.writeln("").expect("empty write should succeed");
        }

        let output = String::from_utf8(buffer).expect("valid UTF-8");
        assert_eq!(output, "\n");
    }

    #[test]
    fn test_get_references() {
        let mut buffer = Vec::new();
        let mut safe_output = SafeOutput::new(Cursor::new(&mut buffer));

        // Test get_ref
        let _ref = safe_output.get_ref();

        // Test get_mut
        let _mut_ref = safe_output.get_mut();

        // Verify we can still write after using references
        safe_output
            .writeln("after refs")
            .expect("write should succeed");
    }

    // Note: Testing actual BrokenPipe behavior requires integration tests
    // since it involves process::exit() which can't be easily unit tested
}
