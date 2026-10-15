use actix::prelude::*;
use actix_broker::BrokerSubscribe;
use actix_telepathy::prelude::*;
use actix_rt;
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;

use clap::Parser;

/// Example to show the easy transition to distributed applications with Actix-Telepathy
#[derive(Parser, Debug)]
struct Parameters {
    /// Own public IP address to listen to
    #[clap(long)]
    own_addr: String,

    /// Addresses of already known cluster nodes
    #[clap(long)]
    seed_nodes: Vec<String>
}

impl Parameters {
    pub fn get_own_addr(&self) -> SocketAddr {
        self.own_addr.parse().unwrap()
    }

    pub fn get_seed_nodes(&self) -> Vec<SocketAddr> {
        self.seed_nodes.iter().map(|addr| addr.parse().unwrap()).collect()
    }
}


#[derive(RemoteMessage, Serialize, Deserialize)]
#[rtype(String)]
struct MyMessage(pub String);

#[derive(RemoteActor)]
#[remote_messages(MyMessage)]
enum MyActor {
    Test
}

impl Actor for MyActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.register(ctx.address().recipient());
        // optional subscription to receive cluster news
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<MyMessage> for MyActor {
    type Result = String;

    fn handle(&mut self,
              msg: MyMessage,
              _ctx: &mut Self::Context)
    -> Self::Result {
        format!("You said: {}", msg.0)
    }
}

// optional handler for receiving cluster news
impl Handler<ClusterLog> for MyActor {
    type Result = ();

    fn handle(&mut self,
              msg: ClusterLog,
              ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ClusterLog::NewMember(node) => {
                let remote_addr = node.get_remote_addr(Self::ACTOR_ID.to_string());
                let remote_str = remote_addr.node.socket_addr.clone().to_string();
                ctx.spawn(async move {
                    remote_addr.send(MyMessage("Hello!".to_string())).await
                }
                .into_actor(self)
                .map(move |res, _act, _ctx| {
                    println!("{remote_str} replied: {}", res.unwrap())
                }));
            },
            ClusterLog::MemberLeft(_ip_addr) => ()
        }
    }
}
impl ClusterListener for MyActor {}

#[actix_rt::main]
pub async fn main() {
    let args: Parameters = Parameters::parse();
    let _local_addr = MyActor::Test.start();
    let _cluster = Cluster::new(args.get_own_addr(), args.get_seed_nodes());
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}