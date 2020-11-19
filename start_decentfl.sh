#!/bin/bash

for DECENTFL_PROCESS in $(seq 0 `expr $1 - 1`); do
  args="${@:2}"
  echo "cargo run --package decentfl --bin decentfl" $(eval "echo $args") &
done
wait