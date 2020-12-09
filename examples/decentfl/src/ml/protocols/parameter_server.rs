use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use tch::{Tensor};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
#[with_source(source)]
pub struct CentralModelAggregation {
    #[serde(with = "tch_serde::serde_tensor")]
    pub model: Tensor,
    pub source: RemoteAddr
}


#[derive(RemoteActor)]
#[remote_messages(CentralModelAggregation)]
pub struct ParameterServer {
    global_model: Tensor,
    n_clients: usize,
    participated_clients: HashSet<RemoteAddr>,
    socket_addr: SocketAddr
}


impl Actor for ParameterServer {
    type Context = Context<Self>;
}


impl ParameterServer {
    pub fn new(global_model: Tensor, n_clients: usize, socket_addr: SocketAddr) -> Self {
        ParameterServer {
            global_model,
            n_clients,
            participated_clients: HashSet::new(),
            socket_addr
        }
    }

    fn eventually_send_back_aggregate(&mut self) {
        if self.participated_clients.len() == self.n_clients {
            let own_addr = RemoteAddr::new_from_id(self.socket_addr, "ParameterServer");

            for client in self.participated_clients.iter() {
                client.clone().do_send(Box::new(
                    CentralModelAggregation {
                        model: self.global_model.copy(),
                        source: own_addr.clone()
                    }
                ));
            }

            self.participated_clients.clear();
        }
    }
}


impl Handler<CentralModelAggregation> for ParameterServer {
    type Result = ();

    fn handle(&mut self, msg: CentralModelAggregation, _ctx: &mut Context<Self>) -> Self::Result {
        if self.participated_clients.insert(msg.source) {

            let _ = self.global_model.add_out(&self.global_model, &msg.model.divide1(self.n_clients as f64));
        }

        self.eventually_send_back_aggregate();
    }
}
