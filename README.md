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
use actix::{System, Supervisor, Supervised, Handler, Context};
use actix_telepathy::*;

struct OwnListener {}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, mut remote_addr) => {
                remote_addr.do_send(Box::new("welcome"))
            },
            ClusterLog::MemberLeft(addr) => debug!("ClusterLog: MemberLeft")
        }
    }
}

fn main() {
    actix::System::run(|| {
        let cluster_listener = Supervisor::start(|_| OwnListener {});
        let cluster = Cluster::new(local_ip, seed_nodes, vec![cluster_listener.recipient()]);
    });
}
```

**Please consider citing this work when you are using it!**