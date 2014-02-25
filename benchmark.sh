#!/bin/bash
# Run benchmarking 10 times
./zhtta &
T=$(date +%s%N);
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
printf "\nSupressing output of 9 other bencharking tests...\n"
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
T=$(($(date +%s%N)-T));
pkill zhtta
printf "\n10 tests completed in %d.%d seconds.\n" "$((T/1000000000))" "$((T/1000000%1000))"