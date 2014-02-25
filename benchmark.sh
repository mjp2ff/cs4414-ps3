#!/bin/bash
# Run benchmarking 5 times
./zhtta &
time1=$(date +"%s");
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
httperf --server localhost --port 4414 --rate 60 --num-conns 60 --wlog=y,./zhtta-test-NUL.txt
time2=$(date +"%s");
diff=$(($time2 - $time1));
pkill zhtta
echo $'\n'
echo "$(($diff/60)) minutes and $(($diff % 60)) seconds elapsed."