# NEXT_MAYBE: Future Improvements for detect

This document captures potential future improvements based on VIBES framework analysis and Rust community best practices. These represent the path from the current `<🔬🪢💧>` rating toward the ideal `<🔬🎀💠>`.

## Current State Summary

**VIBES Rating**: `<🔬🪢💧>`
- **Expressive Power (🔬)**: Multiple syntactic forms, semantic aliases, flexible parsing
- **Context Flow (🪢)**: Clean pipeline architecture, linear dependencies  
- **Error Surface (💧)**: Graceful runtime handling with helpful warnings

**Target**: `<🔬🎀💠>`
- Maintain expressive power
- Achieve independent components
- Move validation to compile/parse time

## Phase 1: Parse-Time Safety & Validation

### 1. Semantic Validation at Parse Time
**Impact**: High | **Complexity**: Medium | **VIBES**: 💧→🧊

Detect logically impossible queries during parsing rather than execution.

```rust
// In ast.rs - add validation during AST construction
impl TypedPredicate {
    pub fn validate_semantics(&self) -> Result<(), ValidationError> {
        match self {
            TypedPredicate::String { 
                selector: StringSelectorType::Type, 
                op: StringOp::Equals, 
                value 
            } => {
                // Type can only be "file", "dir", "symlink"
                if !["file", "dir", "symlink"].contains(&value.as_str()) {
                    return Err(ValidationError::InvalidType {
                        found: value.clone(),
                        valid: vec!["file", "dir", "symlink"],
                    });
                }
            }
            TypedPredicate::Numeric { 
                selector: NumericSelectorType::Size, 
                op: NumericOp::Less, 
                value 
            } if *value == 0 => {
                return Err(ValidationError::ImpossibleCondition(
                    "Files cannot have negative size"
                ));
            }
            // Detect mutually exclusive conditions
            _ => {}
        }
        Ok(())
    }
}
```

### 2. Type-Safe Query Validation
**Impact**: High | **Complexity**: Low | **VIBES**: 💧→💠

Use phantom types to guarantee validated queries at compile time.

```rust
use std::marker::PhantomData;

#[derive(Debug)]
struct Validated;

#[derive(Debug)]
struct Unvalidated;

struct Query<State = Unvalidated> {
    expr: Expr<RawPredicate>,
    _state: PhantomData<State>,
}

impl Query<Unvalidated> {
    fn validate(self) -> Result<Query<Validated>, ValidationError> {
        // Semantic validation
        validate_semantics(&self.expr)?;
        
        // Logical consistency checks
        check_contradictions(&self.expr)?;
        
        Ok(Query {
            expr: self.expr,
            _state: PhantomData,
        })
    }
}

impl Query<Validated> {
    // Only validated queries can execute
    fn execute(&self) -> impl Stream<Item = PathBuf> {
        // Implementation
    }
}
```

### 3. Enhanced Parse-Time Regex Validation
**Impact**: Medium | **Complexity**: Low | **VIBES**: 💧→💠

```rust
// Validate regex patterns during parsing, not execution
impl TypedPredicate {
    fn validate_regex_pattern(pattern: &str) -> Result<CompiledRegex, ParseError> {
        // Check for common mistakes
        if pattern == "*" {
            return Err(ParseError::invalid_token(
                ".*", 
                "Use .* instead of * for wildcard matching"
            ));
        }
        
        // Pre-compile and cache
        let regex = regex::Regex::new(pattern)
            .map_err(|e| ParseError::RegexCompilation {
                pattern: pattern.to_string(),
                error: e,
                suggestion: suggest_regex_fix(pattern),
            })?;
            
        Ok(CompiledRegex(regex))
    }
}
```

## Phase 2: Query Intelligence

### 4. Query Explanation Mode (`--explain`)
**Impact**: High | **Complexity**: Medium | **VIBES**: Maintains 🔬

Show users how their query will be evaluated.

