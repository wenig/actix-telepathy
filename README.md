[![crates.io](https://img.shields.io/crates/v/actix-telepathy?label=latest)](https://crates.io/crates/actix-telepathy)
![Tests on main](https://github.com/wenig/actix-telepathy/workflows/Rust/badge.svg)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)
[![Dependency Status](https://deps.rs/crate/actix-telepathy/0.5.5/status.svg)](https://deps.rs/crate/actix-telepathy/0.5.5)
![Downloads](https://img.shields.io/crates/d/actix-telepathy.svg)

# Actix Telepathy

Inspired by [actix-remote](https://github.com/actix/actix-remote) and [Akka Cluster](https://github.com/akka/akka), Actix Telepathy is an extension to the Rust actor framework [Actix](https://github.com/actix/actix). It empowers Rust users to build distributed application within the actor framework.

## Notes

So far, we only support single seed nodes. Connecting to different seed nodes can result in unexpected behavior.

## Version Matches

| actix | -telepathy | -telepathy-derive |
|-------|------------|-------------------|
| 0.10  | 0.1        | 0.1               |
| 0.11  | 0.2        | 0.1               |
| 0.12  | 0.3        | 0.2               |
| 0.12  | 0.4        | 0.3               |
| 0.13  | 0.5        | 0.3               |

## Tests

Run ignored tests sequentially, because these tests run multiple threads themselves. 

```
cargo test -- --ignored --test-threads=1
```

## Usage

### Cargo.toml

```toml
[dependencies]
actix = "0.13.1"
actix-telepathy = "0.5.5"
```

### main.rs

```rust
use actix_rt;
use actix_telepathy::prelude::*;
use actix::prelude::*;
use tokio;
use std::net::{ToSocketAddrs, SocketAddr};

#[actix_rt::main]
async fn main() {
    let bind_addr = "127.0.0.1:1992".parse().unwrap();
    let seed_nodes = vec![];
    let _cluster = Cluster::new(bind_addr, seed_nodes);

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
```

**Please consider citing this work when you are using it!**

```bibtex
@inproceedings{10.1145/3623506.3623575,
    author = {Wenig, Phillip and Papenbrock, Thorsten},
    title = {Actix-Telepathy},
    year = {2023},
    isbn = {9798400704000},
    publisher = {Association for Computing Machinery},
    address = {New York, NY, USA},
    url = {https://doi.org/10.1145/3623506.3623575},
    doi = {10.1145/3623506.3623575},
    abstract = {The actor programming model supports the development of concurrent applications by encapsulating state and behavior into independent actors. Each actor is a computational entity with strictly private state and behavior. Actors communicate via asynchronous messaging and, in this way, require neither shared memory nor locking. This makes the actor model suitable not only for parallel programming but also for distributed applications engineering. The Rust programming language is a statically-typed language that gained a lot of attention in the past years due to its efficient, economical and safe memory management. To ease the development of parallel applications, several actor model frameworks have been built for Rust. However, no actively maintained Rust actor framework provides the necessary features to write distributed applications. For this reason, we propose an extension for Rust’s Actix library, called Actix-Telepathy, that enables remote messaging and offers clustering support. It allows developers to setup remote actors that can communicate across a computer network with the help of a straight forward and easy to understand interface. Our evaluation demonstrates that Actix-Telepathy competes well in remote messaging performance and memory consumption with other actor libraries, such as Scala’s popular Akka library.},
    booktitle = {Proceedings of the 10th ACM SIGPLAN International Workshop on Reactive and Event-Based Languages and Systems},
    pages = {14–24},
    numpages = {11},
    keywords = {Distributed Computing, Rust, Actor Model},
    location = {Cascais, Portugal},
    series = {REBLS 2023}
}
```
