# detect UX Test Scenarios

Task-based scenarios for evaluating detect's user experience. Each scenario describes a goal without prescribing the exact commands. The agent should determine the appropriate detect queries to accomplish each task.

**Test Methodology**:
- Agent reads scenario description
- Agent determines appropriate detect query/queries
- Agent executes and evaluates results
- Agent notes: ease of discovery, error quality, result accuracy, surprises

---

## Part 1: Discovery & Basic Usage

### Scenario 1.1: Getting Started
**Context**: You've just learned about detect. You need to understand what it can do.

**Tasks**:
1. Find out what selectors are available
2. Find out what operators work with string selectors
3. Get examples of basic queries

**Evaluation**:
- How did you discover this information?
- Was the help system sufficient?
- What was unclear or missing?

---

### Scenario 1.2: Find Files by Extension
**Context**: You're exploring a new codebase.

**Tasks**:
1. Find all Rust files
2. Find all configuration files (JSON, TOML, YAML)
3. Find all markdown or text documentation

**Evaluation**:
- What query syntax felt natural to you?
- Did your first attempt work?
- Were there multiple valid approaches?

---

### Scenario 1.3: Find Files by Name Pattern
**Context**: You're looking for specific files.

**Tasks**:
1. Find the main Cargo.toml file
2. Find all files with "test" in their name
3. Find files starting with "lib"
4. Find all README files (any extension)

**Evaluation**:
- What's the difference between name/basename/stem? How did you learn?
- Did regex vs. literal matching behave as expected?
- Any surprises in what matched?

---

## Part 2: Filtering & Refinement

### Scenario 2.1: Find Large Files
**Context**: Repository is using too much disk space.

**Tasks**:
1. Find all files over 1 megabyte
2. Find files between 500KB and 2MB
3. Find the largest files in the project
4. Find empty files that could be deleted

**Evaluation**:
- How did you specify sizes? Was the syntax intuitive?
- Did you need to look up size unit syntax?
- Can you actually identify the largest files, or just files above a threshold?

---

### Scenario 2.2: Find Recent Changes
**Context**: You need to understand recent development activity.

**Tasks**:
1. Find files modified in the last 7 days
2. Find files modified in the last hour
3. Find files modified yesterday
4. Find files not touched in the last 3 months

**Evaluation**:
- What time syntax did you try first?
- How did you learn the time format options?
- Were relative vs. absolute times intuitive?

---

### Scenario 2.3: Combine Multiple Criteria
**Context**: You need more specific searches.

**Tasks**:
1. Find large Rust files (over 20KB)
2. Find recent configuration files (modified this week)
3. Find old, large files that might be worth archiving
4. Find small test files

**Evaluation**:
- How did you combine conditions?
- Did boolean operators (AND/OR) work as expected?
- What was the precedence behavior?

---

## Part 3: Content Search

### Scenario 3.1: Find Code Patterns
**Context**: You're analyzing code structure.

**Tasks**:
1. Find all files containing TODO comments
2. Find Rust files with async functions
3. Find files that import/use a specific module
4. Find files containing error handling code

**Evaluation**:
- Was content search syntax clear?
- How did you distinguish literal vs. regex search?
- Performance acceptable for content searches?

---

### Scenario 3.2: Code Quality Audit
**Context**: Preparing for code review.

**Tasks**:
1. Find all TODO and FIXME comments
2. Find debug/console logging statements
3. Find commented-out code blocks
4. Find recent files with TODOs (technical debt added recently)

**Evaluation**:
- Could you combine content + temporal filters easily?
- Did regex patterns work for multiple keywords?
- Any false positives/negatives?

---

### Scenario 3.3: Dependency Analysis
**Context**: Understanding project dependencies.

**Tasks**:
1. Find all files importing a specific library
2. Find files using a deprecated API
3. Find configuration files mentioning a service name
4. Find files with database queries

**Evaluation**:
- Were regex patterns necessary or could you use simple contains?
- How did you handle case sensitivity?
- Any challenges with special characters in search terms?

---

## Part 4: Error Recovery & Edge Cases

### Scenario 4.1: Learning from Mistakes
**Context**: You're new to detect and making errors.

**Tasks**:
1. Try to search using a field name you invent (not a real selector)
2. Try to use a numeric operator on a string field
3. Try to search for a file size without specifying units
4. Try an incomplete query (missing value, missing operator, etc.)

