use log::*;
use actix::prelude::*;
use actix_telepathy::*;
use serde::{Deserialize, Serialize};
use tch::Tensor;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
struct SubClusterMessage {

}


#[derive(Message)]
#[rtype("Result = ()")]
enum InternalMessage {
    Scatter(Tensor),
    ScatterResponse(Tensor),
}


#[derive(RemoteActor)]
#[remote_messages()]
pub struct SubCluster {
    size: usize,
    partners: Vec<RemoteAddr>
}

impl SubCluster {
    pub fn new(size: usize) -> Self {
        SubCluster {size, partners: vec![]}
    }

    pub fn find_partners(&self) {

    }
}

impl Actor for SubCluster {
    type Context = Context<Self>;
}

impl Handler<SubClusterMessage> for SubCluster {
    type Result = ();

    fn handle(&mut self, msg: SubClusterMessage, ctx: &mut Self::Context) -> Self::Result {
        debug!("received sub-cluster-message");
    }
}

impl Handler<InternalMessage> for SubCluster {
    type Result = ();

    fn handle(&mut self, msg: InternalMessage, _ctx: &mut Self::Context) -> Self::Result {
        debug!("scatter...")
    }
}

pub trait CollectiveApi {
    fn receive(&self, recvbuf: &Tensor, src: RemoteAddr);
    fn allreduce(&self, sendbuf: &Tensor, recvbuf: &Tensor);
    fn scatter(&self, sendbuf: &Tensor, recvbuf: &Tensor);
    fn allscatter(&self, sendbuf: &Tensor, recvbuf: &Tensor);
}

impl CollectiveApi for Addr<SubCluster> {
    fn receive(&self, recvbuf: &Tensor, src: RemoteAddr) {

    }

    fn allreduce(&self, sendbuf: &Tensor, recvbuf: &Tensor) {

    }

    fn scatter(&self, sendbuf: &Tensor, recvbuf: &Tensor) {

    }

    fn allscatter(&self, sendbuf: &Tensor, recvbuf: &Tensor) {

    }
}
