use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use serde::{Serialize, Deserialize};
use glass_pumpkin::prime;
use glass_pumpkin::num_bigint::{BigInt, RandBigInt, ToBigInt};
use num_integer::Integer;
use std::ops::{Sub, Add};
use rand::Rng;
use gcd::Gcd;
use tch::{Tensor, Kind, Device, IndexOp};
use crate::security::protocols::oblivious_transfer::messages::{OTMessage1Request, OTMessage1Response, OTMessage2Request, OTStart};
use crate::security::protocols::oblivious_transfer::{OTDone, ObliviousTransferReceiver};
use rand::rngs::OsRng;
use crate::security::protocols::oblivious_transfer::bigint_extensions::NegModPow;


enum State {
    INITIAL,
    PROTOCOL_STARTED,
    PROTOCOL_CARRIED_OUT
}


#[derive(RemoteActor)]
#[remote_messages(OTMessage1Response)]
pub struct ObliviousTransferSender {
    parent: Recipient<OTDone>,
    receiver: AnyAddr<ObliviousTransferReceiver>,
    messages: Vec<BigInt>,
    n: BigInt,
    e: BigInt,
    d: BigInt,
    x: [Vec<BigInt>; 2],
    state: State,
    own_addr: Option<AnyAddr<Self>>
}


impl ObliviousTransferSender {
    pub fn new(parent: Recipient<OTDone>, prime_size: u64, field_size: u64, size: i64, receiver: AnyAddr<ObliviousTransferReceiver>, messages: Tensor) -> Self {
        assert_eq!(messages.size(), vec![1, 2]);

        let mut rng = OsRng::default();
        let mut p = prime::new(prime_size as usize).expect("Could not create prime");
        let mut q = prime::new(prime_size as usize).expect("Could not create prime");

        while p == q {
            p = prime::new(prime_size as usize).expect("Could not create prime");
            q = prime::new(prime_size as usize).expect("Could not create prime");
        }

        let p = p.to_bigint().unwrap();
        let q = q.to_bigint().unwrap();
        let one = BigInt::from(1 as u16);
        let two = BigInt::from(2 as u16);
        let n = p.clone() * q.clone();
        let phi_n = (p - &one) * (q - &one);
        let mut e = rng.gen_bigint_range(&two, &phi_n);

        while e.gcd(&phi_n) != one {
            e = rng.gen_bigint_range(&two, &phi_n);
        };

        let d = e.neg_modpow(&one, &phi_n);
        let mut x = [vec![], vec![]];
        let lbound = &BigInt::from(2);
        let ubound = &BigInt::from(field_size);
        for c in 0..2 {
            for _ in 0..size {
                x[c].push(rng.gen_bigint_range(lbound, ubound));
            }
        }

        let messages = vec![
            BigInt::from(messages.int64_value(&[0, 0]) as u64),
            BigInt::from(messages.int64_value(&[0, 1]) as u64)
        ];

        println!("Hello world");

        Self {
            parent,
            receiver,
            messages,
            n,
            e,
            d,
            x,
            state: State::INITIAL
        }
    }

    fn start_protocol(&mut self) {
        debug!("start protocol");
        self.receiver.clone().do_send(Box::new(
            OTMessage1Request {
                n: self.n.clone(),
                e: self.e.clone(),
                x: self.x.clone(),
                source: self.own_address.unwrap
            }
        ));
        debug!("m1 sent");
        self.state = State::PROTOCOL_STARTED
    }

    fn receive_response(&mut self, msg: OTMessage1Response) {
        let v = msg.v;
        let klist: [Vec<BigInt>; 2] = [
            v.iter().zip(self.x[0].iter()).map(|(v_, x_)| {
                ((v_ - x_) % &self.n).modpow(&self.d, &self.n)
            }).collect(),
            v.iter().zip(self.x[1].iter()).map(|(v_, x_)| {
                ((v_ - x_) % &self.n).modpow(&self.d, &self.n)
            }).collect(),
        ];
        let mprimelist = [
            klist[0].as_slice().into_iter().map(|x| (x + &self.messages[0]) % &self.n).collect(),
            klist[1].as_slice().into_iter().map(|x| (x + &self.messages[1]) % &self.n).collect()
        ];
        self.receiver.clone().do_send(Box::new(
            OTMessage2Request { mprimelist }
        ));
        self.state = State::PROTOCOL_CARRIED_OUT
    }
}


impl Actor for ObliviousTransferSender {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        match self.receiver {
            AnyAddr::Local(addr) => {}
            AnyAddr::Remote(addr) => { self.own_addr = Some(AnyAddr::Remote()) }
        }
        self.own_addr = Some(ctx)
    }
}


impl Handler<OTStart> for ObliviousTransferSender {
    type Result = ();

    fn handle(&mut self, _msg: OTStart, ctx: &mut Self::Context) -> Self::Result {
        self.start_protocol();
    }
}


impl Handler<OTMessage1Response> for ObliviousTransferSender {
    type Result = ();

    fn handle(&mut self, msg: OTMessage1Response, ctx: &mut Self::Context) -> Self::Result {
        match self.state {
            State::PROTOCOL_STARTED => {
                self.receive_response(msg)
            },
            _ => ctx.address().do_send(msg)
        }
    }
}
