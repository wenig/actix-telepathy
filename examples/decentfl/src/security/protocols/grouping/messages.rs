use actix::prelude::*;
use actix_telepathy::*;
use serde::{Serialize, Deserialize};


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
#[with_source(source)]
pub struct GroupingRequest {
    pub source: RemoteAddr
}

#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype("Result = ()")]
pub struct GroupingResponse {
    pub group: Vec<RemoteAddr>
}


#[derive(Message)]
#[rtype("Result = ()")]
pub enum FindGroup {
    Request,
    Response(Vec<RemoteAddr>)
}
