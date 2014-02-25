ps3
===

Matt Pearson-Beck, Jeff Principe, Arjun Shankar

NOTES

-- ```make``` now has built-in testing - tests zhtta-test-NUL.txt 10 times, prints out the output of first run and the time taken to run all 10.

-- If you just want to benchmark without recompiling, run ```./benchmark.sh```. Make sure you have recompiled first (just use make honestly).

Run this to fix given test files: ```tr "\n" "\0" < zhtta-test.txt > zhtta-test-NUL.txt```


Includes:

- zhtta.rs: code for Zhtta web server.

- gash.rs: reference solution for PS2, that you can use for your
  embedded shell (but feel free to use your own gash if you prefer).

- zhtta-test.txt: list of test URLs

- www/index.shtml: a simple test file

