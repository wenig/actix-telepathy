use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use glass_pumpkin::num_bigint::BigInt;
use tch::Tensor;
use serde::{Serialize, Deserialize};
use crate::security::protocols::oblivious_transfer::ObliviousTransferSender;


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
pub struct OTMessage1Request {
    pub n: BigInt,
    pub e: BigInt,
    pub x: [Vec<BigInt>; 2]
    pub source: AnyAddr<ObliviousTransferSender>
}


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
pub struct OTMessage1Response {
    pub v: Vec<BigInt>
}


#[derive(Message, Serialize, Deserialize, RemoteMessage)]
#[rtype(result = "()")]
pub struct OTMessage2Request {
    pub mprimelist: [Vec<BigInt>; 2]
}

#[derive(Message)]
#[rtype(result = "()")]
pub enum OTDone {
    Sender,
    Receiver(Tensor, RemoteAddr)
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct OTStart {}
