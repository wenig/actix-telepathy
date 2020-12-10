use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use crate::ml::{Training, Addresses, Epoch, ModelAggregation, ParameterClient};
use std::net::SocketAddr;
use tch::Tensor;


#[derive(Message)]
#[rtype("Result = ()")]
pub struct ClusterAddr(Addr<Cluster>);

impl ClusterAddr {
    pub fn new(cluster: Addr<Cluster>) -> Self {
        ClusterAddr {0: cluster}
    }
}


pub struct OwnListener {
    local_addr: SocketAddr,
    server_addr: SocketAddr,
    server_remote_addr: Option<RemoteAddr>,
    cluster_size: usize,
    current_size: usize,
    cluster: Option<Addr<Cluster>>,
    cluster_full: bool,
    training: Option<Addr<Training>>,
    centralized: bool,
    init_model: Option<Tensor>,
    group_size: usize
}

impl OwnListener {
    pub fn new(local_addr: SocketAddr, server_addr: SocketAddr, cluster_size: usize, training: Option<Addr<Training>>, centralized: bool, init_model: Option<Tensor>, group_size: usize) -> Self {
        OwnListener {
            local_addr,
            server_addr,
            server_remote_addr: None,
            cluster_size,
            current_size: 1,
            cluster: None,
            cluster_full: false,
            training,
            centralized,
            init_model,
            group_size
        }
    }

    fn initiate_training(&self) -> () {
        match self.training.clone() {
            Some(t) => t.do_send(Epoch {}),
            None => {}
        }
    }
}

impl ClusterListener for OwnListener {}
impl Supervised for OwnListener {}

impl Actor for OwnListener {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
    }
}

impl Handler<ClusterLog> for OwnListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ClusterLog::NewMember(addr, remote_addr) => {
                if self.training.is_some() & (addr == self.server_addr) {
                    let server_id = if self.centralized {
                        "ParameterServer"
                    } else {
                        "GroupingServer"
                    };
                    let server_addr = RemoteAddr::new_from_key(remote_addr.socket_addr, remote_addr.network_interface.unwrap(), server_id);
                    self.server_remote_addr = Some(server_addr);

                    let model_aggregation = if self.centralized {
                        ParameterClient::new(
                            self.init_model.as_ref().unwrap().copy(),
                            self.training.clone().unwrap().recipient(),
                            self.cluster.clone().unwrap(),
                            self.local_addr.clone(),
                            self.server_remote_addr.clone().unwrap(),
                        ).start().recipient()
                    } else {
                        ModelAggregation::new(
                            self.training.clone().unwrap().recipient(),
                            self.cluster.clone().unwrap(),
                            self.local_addr.clone(),
                            self.server_remote_addr.clone().unwrap(),
                            self.group_size
                        ).start().recipient()
                    };

                    self.training.as_ref().unwrap().do_send(Addresses::new(
                        model_aggregation
                    ))
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
