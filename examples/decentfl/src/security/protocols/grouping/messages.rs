use actix::prelude::*;
use actix_telepathy::*;
use serde::{Serialize, Deserialize};


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub enum GroupingMessage {
    Request(RemoteAddr),
    Response(Vec<RemoteAddr>)
}


#[derive(Message)]
#[rtype("Result = ()")]
pub enum FindGroup {
    Request,
    Response(Vec<RemoteAddr>)
}
