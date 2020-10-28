#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::*;
use actix::{System, Actor, Handler, Context, Supervisor, Supervised};
use structopt::StructOpt;
use log::Level;


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Option<String>,
}

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
                //remote_addr.do_send_remote(RemoteMessage::new(Box::new("welcome")));
            },
            ClusterLog::MemberLeft(addr) => debug!("ClusterLog: MemberLeft")
        }
    }
}


fn main() {
    env_logger::init();

    let args = Parameters::from_args();
    let local_ip = args.local_ip.to_lowercase().trim().to_owned();
    let seed_nodes = args.seed_nodes.map(|n| n.to_lowercase().trim().to_owned());

    //let sys = actix::System::new("remote-example");

    actix::System::run(|| {
        let cluster_listener = Supervisor::start(|_| OwnListener {});
        let cluster = Cluster::new(local_ip, seed_nodes, vec![cluster_listener.recipient()]);
    });
    //let _ = sys.run();
}
