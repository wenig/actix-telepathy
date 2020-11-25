#!/bin/bash

source .env

for DECENTFL_PROCESS in $(seq 0 `expr $DECENTFL_PROCESSES - 1`); do
  host=$HOSTNAME:`expr $DECENTFL_BASEPORT + $DECENTFL_PROCESS`
  if [[ "$DECENTFL_BASEHOST:$DECENTFL_BASEPORT" != "$host" ]]; then
    seed="--seed-nodes $DECENTFL_BASEHOST:$DECENTFL_BASEPORT"
  fi
  server=$DECENTFL_BASEHOST:$DECENTFL_BASEPORT
  split=`expr $DECENTFL_SPLIT_OFFSET + $DECENTFL_PROCESS`
  args="${@:1}"
  RUST_LOG=debug cargo run --package decentfl --bin decentfl $host $args --db-path "decentfl.$split.db" --split $split --server-addr $server $seed &
done
sleep 1200; killall decentfl