```rust
pub struct QueryExplanation {
    parsed: String,           // Human-readable AST
    evaluation_order: Vec<String>, // Order of predicate evaluation
    estimated_cost: Cost,     // Performance estimate
    optimizations: Vec<String>, // Suggested improvements
}

impl Expr<Predicate> {
    pub fn explain(&self) -> QueryExplanation {
        QueryExplanation {
            parsed: self.to_human_readable(),
            evaluation_order: self.evaluation_order(),
            estimated_cost: self.estimate_cost(),
            optimizations: self.suggest_optimizations(),
        }
    }
    
    fn suggest_optimizations(&self) -> Vec<String> {
        let mut suggestions = Vec::new();
        
        // Detect expensive predicates that could be reordered
        if self.has_content_search() && self.has_extension_filter() {
            suggestions.push(
                "Consider moving extension filter before content search for better performance"
            );
        }
        
        suggestions
    }
}

// CLI integration
if args.explain {
    let explanation = expr.explain();
    println!("Query Analysis:");
    println!("  Parsed as: {}", explanation.parsed);
    println!("  Evaluation order: {:?}", explanation.evaluation_order);
    println!("  Estimated cost: {:?}", explanation.estimated_cost);
    if !explanation.optimizations.is_empty() {
        println!("  Suggestions:");
        for opt in explanation.optimizations {
            println!("    - {}", opt);
        }
    }
    return Ok(());
}
```

### 5. Streaming Preview Mode (`--preview N`)
**Impact**: Medium | **Complexity**: Medium | **VIBES**: Improves 💧

```rust
pub async fn preview_query(
    expr: &str,
    root: &Path,
    limit: usize,
) -> Result<PreviewResult, DetectError> {
    let parsed = parse_expr(expr)?;
    
    println!("Query: {}", expr);
    println!("Interpretation: {}", parsed.to_human_readable());
    println!("Searching in: {}", root.display());
    println!("\nPreview (first {} matches):", limit);
    println!("{:-<60}", "");
    
    let start = Instant::now();
    let mut count = 0;
    let mut shown = 0;
    
    let mut stream = evaluate_streaming(root, parsed);
    
    while let Some(path) = stream.next().await {
        count += 1;
        if shown < limit {
            println!("{}", path.display());
            shown += 1;
        }
        
        // Early termination after sampling enough
        if count > limit * 10 {
            break;
        }
    }
    
    let elapsed = start.elapsed();
    println!("{:-<60}", "");
    println!("Found {} matches in {:?}", shown, elapsed);
    
    if count > shown {
        println!("(Showing first {} of estimated {} total matches)", shown, count * 10);
    }
    
    Ok(PreviewResult { shown, estimated_total: count * 10, elapsed })
}
```

### 6. Semantic Query Templates
**Impact**: Medium | **Complexity**: Low | **VIBES**: Improves 🔬

Add built-in shortcuts for common queries.

```rust
// In grammar - add macro predicates
macro_predicate = { 
    "recent" | "large" | "empty" | "source" | 
    "config" | "test" | "generated" | "binary"
}

// Expansion in parser
fn expand_macro(name: &str) -> Result<String, ParseError> {
    match name {
        "recent" => Ok("modified > -7days"),
        "large" => Ok("size > 10mb"),
        "empty" => Ok("size == 0"),
        "source" => Ok("extension in [rs,ts,js,py,go,java,cpp,c,h]"),
        "config" => Ok("name in [Cargo.toml,package.json,.env,config.toml]"),
        "test" => Ok("(name contains test || name contains spec)"),
        "generated" => Ok("(name contains generated || name contains .pb.)"),
        "binary" => Ok("type == file && !contents ~= .*"),
        _ => Err(ParseError::UnknownMacro(name.to_string()))
    }
}

// Usage examples:
// detect 'recent && source'
// detect 'large && !binary'
// detect 'config && contents contains password'
```

## Phase 3: Architecture Evolution

### 7. Plugin Architecture for Evaluators
**Impact**: High | **Complexity**: High | **VIBES**: 🪢→🎀

Decouple evaluation strategies for different contexts.

```rust
// Core trait for evaluation strategies
trait Evaluator: Send + Sync {
    type Entry;
    type Error: Error;
    
    async fn evaluate(
        &self,
        expr: &Expr<Predicate>,
        entry: &Self::Entry,
    ) -> Result<bool, Self::Error>;
    
    fn can_optimize(&self, expr: &Expr<Predicate>) -> bool;
}

// Filesystem evaluator (current implementation)
pub struct FilesystemEvaluator {
    respect_gitignore: bool,
    follow_symlinks: bool,
}

impl Evaluator for FilesystemEvaluator {
    type Entry = DirEntry;
    type Error = io::Error;
    
    async fn evaluate(&self, expr: &Expr<Predicate>, entry: &DirEntry) -> Result<bool, io::Error> {
        // Current fs evaluation logic
    }
}

// Git evaluator
pub struct GitEvaluator {
    repo: Repository,
    ref_spec: Option<String>,
}

impl Evaluator for GitEvaluator {
    type Entry = TreeEntry;
    type Error = git2::Error;
    
    async fn evaluate(&self, expr: &Expr<Predicate>, entry: &TreeEntry) -> Result<bool, git2::Error> {
        // Git-specific evaluation
    }
}

// Future: Docker, S3, Database evaluators
```

