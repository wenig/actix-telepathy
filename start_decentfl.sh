set -o allexport; source .env; set +o allexport

for DECENTFL_PROCESS in $(seq 0 `expr $DECENTFL_PROCESSES - 1`); do
  eval "echo $1"
done
