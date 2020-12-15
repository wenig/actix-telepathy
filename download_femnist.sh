#!/bin/sh

git clone https://github.com/TalwalkarLab/leaf.git
cd leaf/data/femnist
./preprocess.sh -s niid --sf 0.01 -k 0 -t sample
