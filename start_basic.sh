#!/bin/bash

for DECENTFL_PROCESS in $(seq 0 `expr $1 - 1`); do
  args="${@:2}"
  echo "RUST_ENV=debug cargo run --package decentfl --bin basic" #$(eval "echo $args") &
done
wait