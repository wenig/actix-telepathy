#!/bin/bash


update_everies=(1 2 3 7 10 15)
group_sizes=(1)
seeds=(1992 1993 1994)


for update_every in "${update_everies[@]}"; do
  for group_size in "${group_sizes[@]}"; do
    for seed in "${seeds[@]}"; do
      echo "update frequency: $update_every | cluster size: $group_size"
      args="${@:1}"
      bash start_decentfl.sh $args --group-size $group_size --update-every $update_every --seed $seed
      sleep 5
    done
  done
done

