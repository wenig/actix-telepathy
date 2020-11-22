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
use std::thread::sleep;
use actix::clock::Duration;

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(Result = "()")]
struct Welcome {}


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Vec<String>,
}

#[derive(RemoteActor)]
#[remote_messages(Welcome)]
struct OwnListener {}

impl OwnListener {
    const IDENTIFIER: &'static str = "own_listener";

    pub fn new() -> Self {
        OwnListener {}
    }
}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;
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

    fn handle(&mut self, msg: Welcome, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Welcome said {} ", msg.get_identifier())
    }
}

#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args = Parameters::from_args();

    let cluster_listener = OwnListener::new().start();
    let _cluster = Cluster::new(
        args.local_ip,
        args.seed_nodes,
        vec![cluster_listener.clone().recipient()],
        vec![(cluster_listener.recipient(), OwnListener::IDENTIFIER)]);
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
