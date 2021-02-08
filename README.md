![Tests on main](https://github.com/wenig/actix-telepathy/workflows/Rust/badge.svg)

# Actix Telepathy

Inspired by [actix-remote](https://github.com/actix/actix-remote) and [Akka Cluster](https://github.com/akka/akka), Actix Telepathy is an extension to the Rust actor framework [Actix](https://github.com/actix/actix). It empowers Rust users to build distributed application within the actor framework.

## Usage

### Cargo.toml

```toml
[dependencies]
actix = "0.10"
actix-telepathy = "0.1.0"
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
