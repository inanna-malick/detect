/// Provide helpful grammar guide when parsing fails
pub fn get_parse_error_hints() -> &'static str {
    "Quick grammar guide:
  • Selectors:
    - name (or filename)     # file name
    - path (or filepath)     # full path
    - ext (or extension)     # file extension
    - size (or filesize)     # size in bytes
    - type (or filetype)     # file, dir, or symlink
    - contents (or file)     # file contents
    - modified (or mtime)    # modification time
    - created (or ctime)     # creation time
    - accessed (or atime)    # access time
  • Operators:
    - == (or =)             # exact match (case-sensitive)
    - !=                    # not equal
    - ~= (or ~, =~)         # regex match
    - >, <, >=, <=          # comparisons
    - contains              # substring search
    - in [...]              # set membership
    - &&, ||, !             # boolean logic
  • Examples:
    - name ~= \".*\\.rs\"      # regex match
    - size > 1024            # size in bytes
    - type == \"file\"         # file or directory
    - contents ~= \"TODO\"     # search file contents
    - modified > \"-7.days\"   # modified in last week"
}
