## detect: a command line tool for finding filesystem entities using expressions


```shell
➜  detect 'name ~= detect || extension ~= rs && contents ~= map_frame'
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3
./src/expr/frame.rs
```

## boolean operators
- `a && b`
- `a || b`
- `!a`
- `(a)`


## string operators
- `==`
- `~=` (regex match)

## numeric operators
- `>`, `>=`, `<`, `<=`
- `==`

# Selectors

All selectors start with '@', eg '@name'

## file path selectors

- name (or filename)
- path (or filepath)
- extension (or ext)

## metadata selectors

- size (or filesize)
- type (or filetype)

## file contents predicates

- contents (or file)
