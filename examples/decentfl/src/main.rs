mod ml;
mod security;

use log::*;
use structopt::StructOpt;
use actix_rt;
use actix::prelude::*;
use actix_telepathy::*;
use security::{GroupingClient, GroupingServer, FindGroup};


#[derive(StructOpt, Debug)]
struct Parameters{
    local_addr: String,
    cluster_size: usize,
    seed_nodes: Option<String>,
}


#[derive(Message)]
#[rtype("Result = ()")]
struct ClusterAddr(Addr<Cluster>);


struct OwnListener {
    local_addr: String,
    server_addr: String,
    server_remote_addr: Option<RemoteAddr>,
    cluster_size: usize,
    current_size: usize,
    grouping_client: Option<Addr<GroupingClient>>,
    grouping_server: Option<Addr<GroupingServer>>,
    own_rec: Option<Recipient<FindGroup>>,
    cluster: Option<Addr<Cluster>>,
    cluster_full: bool
}

impl OwnListener {
    const IDENTIFIER: &'static str = "own_listener";

    pub fn new(local_addr: String, server_addr: String, cluster_size: usize) -> Self {
        OwnListener {
            local_addr,
            server_addr,
            server_remote_addr: None,
            cluster_size,
            current_size: 1,
            grouping_client: None,
            grouping_server: None,
            own_rec: None,
            cluster: None,
            cluster_full: false
        }
    }

    fn initiate_training(&mut self) -> () {
        if self.local_addr == self.server_addr {
            self.grouping_server = Some(GroupingServer::new(
                2,
                0
            ).start());
            self.cluster.clone().unwrap().register_actor(self.grouping_server.clone().unwrap().recipient(), "GroupingServer");
        } else {
            self.grouping_client = Some(GroupingClient::new(
                self.own_rec.clone().unwrap(),
                self.local_addr.clone(),
                self.server_remote_addr.clone().unwrap()
            ).start());
            self.cluster.clone().unwrap().register_actor(self.grouping_client.clone().unwrap().recipient(), "GroupingClient");
            self.grouping_client.clone().unwrap().do_send(FindGroup::Request);
        }
    }
}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_rec = Some(ctx.address().recipient());
    }
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, remote_addr) => {
                if addr == self.server_addr {
                    let server_addr = RemoteAddr::new_from_key(remote_addr.socket_addr, remote_addr.network_interface.unwrap(), "GroupingServer");
                    self.server_remote_addr = Some(server_addr)
                }
                debug!("ClusterLog: Member Joined");
                self.current_size = self.current_size + 1;
                if self.current_size == self.cluster_size {
                    self.cluster_full = true;

                    if self.cluster.is_some() {
                        self.initiate_training();
                    }
                }
            },
            ClusterLog::MemberLeft(_addr) => {
                debug!("ClusterLog: MemberLeft");
                self.current_size = self.current_size - 1;
            }
        }
    }
}

impl Handler<ClusterAddr> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterAddr, _ctx: &mut Context<Self>) -> Self::Result {
        self.cluster = Some(msg.0);
        if self.cluster_full {
            self.initiate_training()
        }
    }
}

impl Handler<FindGroup> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: FindGroup, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            FindGroup::Response(group) => {
                let s: String = group.into_iter().map(|x| format!("{}, ", x.socket_addr.clone())).collect();
                debug!("group {}", s)
            },
            _ => ()
        }
    }
}


#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args = Parameters::from_args();

    let cluster_listener = OwnListener::new(
        args.local_addr.clone(),
        "127.0.0.1:8000".to_string(),
        args.cluster_size
    ).start();
    let cluster = Cluster::new(
        args.local_addr,
        args.seed_nodes,
        vec![cluster_listener.clone().recipient()],
        vec![]);
    cluster_listener.do_send(ClusterAddr {0: cluster});
    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
