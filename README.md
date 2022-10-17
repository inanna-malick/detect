## detect: a command line tool for finding filesystem entities using expressions

<pre><font color="#FF3333"><b>➜  </b></font><font color="#95E6CB"><b>detect</b></font> <font color="#77A8D9"><b>git:(</b></font><font color="#FF3333"><b>readme</b></font><font color="#77A8D9"><b>) </b></font><font color="#FFD580"><b>✗</b></font> detect --expr &quot;executable() &amp;&amp; filename(detect) || extension(.rs) &amp;&amp; contains(map_layer)&quot;
./target/release/detect
./target/release/deps/detect-6395eb2c29a3ed5e
./target/debug/detect
./target/debug/deps/detect-34cec1d5ea27ff11
./target/debug/deps/detect-e91a01500af9a97b
./target/debug/deps/detect-0b57d7084445c8b2
./target/debug/deps/detect-32c3beb592fdbbe3
./src/expr.rs
./src/expr/recurse.rs
</pre>