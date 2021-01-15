use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use serde::{Serialize, Deserialize};
use glass_pumpkin::prime;
use glass_pumpkin::num_bigint::{BigInt, RandBigInt};
use num_traits::ToPrimitive;
use std::ops::{Sub, Add};
use gcd::Gcd;
use tch::{Tensor, Kind, Device, IndexOp};
use crate::security::protocols::oblivious_transfer::messages::{OTMessage1Request, OTMessage1Response, OTMessage2Request};
use rand::Rng;
use crate::security::protocols::oblivious_transfer::{OTDone, ObliviousTransferSender};
use rand::prelude::ThreadRng;
use rand::rngs::OsRng;
use tch::nn::Optimizer;


enum State {
    INITIAL,
    DECISION_RESPONDED,
    PROTOCOL_CARRIED_OUT
}


#[derive(RemoteActor)]
#[remote_messages(OTMessage1Request, OTMessage2Request)]
pub struct ObliviousTransferReceiver {
    parent: Recipient<OTDone>,
    sender: Option<AnyAddr<ObliviousTransferSender>>,
    b: u8,
    k: Option<BigInt>,
    state: State
}


impl ObliviousTransferReceiver {
    pub fn new(parent: Recipient<OTDone>, size: i64, b: u8) -> Self {
        Self {
            parent,
            sender: None,
            b,
            k: None,
            state: State::INITIAL
        }
    }

    fn receive_public_keys(&mut self, msg: OTMessage1Request) {
        self.sender = Some(msg.source);
        debug!("public keys received");

        let n = msg.n;
        let e = msg.e;
        let x = msg.x;
        let mut rng = OsRng::default();

        self.k = Some(rng.gen_bigint_range(&BigInt::from(2 as u16), &n));
        let pow_add = self.k.as_ref().unwrap().modpow(&e, &n);
        let v = x[self.b.clone() as usize].as_slice().into_iter().map(|x_| (x_ % &n).add(&pow_add)).collect();

        let msg = OTMessage1Response { v };

        match self.sender.as_ref().unwrap() {
            AnyAddr::Local(addr) => addr.do_send(msg),
            AnyAddr::Remote(addr) => addr.clone().do_send(Box::new(msg))
        };

        self.state = State::DECISION_RESPONDED;
    }

    fn receive_mprimelist(&mut self, msg: OTMessage2Request) {
        debug!("received mprimlist");

        let mprimelist = msg.mprimelist;
        let k = self.k.as_ref().unwrap();
        let m: Vec<BigInt> = mprimelist[self.b.clone() as usize].as_slice().into_iter().map(|x| x - k).collect();
        let m: Vec<i64> = m.into_iter().map(|x| x.to_i64().expect("Number should be in i64 range")).collect();
        let tensor = Tensor::of_slice(m.as_slice());
        self.parent.do_send(OTDone::Receiver(tensor, self.sender.as_ref().unwrap().clone()));
        self.state = State::PROTOCOL_CARRIED_OUT;
        debug!("sent messages to parent");
    }
}


impl Actor for ObliviousTransferReceiver {
    type Context = Context<Self>;
}


impl Handler<OTMessage1Request> for ObliviousTransferReceiver {
    type Result = ();

    fn handle(&mut self, msg: OTMessage1Request, ctx: &mut Self::Context) -> Self::Result {
        match self.state {
            State::INITIAL => self.receive_public_keys(msg),
            _ => ctx.address().do_send(msg)
        }
    }
}


impl Handler<OTMessage2Request> for ObliviousTransferReceiver {
    type Result = ();

    fn handle(&mut self, msg: OTMessage2Request, ctx: &mut Self::Context) -> Self::Result {
        match self.state {
            State::DECISION_RESPONDED => self.receive_mprimelist(msg),
            _ => ctx.address().do_send(msg)
        }
    }
}
