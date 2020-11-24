#!/bin/bash

source .env

for DECENTFL_PROCESS in $(seq 0 `expr $DECENTFL_PROCESSES - 1`); do
  RUST_ENV=debug cargo run --package decentfl --bin basic $HOSTNAME:`expr $DECENTFL_BASEPORT + $DECENTFL_PROCESS` $DECENTFL_BASEHOST:$DECENTFL_BASEPORT &
done
wait