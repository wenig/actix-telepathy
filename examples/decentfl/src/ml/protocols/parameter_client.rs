use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use crate::ml::protocols::{CentralModelAggregation, ModelMessage};
use tch::Tensor;
use std::net::SocketAddr;


#[derive(RemoteActor)]
#[remote_messages(CentralModelAggregation)]
pub struct ParameterClient {
    model: Tensor,
    parent: Recipient<ModelMessage>,
    cluster: Addr<Cluster>,
    socket_addr: SocketAddr,
    server_addr: RemoteAddr,
}


impl ParameterClient {
    pub fn new(init_model: Tensor, parent: Recipient<ModelMessage>, cluster: Addr<Cluster>, socket_addr: SocketAddr, server_addr: RemoteAddr) -> Self {
        Self {
            model: init_model,
            parent,
            cluster,
            socket_addr,
            server_addr
        }
    }

    fn update_model(&mut self, model: Tensor) {
        let gradients = model.subtract(&self.model);

        self.server_addr.clone().do_send(Box::new(
            CentralModelAggregation {
                model: gradients,
                source: RemoteAddr::new_from_id(self.socket_addr, "ParameterClient")
            }
        ));
    }

    fn receive_global(&mut self, model: Tensor) {
        self.model = model.copy();
        let _r = self.parent.clone().do_send(ModelMessage::Response(model));
    }
}


impl Actor for ParameterClient {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        let own_addr = ctx.address().recipient();
        self.cluster.register_actor(own_addr, "ParameterClient");
    }
}


impl Handler<ModelMessage> for ParameterClient {
    type Result = ();

    fn handle(&mut self, msg: ModelMessage, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            ModelMessage::Request(model) => {
                self.update_model(model);
            },
            _ => ()
        }
    }
}


impl Handler<CentralModelAggregation> for ParameterClient {
    type Result = ();

    fn handle(&mut self, msg: CentralModelAggregation, _ctx: &mut Context<Self>) -> Self::Result {
        self.receive_global(msg.model)
    }
}
