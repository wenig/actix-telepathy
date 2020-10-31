#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::*;
use actix::{System, Handler, Actor, Context, Supervisor, Supervised, Message, AsyncContext};
use structopt::StructOpt;
use log::Level;
use std::str::FromStr;
use std::any::TypeId;

#[derive(Message)]
#[rtype(Result = "()")]
struct Welcome {}

impl Sendable for Welcome {
    const IDENTIFIER: String = "welcome".to_string();
}

impl ToString for Welcome {
    fn to_string(&self) -> String {
        format!("{{\"identifier\": \"{}\"}}", Self::IDENTIFIER)
    }
}

impl FromStr for Welcome {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Welcome {})
    }
}


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Option<String>,
}

#[derive(RemoteActor)]
#[remote_messages(Welcome)]
struct OwnListener {}

impl OwnListener {
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
                ctx.address().do_send(RemoteMessage::new(remote_addr, Box::new(Welcome {})));
            },
            ClusterLog::MemberLeft(addr) => debug!("ClusterLog: MemberLeft")
        }
    }
}

impl Handler<Welcome> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: Welcome, _ctx: &mut Context<Self>) -> Self::Result {
        debug!("Welcome said")
    }
}


fn test(s: &String) {
    debug!("{}", s)
}


fn main() {
    env_logger::init();

    let args = Parameters::from_args();
    let local_ip = args.local_ip.to_lowercase().trim().to_owned();
    let seed_nodes = args.seed_nodes.map(|n| n.to_lowercase().trim().to_owned());

    //let sys = actix::System::new("remote-example");

    actix::System::run(|| {
        let cluster_listener = Supervisor::start(|_| OwnListener::new());
        let cluster = Cluster::new(local_ip, seed_nodes, vec![cluster_listener.clone().recipient()]);
        cluster.register_actor(cluster_listener.recipient());
    });
    //let _ = sys.run();
}
