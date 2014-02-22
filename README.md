ps3
===

Matt Pearson-Beck, Jeff Principe, Arjun Shankar

Code for benchmarking: ```time ./benchmark.sh &>/dev/null```

Run this to fix test files: ```tr "\n" "\0" < zhtta-test.txt > zhtta-test-NUL.txt```

Includes:

- zhtta.rs: code for Zhtta web server.

- gash.rs: reference solution for PS2, that you can use for your
  embedded shell (but feel free to use your own gash if you prefer).

- zhtta-test.txt: list of test URLs

- www/index.shtml: a simple test file

