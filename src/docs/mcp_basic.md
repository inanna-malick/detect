Search filesystem entities by name, metadata AND contents in a single query.

Every query follows: selector operator value
Where selector is: name, ext, size, contents, modified, etc.
Where operator is: ==, contains, >, in [...], etc.
Where value is: text, number, date, regex pattern, etc.

SIMPLEST QUERIES:
name == README.md                                     - selector: name, operator: ==, value: README.md
size > 1000000                                        - selector: size, operator: >, value: 1000000
contents contains TODO                                - selector: contents, operator: contains, value: TODO
ext in [js, ts]                                       - selector: ext, operator: in, value: [js, ts]
modified > "-7.days"                                  - selector: modified, operator: >, value: "-7.days"

COMMON DEVELOPER TASKS:
Find security issues:
ext == env && contents contains SECRET                 - Secrets in env files
ext in [js, py, rb] && contents contains api_key      - API keys in code  
name contains config && size < 50000 && contents contains password - Passwords in small configs

Find code that needs attention:
name contains test && modified < "-90.days"            - Stale test files
modified > "-30.days" && contents contains TODO        - Recent TODOs
size > 10000 && contents contains FIXME                - FIXMEs in large files

Find specific file types:
ext == log && size > 100000000                        - Large log files
name contains migration && ext in [sql, py]            - Database migrations
ext in [yml, yaml] && contents contains localhost      - Local config files

IMPOSSIBLE WITH GREP/GLOB:
These queries cannot be done with standard tools:
• Files modified in the last week with specific content
• Large files (>10MB) containing passwords
• Config files created today
• Test files that haven't been touched in 90 days
• Files by size range with content search

QUICK EXAMPLES:
name == config.json                                - Find exact file
ext in [js, ts, jsx]                               - Find JavaScript files
contents contains TODO                            - Search inside any file
size > 1000000                                     - Files over 1MB
modified > "-7.days"                               - Recently modified

CORE OPERATORS (just 4!):
==         exact match
contains   substring search
in [...]   multiple options
>          greater than

SELECTORS:
name      filename/path
ext       file extension
size      file size (bytes)
contents  file contents
modified  modification time
type      file/dir/symlink

BOOLEAN LOGIC:
&&  means AND
||  means OR
!   means NOT
()  for grouping

PRACTICAL MULTI-LAYER EXAMPLES:
Find large Python files with class definitions:
ext == py && size > 50000 && contents contains class

Find recent markdown files mentioning bugs:
ext == md && modified > "-30.days" && contents contains bug

Find test files that might have issues:
name contains test && ext in [py, js, go] && contents contains TODO

Find configuration files with secrets:
ext in [json, yaml, yml] && name contains config && contents contains secret

SMART EXCLUSION PATTERNS:
ext == js && !path contains node_modules && contents contains TODO
ext == py && !path contains __pycache__ && contents contains import
ext == rs && !path contains /target/ && contents contains unsafe

TEMPORAL QUERIES:
modified > "-7.days"                                - Last week
modified > "-1.hour"                                - Last hour
modified > "-30.minutes"                            - Last 30 minutes
modified > "2024-01-01"                             - After specific date

Need regex patterns or advanced operators? Use the detect_help tool.