#!/bin/bash

links=("http://yann.lecun.com/exdb/mnist/train-images-idx3-ubyte.gz" "http://yann.lecun.com/exdb/mnist/train-labels-idx1-ubyte.gz" "http://yann.lecun.com/exdb/mnist/t10k-images-idx3-ubyte.gz" "http://yann.lecun.com/exdb/mnist/t10k-labels-idx1-ubyte.gz")
hashes=("f68b3c2dcbeaaa9fbdd348bbdeb94873" "d53e105ee54ea40749a09fcbcd1e9432" "9fb629c4189551a2d022fa330f9573f3" "ec29112dd5afa0611ce80d1b7f02629c")
name=MNIST
mkdir -p $TORCH_DATASETS/$name

for i in {0..3}; do
  url=${links[i]}
  hash=${hashes[i]}
  wget $url -P $TORCH_DATASETS/$name
  if [[ "$(md5sum $TORCH_DATASETS/$name/"$(basename $url)" | cut -d\  -f1 )" != "$hash" ]]; then
    echo "[WARNING]\t hashsum of $url does not fit"
  else
    gzip -d $TORCH_DATASETS/$name/"$(basename $url)"
  fi
done
