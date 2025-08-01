/// Provide helpful grammar guide when parsing fails
pub fn get_parse_error_hints() -> &'static str {
    "Quick grammar guide:
  • Selectors:
    - path (or path.full)    # complete file path
    - path.parent            # directory containing file
    - path.name              # filename with extension
    - path.stem              # filename without extension
    - path.suffix            # file extension without dot
    - type                   # file, dir, or symlink
    - contents               # file contents
    - size                   # size in bytes (supports KB/MB/GB)
    - modified               # modification time
    - created                # creation time
    - accessed               # access time
  • Operators:
    - == (or =)             # exact match (case-sensitive)
    - !=                    # not equal
    - ~= (or ~, =~)         # regex match
    - >, <, >=, <=          # comparisons
    - contains              # substring search
    - in [...]              # set membership
    - &&, ||, !             # boolean logic
  • Examples:
    - path.full ~= \".*\\.rs\"     # Rust files
    - path.name == \"test.rs\"    # specific filename
    - path.parent contains \"src\" # in src directory
    - size > 1KB             # files larger than 1KB
    - type == \"file\"         # regular files only
    - contents ~= \"TODO\"     # search file contents
    - modified > \"-7.days\"   # modified in last week"
}
