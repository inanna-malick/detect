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
