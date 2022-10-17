## detect: a command line tool for finding filesystem entities using expressions


```shell
➜  detect --expr "executable() && filename(detect) || 
                  extension(.rs) && contains(map_layer)" 
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3
./src/expr.rs
./src/expr/recurse.rs
```

using this expression language

```rust
/// Filesystem entity matcher expression, with branches for matchers on
/// - file name
/// - file metadata
/// - file contents
pub enum Expr<Name, Metadata, Content> {
    // literal boolean values
    KnownResult(bool),
    // boolean operators
    Not(Box<Self>),
    And(Vec<Self>),
    Or(Vec<Self>),
    // predicates
    Name(Name),
    Metadata(Metadata),
    Contents(Content),
}
```

and my [recursion crate](recursion crate) to provide concise, performant, and stack safe evaluation that takes advantage of boolean operator short circuiting to minimze syscalls used.

For example, this code attempts to evaluate an expression given _just_ a filesystem entity's path, to minimize metadata and file content reads in cases where we can evaluate an expression given just a file name.

```rust
let e: Expr<Done, &MetadataPredicate, &ContentPredicate> = e.collapse_layers(|layer| {
    match layer {
        // evaluate all NamePredicate predicates
        ExprLayer::Name(p) => Expr::KnownResult(p.is_match(path)),
        // boilerplate
        ExprLayer::Operator(op) => op.attempt_short_circuit(),
        ExprLayer::KnownResult(k) => Expr::KnownResult(k),
        ExprLayer::Metadata(p) => Expr::Metadata(p),
        ExprLayer::Contents(p) => Expr::Contents(p),
    }
});


// short circuit before querying metadata (expensive)
if let Expr::KnownResult(b) = e {
    return Ok(b);
}
```