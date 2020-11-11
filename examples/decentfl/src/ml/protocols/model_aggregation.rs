use actix::prelude::*;
use actix_telepathy::*;
use serde::{Serialize, Deserialize};
use crate::ml::model::Net;


#[derive(Message)]
#[rtype("Result = ()")]
pub enum ModelMessage {
    Request(Net),
    Response(Net)
}

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub struct AggregationMessage {
    model: Net
}


#[derive(RemoteActor)]
#[remote_messages()]
pub struct ModelAggregation {
    parent: Recipient<ModelMessage>
}

impl ModelAggregation {
    pub fn new(parent: Recipient<ModelMessage>) -> Self {
        Self { parent }
    }

    fn start_protocol(&mut self, model: Net) {
        debug!("Start aggregation protocol");
        debug!("Get group");
        debug!("Build sub cluster");
        debug!("(MASCOT)");
        debug!("Encrypt Model");
        debug!("Share Model");
        debug!("(Krum shares)");
        debug!("Reveal Aggregation");
    }
}

impl Actor for ModelAggregation {
    type Context = Context<Self>;
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

impl Handler<AggregationMessage> for ModelAggregation {
    type Result = ();

    fn handle(&mut self, msg: AggregationMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.parent.do_send(ModelMessage::Response(msg.model));
    }
}