use log::*;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use serde::{Serialize, Deserialize};
//use glass_pumpkin::prime;
use glass_pumpkin::num_bigint::BigUint;
//use std::ops::Sub;
//use rand::Rng;
//use gcd::Gcd;


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
    p: BigUint,
    q: BigUint,
    n: BigUint,
    phi_n: i64,
    e: i64,
    d: i64,
    x: i64,
    b: i64,
    k: i64
}

#[allow(dead_code)]
impl ObliviousTransfer {
    /*pub fn new(prime_size: u64, field_size: u64, size: u64) -> Self {
        let mut rng = rand::thread_rng();
        let mut p = prime::new(prime_size as usize).expect("Could not create prime");
        let mut q = prime::new(prime_size as usize).expect("Could not create prime");

        while p == q {
            p = prime::new(prime_size as usize).expect("Could not create prime");
            q = prime::new(prime_size as usize).expect("Could not create prime");
        }
        let one = BigUint::from(1);
        let n = p.clone() * q.clone();
        let phi_n = (p - one.clone()) * (q - one.clone());
        let e = BigUint::from(rng.gen_range(2, phi_n.clone()));

        while e.gcd(&phi_n) != one {
            let e = rng.gen_range(2, phi_n.clone());
        };
    }*/
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