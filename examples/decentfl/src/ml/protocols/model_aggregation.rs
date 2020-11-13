use log::*;
use actix::prelude::*;
use actix_telepathy::*;
use serde::{Serialize, Deserialize};
use tch::{Tensor, IndexOp};
use crate::security::{GroupingClient, FindGroup, random_additive};
use std::ops::Div;


#[derive(Message)]
#[rtype("Result = ()")]
pub enum ModelMessage {
    Request(Tensor),
    Response(Tensor)
}

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub struct AggregationMessage {
    #[serde(with = "tch_serde::serde_tensor")]
    model: Tensor
}


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub struct EncryptionMessage {
    #[serde(with = "tch_serde::serde_tensor")]
    model: Tensor
}


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
#[with_source(source)]
pub struct GroupingMessage {
    source: RemoteAddr
}


#[derive(RemoteActor)]
#[remote_messages(AggregationMessage, GroupingMessage, EncryptionMessage)]
pub struct ModelAggregation {
    own_addr: Option<Addr<ModelAggregation>>,
    parent: Recipient<ModelMessage>,
    cluster: Addr<Cluster>,
    socket_addr: String,
    server_addr: RemoteAddr,
    grouping_client: Option<Addr<GroupingClient>>,
    current_group: Option<Vec<RemoteAddr>>,
    accepted: Vec<RemoteAddr>,
    own_model: Option<Tensor>,
    shares: Vec<Tensor>
}

// todo register at cluster
impl ModelAggregation {
    pub fn new(parent: Recipient<ModelMessage>, cluster: Addr<Cluster>, socket_addr: String, server_addr: RemoteAddr) -> Self {
        Self {
            own_addr: None,
            parent,
            cluster,
            socket_addr,
            server_addr,
            grouping_client: None,
            current_group: None,
            accepted: vec![],
            own_model: None,
            shares: vec![]
        }
    }

    fn start_protocol(&mut self, model: Tensor) {
        self.own_model = Some(model);
        self.grouping_client.clone().expect("ModelAggregation Actor needs to be started").do_send(FindGroup::Request);
    }

    fn build_sub_cluster(&mut self) {
        debug!("build sub cluster");
        for partner in self.current_group.as_ref().unwrap().iter() {
            if partner.socket_addr != self.socket_addr {
                partner.clone().do_send(Box::new(GroupingMessage {
                    source: RemoteAddr::new_from_id(self.socket_addr.clone(), "ModelAggregation")
                }));
            }
        }
    }

    fn accept_partner(&mut self, partner: RemoteAddr) {
        debug!("accept partner");
        if self.current_group.as_ref().unwrap().iter().any(|i| i.clone() == partner) {
            self.accepted.push(partner);
        }
        if self.accepted.len() == (self.current_group.as_ref().unwrap().len() - 1) {
            self.share_encrypted_model()
        }
    }

    fn share_encrypted_model(&mut self) {
        let encrypted_models = random_additive(
            self.own_model.as_ref().expect("Model should be set at that point"),
            self.current_group.as_ref().expect("Current group should be set at that point").len() as i64
        );
        let len = encrypted_models.size().get(0).expect("Should have more than 0 dimensions").clone();
        self.shares = vec![self.own_model.as_ref().unwrap().empty_like(); len as usize];
        for i in 0..len {
            let mut partner = self.current_group.as_ref().unwrap().get(i as usize).unwrap().clone();

            if partner.socket_addr == self.own_addr {
                self.shares.
            } else {
                partner.do_send(Box::new(EncryptionMessage { model: encrypted_models.i(i).copy()}));
            }

        }
    }

    fn receive_shares(&mut self, share: Tensor) {
        // todo push share at right position depending on current_group
        self.shares.push(share);

        if self.shares.len() == self.current_group.as_ref().expect("Current group should be set at that point").len() {
            // todo add krum
            let revealed: Tensor = self.shares.iter().sum();
            for partner in self.current_group.as_ref().unwrap() {
                partner.clone().do_send(Box::new(AggregationMessage{ model: revealed.copy() }))
            }
            self.shares = vec![];
        }
    }

    fn receive_reveals(&mut self, share: Tensor) {
        self.shares.push(share);

        if self.shares.len() == self.current_group.as_ref().expect("Current group should be set at that point").len() {
            let revealed: Tensor = self.shares.iter().sum::<Tensor>().div(self.shares.len() as i64);
            self.shares = vec![];
            self.own_model = None;

            let _r = self.parent.do_send(ModelMessage::Response(revealed));
        }
    }
}

impl Actor for ModelAggregation {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
        self.grouping_client = Some(GroupingClient::new(
            self.own_addr.clone().unwrap().recipient(),
            self.socket_addr.clone(),
            self.server_addr.clone()).start());
        self.cluster.register_actor(self.grouping_client.clone().unwrap().recipient(), "GroupingClient");
        self.cluster.register_actor(self.own_addr.clone().unwrap().recipient(), "ModelAggregation");
    }
}

impl Handler<ModelMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: ModelMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            ModelMessage::Request(model) => self.start_protocol(model),
            _ => ()
        }
    }
}


impl Handler<GroupingMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: GroupingMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.accept_partner(msg.source);
    }
}


impl Handler<EncryptionMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: EncryptionMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.receive_shares(msg.model)
    }
}


impl Handler<AggregationMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: AggregationMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.receive_reveals(msg.model)
    }
}


impl Handler<FindGroup> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: FindGroup, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            FindGroup::Response(group) => {
                self.current_group = Some(group.clone().into_iter().map(|mut x| {
                    x.change_id("ModelAggregation".to_string());
                    x
                }).collect());
                self.cluster.do_send(NodeResolving::VecRequest(group.into_iter().map(|x| x.socket_addr).collect(), self.own_addr.clone().unwrap().recipient()))
            },
            _ => ()
        }
    }
}


impl Handler<NodeResolving> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: NodeResolving, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            NodeResolving::VecResponse(group) => {
                let current_group = self.current_group.as_mut().expect("Group should be set at that point");
                for i in 0..group.len() {
                    let remote = current_group.get_mut(i).unwrap();
                    let node = group.get(i).unwrap().clone();
                    remote.network_interface = node;
                }
                self.build_sub_cluster()
            },
            _ => ()
        }
    }
}