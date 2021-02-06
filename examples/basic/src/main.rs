mod serializer;

#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::prelude::*;
use actix::prelude::*;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
#[allow(unused_imports)]
use serializer::MySerializer;
use tokio;
use std::net::{ToSocketAddrs, SocketAddr};
use std::fs;
use actix_broker::{BrokerSubscribe, BrokerIssue, SystemBroker, ArbiterBroker, Broker};


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(Result = "()")]
struct Welcome {}

fn from_addr(s: &str) -> SocketAddr {
    s.to_socket_addrs().expect("Could not parse seed node").next().unwrap()
}

#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: SocketAddr,
    #[structopt(parse(from_str = from_addr))]
    seed_nodes: Vec<SocketAddr>,
}

#[derive(RemoteActor)]
#[remote_messages(Welcome)]
struct OwnListener {
    count: usize,
    filename: String
}

impl OwnListener {
    const IDENTIFIER: &'static str = "own_listener";

    pub fn new(filename: String) -> Self {
        OwnListener {count: 0, filename}
    }
}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        self.subscribe_system_async::<ClusterLog>(ctx);
    }
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, remote_addr) => {
                RemoteAddr::new_from_key(
                    addr,
                    remote_addr.network_interface.unwrap(),
                    OwnListener::IDENTIFIER
                ).do_send(Box::new(Welcome {}));
            },
            ClusterLog::MemberLeft(_addr) => debug!("ClusterLog: MemberLeft")
        }
    }
}

impl Handler<Welcome> for OwnListener {
    type Result = ();

    fn handle(&mut self, _msg: Welcome, _ctx: &mut Context<Self>) -> Self::Result {
        self.count = self.count + 1;
        debug!("Welcome said {}x", self.count);
        if self.count == 100 {
            fs::write(format!("./{}", self.filename.as_str()), "100").expect("Unable to write file");
        }
    }
}

#[actix_rt::main]
async fn main() {
    env_logger::init();

    debug!("{}", "localhost:8000".to_socket_addrs().unwrap().next().unwrap());

    let args = Parameters::from_args();

    let cluster_listener = OwnListener::new(args.local_ip.to_string()).start();
    let _cluster = Cluster::new(
        args.local_ip.to_socket_addrs().unwrap().next().unwrap(),
        args.seed_nodes,
        vec![cluster_listener.clone().recipient()],
        vec![(cluster_listener.recipient(), OwnListener::IDENTIFIER)]);
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