### 8. Parallel Evaluation Strategy
**Impact**: Medium | **Complexity**: High | **VIBES**: Performance, not ergonomics

```rust
use rayon::prelude::*;
use tokio::sync::mpsc;

struct ParallelEvaluator {
    name_predicates: Vec<NamePredicate>,
    metadata_predicates: Vec<MetadataPredicate>,
    content_predicates: Vec<ContentPredicate>,
}

impl ParallelEvaluator {
    fn from_expr(expr: &Expr<Predicate>) -> Self {
        // Partition predicates by evaluation cost
        // Names are cheap (string ops)
        // Metadata is medium (stat calls)
        // Content is expensive (file I/O)
    }
    
    async fn evaluate_parallel(
        &self,
        entries: impl Stream<Item = PathBuf>,
    ) -> impl Stream<Item = PathBuf> {
        let (tx, rx) = mpsc::channel(1000);
        
        // Stage 1: Parallel name filtering (CPU-bound)
        let name_filtered = entries
            .par_bridge()
            .filter(|path| self.evaluate_names(path))
            .collect::<Vec<_>>();
        
        // Stage 2: Parallel metadata filtering (some I/O)
        let metadata_filtered = name_filtered
            .par_iter()
            .filter(|path| self.evaluate_metadata(path))
            .collect::<Vec<_>>();
        
        // Stage 3: Async content filtering (heavy I/O)
        for path in metadata_filtered {
            let tx = tx.clone();
            let preds = self.content_predicates.clone();
            tokio::spawn(async move {
                if evaluate_content(&path, &preds).await {
                    tx.send(path).await.ok();
                }
            });
        }
        
        tokio_stream::wrappers::ReceiverStream::new(rx)
    }
}
```

### 9. User-Extensible Selectors
**Impact**: Medium | **Complexity**: Medium | **VIBES**: Improves 🔬

```toml
# ~/.config/detect/selectors.toml
[selectors.vendored]
patterns = ["vendor/**", "node_modules/**", "target/**"]
description = "Third-party vendored code"

[selectors.docs]
patterns = ["*.md", "*.rst", "*.txt", "docs/**"]
description = "Documentation files"

[selectors.ci]
patterns = [".github/**", ".gitlab-ci.yml", ".travis.yml", "Jenkinsfile"]
description = "CI/CD configuration"

[macros.todo]
expansion = "contents ~= (TODO|FIXME|HACK|XXX|NOTE)"
description = "Code comments needing attention"
```

```rust
// Runtime loading
struct CustomSelectorRegistry {
    selectors: HashMap<String, CustomSelector>,
    macros: HashMap<String, String>,
}

impl CustomSelectorRegistry {
    fn load_from_config() -> Result<Self, ConfigError> {
        let config_path = dirs::config_dir()
            .map(|p| p.join("detect/selectors.toml"));
        
        if let Some(path) = config_path {
            if path.exists() {
                let config = std::fs::read_to_string(path)?;
                return toml::from_str(&config);
            }
        }
        
        Ok(Self::default())
    }
}
```

## Phase 4: Advanced Features

### 10. Incremental Search with Cache
**Impact**: High for repeated searches | **Complexity**: High

```rust
// Cache filesystem metadata for repeated searches
struct IncrementalIndex {
    root: PathBuf,
    entries: HashMap<PathBuf, CachedEntry>,
    last_scan: SystemTime,
}

struct CachedEntry {
    metadata: Metadata,
    content_hash: Option<u64>, // Lazy content hashing
    matches: HashMap<String, bool>, // Query -> result cache
}

impl IncrementalIndex {
    async fn search(&mut self, expr: &str) -> Vec<PathBuf> {
        // Check if we've evaluated this exact query before
        if let Some(cached) = self.get_cached_results(expr) {
            return cached;
        }
        
        // Incremental scan for changes since last_scan
        self.update_changed_files().await;
        
        // Evaluate and cache results
        let results = self.evaluate_with_cache(expr).await;
        self.cache_results(expr, &results);
        results
    }
}
```

