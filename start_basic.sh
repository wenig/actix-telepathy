#!/bin/bash

source .env

for DECENTFL_PROCESS in $(seq 0 `expr $DECENTFL_PROCESSES - 1`); do
  host=$HOSTNAME:`expr $DECENTFL_BASEPORT + $DECENTFL_PROCESS`
  if [[ "$DECENTFL_BASEHOST:$DECENTFL_BASEPORT" != "$host" ]]; then
    server=$DECENTFL_BASEHOST:$DECENTFL_BASEPORT
  fi
  RUST_LOG=debug cargo run --package basic --bin basic $host $server &
done
wait