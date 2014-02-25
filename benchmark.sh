#!/bin/bash
# Run benchmarking 5 times
./zhtta &
time1=$(date +"%s");
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
printf "\nSupressing output of 4 other bencharking tests...\n"
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt &> /dev/null
time2=$(date +"%s");
diff=$(($time2 - $time1));
pkill zhtta
printf "\n5 tests completed in %d:%0.2d\n" $(($diff/60)) $(($diff % 60))