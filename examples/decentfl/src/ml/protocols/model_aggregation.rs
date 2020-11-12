use actix::prelude::*;
use actix_telepathy::*;
use serde::{Serialize, Deserialize};
use tch::Tensor;
use crate::security::{GroupingClient, GroupingServer, FindGroup};


#[derive(Message)]
#[rtype("Result = ()")]
pub enum ModelMessage {
    Request(Tensor),
    Response(Tensor)
}

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub enum AggregationMessage {
    Encrypted(Tensor),
    Reveal(Tensor)
}


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
#[with_source(source)]
pub struct GroupingMessage {
    source: RemoteAddr
}


#[derive(RemoteActor)]
#[remote_messages(AggregationMessage)]
pub struct ModelAggregation {
    parent: Recipient<ModelMessage>,
    socket_addr: String,
    server_addr: RemoteAddr,
    grouping_client: Option<Addr<GroupingClient>>,
    grouping_server: Option<Addr<GroupingServer>>,
    current_group: Option<Vec<RemoteAddr>>,
    accepted: Vec<RemoteAddr>,
    own_model: Option<Tensor>,
    shares: Vec<Tensor>
}

impl ModelAggregation {
    pub fn new(parent: Recipient<ModelMessage>, socket_addr: String, server_addr: RemoteAddr) -> Self {
        Self {
            parent,
            socket_addr,
            server_addr,
            grouping_client: None,
            grouping_server: None,
            current_group: None,
            accepted: vec![],
            own_model: None,
            shares: vec![]
        }
    }

    fn start_protocol(&mut self, model: Tensor) {
        debug!("Start aggregation protocol");
        debug!("Get group");
        self.own_model = Some(model);
        self.grouping_client.expect("ModelAggregation Actor needs to be started").do_send(FindGroup::Request);
        debug!("Build sub cluster");
        debug!("(MASCOT)");
        debug!("Encrypt Model");
        debug!("Share Model");
        debug!("(Krum shares)");
        debug!("Reveal Aggregation");
    }

    fn build_sub_cluster(&mut self, group: Vec<RemoteAddr>) {
        self.current_group = Some(group);
        for partner in self.current_group.unwrap().iter() {
            partner.clone().do_send(Box::new(AggregationMessage::Grouping));
        }
    }

    fn accept_partner(&mut self, partner: RemoteAddr) {
        if self.current_group.unwrap().iter().any(|&i| i == partner) {
            self.accepted.push(partner);
        }
        if self.accepted.len() == self.current_group.unwrap().len() {
            self.share_encrypted_model()
        }
    }

    fn share_encrypted_model(&self) {

    }

    fn receive_shares(&mut self, share: Tensor) {
        self.shares.push(share);

        if self.shares.len() == self.current_group.expect("Current group should be set at that point").len() {
            // todo add krum
            let revealed = self.shares.iter().sum();
            for partner in self.current_group.unwrap() {
                partner.clone().do_send(Box::new(AggregationMessage::Reveal(revealed.copy())))
            }
            self.shares = vec![];
        }
    }

    fn receive_reveals(&mut self, share: Tensor) {
        self.shares.push(share);

        if self.shares.len() == self.current_group.expect("Current group should be set at that point").len() {
            let revealed = self.shares.iter().sum().div(self.shares.len());
            self.shares = vec![];
            self.own_model = None;

            self.parent.do_send(ModelMessage::Response(revealed));
        }
    }
}

impl Actor for ModelAggregation {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.grouping_client = Some(GroupingClient::new(
            ctx.address().recipient(),
            self.socket_addr.clone(),
            self.server_addr.clone()).start())
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

    fn handle(&mut self, msg: GroupingMessage, ctx: &mut Self::Context) -> Self::Result {
        self.accept_partner(msg.source);
    }
}


impl Handler<AggregationMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: AggregationMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            AggregationMessage::Encrypted(params) => self.receive_shares(params),
            AggregationMessage::Reveal(params) => self.receive_reveals(params)
        }
    }
}


impl Handler<FindGroup> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: FindGroup, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            FindGroup::Response(group) => self.build_sub_cluster(group),
            _ => ()
        }
    }
}