### 11. Query Composition and Saved Queries
**Impact**: Medium | **Complexity**: Low

```toml
# ~/.config/detect/queries.toml
[queries.find-todos]
query = "contents ~= TODO && modified > -7days"
description = "Recent TODO comments"

[queries.large-media]
query = "extension in [jpg,png,mp4,mov] && size > 10mb"
description = "Large media files"
```

```rust
// CLI integration
// detect --saved find-todos
// detect --saved large-media --exclude vendor
```

### 12. LSP Integration
**Impact**: High for IDE users | **Complexity**: Very High

```rust
// Language Server Protocol for IDE integration
struct DetectLanguageServer {
    index: Arc<RwLock<IncrementalIndex>>,
}

impl LanguageServer for DetectLanguageServer {
    async fn completion(&self, params: CompletionParams) -> CompletionList {
        // Provide completions for selectors, operators, values
    }
    
    async fn hover(&self, params: HoverParams) -> Option<Hover> {
        // Show documentation for selectors and operators
    }
    
    async fn diagnostics(&self, params: DocumentDiagnosticsParams) -> Vec<Diagnostic> {
        // Real-time query validation
    }
}
```

## Implementation Roadmap

### Priority 1: Critical Fixes (1-2 days)
- [ ] SIGPIPE handling (already in NEXT.md)
- [ ] Semantic validation at parse time
- [ ] Enhanced error messages

### Priority 2: User Experience (2-3 days)
- [ ] Query explanation mode
- [ ] Preview mode
- [ ] Query templates/macros

### Priority 3: Architecture (3-4 days)
- [ ] Plugin architecture
- [ ] Parallel evaluation
- [ ] User-extensible selectors

### Priority 4: Advanced Features (1+ weeks)
- [ ] Incremental search cache
- [ ] Saved queries
- [ ] LSP integration

## Success Metrics

- **Parse-time detection**: 90% of invalid queries caught before execution
- **Query explanation**: Users can understand and optimize their queries
- **Performance**: 2-3x speedup for content-heavy queries via parallel evaluation
- **Extensibility**: Community can add custom evaluators without modifying core
- **Developer experience**: IDE integration with completions and validation

## Risk Mitigation

1. **Backward Compatibility**: All new features behind flags initially
2. **Performance Regression**: Benchmark suite before/after each change
3. **Complexity Creep**: Each feature must have clear use case
4. **Testing Strategy**: Property-based testing for DSL changes
5. **Documentation**: Every new feature documented with examples

## Rust-Specific Best Practices

### Use Zero-Cost Abstractions
```rust
// Good: Generic over predicate type
fn evaluate<P: Predicate>(expr: &Expr<P>) -> bool

// Bad: Dynamic dispatch when not needed
fn evaluate(expr: &Expr<Box<dyn Predicate>>) -> bool
```

### Leverage Type System for Correctness
```rust
// Good: Impossible states unrepresentable
enum TimeSpec {
    Relative(Duration),
    Absolute(DateTime<Local>),
}

// Bad: Nullable fields that are mutually exclusive
struct TimeSpec {
    relative: Option<Duration>,
    absolute: Option<DateTime<Local>>,
}
```

### Prefer Iterator Chains
```rust
// Good: Lazy evaluation, composable
entries.iter()
    .filter(|e| predicate1(e))
    .filter(|e| predicate2(e))
    .take(limit)

// Bad: Eager collection
let mut results = Vec::new();
for entry in entries {
    if predicate1(&entry) && predicate2(&entry) {
        results.push(entry);
        if results.len() >= limit { break; }
    }
}
```

## Community Integration

- Consider submitting RFC to nushell for integration
- Create GitHub Action for repository analysis
- Build VSCode extension using LSP
- Package for major package managers (homebrew, cargo, apt)

## Long-Term Vision

The ultimate goal is for detect to become the standard way to query filesystems programmatically, with:
- Multiple backend evaluators (filesystem, git, cloud storage, databases)
- Rich IDE integration with query building assistance
- Performance comparable to specialized tools (ripgrep, fd)
- Extensibility for domain-specific queries
- LLM-friendly interface for AI-assisted development

This positions detect not just as a find replacement, but as a universal query language for hierarchical data structures.