**Evaluation**:
- Were error messages helpful or frustrating?
- Did errors teach you the correct syntax?
- Could you recover without external documentation?

---

### Scenario 4.2: Reserved Words
**Context**: You need to search for common words.

**Tasks**:
1. Find files containing the word "Error"
2. Find files named "Android"
3. Find files mentioning "vendor"
4. Find a file literally named "or" or "and"

**Evaluation**:
- Did these work without quoting?
- If you needed quotes, was it obvious when?
- Any surprising behavior?

---

### Scenario 4.3: Special Characters
**Context**: Dealing with unusual file/content patterns.

**Tasks**:
1. Find files with spaces in their names
2. Find files containing regex special characters in content (e.g., `[](){}`)
3. Find files with emoji in filenames (if any exist)
4. Find files with quotes in their content

**Evaluation**:
- When did you need to escape or quote?
- Were escaping rules clear?
- Any unexpected matching behavior?

---

## Part 5: Real-World Workflows

### Scenario 5.1: Onboarding to a New Codebase
**Context**: First day on a project, exploring structure.

**Task Sequence**:
1. Get a sense of what languages are used (by counting extensions)
2. Find the main entry points (main.rs, index.js, etc.)
3. Locate the test directories
4. Find configuration files
5. Identify documentation

**Evaluation**:
- Could you accomplish this efficiently with detect?
- What would make this workflow smoother?
- Did you need to combine detect with other tools?

---

### Scenario 5.2: Debugging a Regression
**Context**: A feature broke recently, need to find what changed.

**Task Sequence**:
1. Find all files modified in the last 24 hours
2. Among those, find files related to feature X (by name or content)
3. Find other files that import/use the changed files
4. Locate tests for the affected code

**Evaluation**:
- Could you chain these queries effectively?
- Did you need to save intermediate results?
- How did you correlate findings?

---

### Scenario 5.3: Refactoring Preparation
**Context**: You need to rename a function across the codebase.

**Task Sequence**:
1. Find all files using the old function name
2. Exclude test files from the results
3. Find only recent uses (to prioritize active code)
4. Verify no commented-out code would match

**Evaluation**:
- How did you exclude certain paths or file types?
- Could you combine multiple NOT conditions?
- Was it clear what would be affected?

---

### Scenario 5.4: Security Audit
**Context**: Looking for potential security issues.

**Task Sequence**:
1. Find files that might contain secrets (environment files, config with "password" or "key")
2. Find files with hardcoded IPs or URLs
3. Find files importing deprecated security libraries
4. Exclude test/fixture files from results

**Evaluation**:
- Could you express "potential secrets" queries?
- How specific could you get with patterns?
- Any false positives/negatives to handle?

---

### Scenario 5.5: Performance Investigation
**Context**: Application is slow, investigating large files and old code.

**Task Sequence**:
1. Find the 10 largest source files
2. Find old files (not modified in 6+ months) that are also large
3. Find files with suspiciously simple names in production code
4. Identify log files or build artifacts that shouldn't be committed

**Evaluation**:
- Could you identify "largest" or needed external sorting?
- How did you combine age + size criteria?
- What did you learn about the project?

---

## Part 6: Advanced Patterns

### Scenario 6.1: Cross-Language Patterns
**Context**: Multi-language codebase, need language-specific searches.

