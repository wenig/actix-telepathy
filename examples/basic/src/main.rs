mod serializer;

#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::*;
use actix::{System, Handler, Actor, Context, Supervisor, Supervised, Message, AsyncContext, Recipient};
use structopt::StructOpt;
use log::Level;
use std::str::FromStr;
use std::any::TypeId;
use serde::{Serialize, Deserialize};
use serializer::MySerializer;
use std::thread::sleep;
use std::time::Duration;
use tokio;

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(Result = "()")]
struct Welcome {}


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Option<String>,
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

    fn handle(&mut self, msg: ClusterLog, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, mut remote_addr) => {
                RemoteAddr::new_from_key(
                    addr,
                    remote_addr.network_interface.unwrap(),
                    OwnListener::IDENTIFIER
                ).do_send(Box::new(Welcome {}));
            },
            ClusterLog::MemberLeft(addr) => debug!("ClusterLog: MemberLeft")
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
    let local_ip = args.local_ip.to_lowercase().trim().to_owned();
    let seed_nodes = args.seed_nodes.map(|n| n.to_lowercase().trim().to_owned());

    let cluster_listener = OwnListener::new().start();
    let _cluster = Cluster::new(
        local_ip,
        seed_nodes,
        vec![cluster_listener.clone().recipient()],
        vec![(cluster_listener.recipient(), OwnListener::IDENTIFIER)]);
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
