use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use serde::{Serialize, Deserialize};


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
enum OTMessage {
    Sender0(),
    Receiver0(),
    Sender1(),
    Receiver1()
}


#[derive(RemoteActor)]
#[remote_messages(OTMessage)]
#[allow(dead_code)]
pub struct ObliviousTransfer {
    prime_size: i64,
    field_size: i64,
    p: i64,
    q: i64,
    n: i64,
    phi_n: i64,
    e: i64,
    d: i64,
    x: i64,
    b: i64,
    k: i64
}


impl ObliviousTransfer {
}


impl Actor for ObliviousTransfer {
    type Context = Context<Self>;
}


impl Handler<OTMessage> for ObliviousTransfer {
    type Result = ();

    fn handle(&mut self, msg: OTMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            OTMessage::Sender0() => {},
            OTMessage::Receiver0() => {},
            OTMessage::Sender1() => {},
            OTMessage::Receiver1() => {}
        }
    }
}