**Tasks**:
1. Find all test files across languages (Rust's #[test], JavaScript's describe/test, Python's test_*)
2. Find dependency declarations across package managers (Cargo.toml, package.json, requirements.txt)
3. Find error handling across languages (Result<>, try/catch, except)

**Evaluation**:
- Could you express language-specific patterns?
- Did you need multiple queries or one complex one?
- How maintainable are these queries?

---

### Scenario 6.2: Temporal Patterns
**Context**: Understanding code evolution.

**Tasks**:
1. Find files created in the last month (new features)
2. Find files modified recently but created long ago (maintenance)
3. Find files untouched for over a year (candidates for archival)
4. Compare files modified today vs. this week

**Evaluation**:
- Could you query both created and modified times?
- How did you express different time windows?
- Performance on temporal queries acceptable?

---

### Scenario 6.3: Negative Space Exploration
**Context**: Finding what's NOT there.

**Tasks**:
1. Find source files without tests (files not in test directories, without test counterparts)
2. Find large files without documentation comments
3. Find old files without recent updates
4. Find files that don't use a logging library

**Evaluation**:
- How did you express negation?
- Could you combine NOT with other conditions easily?
- Did you find what you expected?

---

## Part 7: Tool Integration

### Scenario 7.1: Shell Integration
**Context**: Using detect in scripts and pipelines.

**Tasks**:
1. Count how many Rust files exist in the project
2. Get a size total for all JavaScript files
3. Pass all test files to a linter
4. Archive all files over 1MB and not modified in 90 days

**Evaluation**:
- Does detect output work cleanly in pipes?
- Can you integrate with xargs, wc, etc. easily?
- Any issues with path formatting or special characters?

---

### Scenario 7.2: IDE/Editor Integration
**Context**: Quick searches during development.

**Tasks**:
1. Quickly find where a specific error message is defined
2. Find all usages of a configuration key
3. Locate examples of a coding pattern
4. Find similar files to the one you're editing

**Evaluation**:
- Is detect fast enough for interactive use?
- Could you bind common queries to keyboard shortcuts?
- How would you integrate results with your editor?

---

### Scenario 7.3: CI/CD Integration
**Context**: Automated checks in build pipeline.

**Tasks**:
1. Verify no files contain debugging statements before release
2. Check that all public APIs have documentation
3. Ensure no secret files are staged for commit
4. Validate file size limits aren't exceeded

**Evaluation**:
- Can detect be used in conditional checks (exit codes)?
- Is performance suitable for CI pipelines?
- Error output appropriate for logs?

---

## Part 8: Interface Comparison

### Scenario 8.1: CLI vs MCP - Same Task
**Context**: You have access to both interfaces.

**Task**: Find all TypeScript files over 50KB modified in the last week

**Approach**:
1. Solve using CLI interface
2. Solve using MCP interface
3. Compare experiences

**Evaluation**:
- Which interface felt more natural?
- Were results identical?
- Performance differences?
- Error handling better in which interface?

---

### Scenario 8.2: MCP-Specific Capabilities
**Context**: Using detect through an AI assistant.

**Tasks**:
1. Search with result limits to avoid overwhelming context
2. Search in specific directories outside current working directory
3. Include gitignored files in search
4. Request help/documentation

**Evaluation**:
- Are MCP-specific parameters discoverable?
- Does JSON response format work well for AI agents?
- Help system adequate for learning through MCP?

---

### Scenario 8.3: Error Recovery Comparison
**Context**: Making intentional mistakes in both interfaces.

**Tasks**:
1. Use invalid syntax in CLI and MCP
2. Request non-existent selectors in both
3. Provide malformed values in both

**Evaluation**:
- Which interface has better error messages?
- Can you recover from errors more easily in one?
- Does MCP lose information (like miette formatting)?

---

## Evaluation Framework

After completing scenarios, assess:

### Discovery
- [ ] Could you learn detect without external docs?
- [ ] Were selectors/operators discoverable?
- [ ] Did error messages teach effectively?

### Correctness
- [ ] Did queries return expected results?
- [ ] Any surprising matches or misses?
- [ ] Edge cases handled well?

### Ergonomics
- [ ] Was syntax intuitive?
- [ ] Did natural queries work?
- [ ] How much trial-and-error needed?

### Performance
- [ ] Response time acceptable?
- [ ] Handled large codebases well?
- [ ] Content search fast enough?

### Composition
- [ ] Could you combine conditions easily?
- [ ] Boolean logic clear?
- [ ] Complex queries maintainable?

### Integration
- [ ] Works well with shell tools?
- [ ] Suitable for scripting?
- [ ] CI/CD friendly?

### Overall Impression
- Most pleasant surprise:
- Most frustrating aspect:
- Biggest UX gap:
- Ready for recommendation? (Yes/No/With caveats)

---

## Test Execution Notes

- Run scenarios in a real project (1000+ files, multiple languages)
- Don't look at documentation first - try to discover naturally
- Note your first instinct for each query
- Record exact commands you tried (including failed attempts)
- Time queries that feel slow
- Document surprising behavior
- Rate difficulty: Easy/Medium/Hard/Impossible
- Note any workarounds needed
