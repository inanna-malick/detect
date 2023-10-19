## detect: a command line tool for finding filesystem entities using expressions


```shell
➜  detect --expr 'executable() && filename(detect) || 
                  extension(.rs) && contains(map_layer)'
./target/debug/detect
./src/expr/frame.rs
```

