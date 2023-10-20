## detect: a command line tool for finding filesystem entities using expressions


```shell
➜  detect 'executable() && filename(detect) || 
                  extension(.rs) && contains(map_frame)'
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3
./src/expr/frame.rs
```

## operators
- `a && b`
- `a || b`
- `!a`
- `(a)`

## file path predicates

- `filename($REGEX)`: file name
- `filepath($REGEX)`: file path
- `extension($STRING)` exact match on extension

## metadata predicates

- `dir()`: is dir
- `executable()`: is executable
- `size(n1..n2)`/`size(..n)`/`size(n..)`: file size in range, supports `1kb`, `1mb`, etc

## file contents predicates

- `contains($REGEX)`: file contents
- `utf8()`: file contents are utf8
