mod one_factor;
mod messages;

use log::*;
use actix::prelude::*;
use actix_telepathy::*;
use std::net::SocketAddr;
use std::default::Default;
use std::fmt::Binary;
use tch::{Tensor, Kind, Device};
use crate::security::protocols::oblivious_transfer::{ObliviousTransferSender, ObliviousTransferReceiver, OTDone};
use crate::security::protocols::mascot::one_factor::{OneFactor, OneFactorCouple};
use std::env::join_paths;
use serde::export::fmt::Display;
use serde::export::Formatter;
use crate::security::protocols::mascot::messages::BeaverTriple;


enum State {
    INITIAL,
    ACTIVE,
    PASSIVE,
    FINISHED
}


pub struct Mascot {
    parent: Recipient<BeaverTriple>,
    cluster: Addr<Cluster>,
    own_addr: Option<Addr<Mascot>>,
    socket_addr: SocketAddr,
    peers: Vec<RemoteAddr>,
    own_position: usize,
    n_triples: i64,
    field_size: u64,
    prime_size: u64,
    a: Tensor,
    b: Tensor,
    x: Vec<i64>,
    y: Vec<i64>,
    coupler: OneFactor,
    state: State,
    done_counter: usize
}


impl Default for Mascot {
    fn default() -> Self {
        Mascot {
            field_size: 1000000,
            prime_size: 2048,
            state: State::INITIAL,
            done_counter: 0,
            ..Default::default()
        }
    }
}


impl Mascot {
    pub fn new(parent: Recipient<BeaverTriple>, socket_addr: SocketAddr, peers: Vec<RemoteAddr>, n_triples: i64) -> Self {
        Self::new_advanced(parent, socket_addr, peers, n_triples, Default::default(), Default::default())
    }

    pub fn new_advanced(parent: Recipient<BeaverTriple>, socket_addr: SocketAddr, peers: Vec<RemoteAddr>, n_triples: i64, field_size: u64, prime_size: u64) -> Self {
        let a = Tensor::randint1(2, field_size as i64, &[n_triples as i64], (Kind::Int64, Device::Cpu));
        let b = Tensor::randint1(2, field_size as i64, &[1], (Kind::Int64, Device::Cpu));
        let own_position = peers.iter().position(|x| x.socket_addr == socket_addr).expect("Own SocketAddr is not part of peers!");
        let n_peers = peers.len();
        let x = vec![0; n_peers];
        let y = x.clone();

        Self {
            parent,
            cluster: Cluster::from_registry(),
            own_addr: None,
            socket_addr,
            peers,
            own_position,
            n_triples,
            field_size,
            prime_size,
            a,
            b,
            x,
            y,
            coupler: OneFactor::new(n_peers),
            ..Default::default()
        }
    }

    fn count_bit_size(&mut self) -> usize {
        format!("{:b}", self.field_size).len()
    }

    fn compose_to_int(x: Tensor) -> i64 {
        let i = Tensor::arange(x.size()[0], (x.kind(), x.device()));
        x.dot(&Tensor::pow2(2, &i)).int64_value(&[0])
    }

    fn decompose_int2vec(x: i64, bit_size: usize) -> Vec<i8> {
        let bit_array: Vec<i8> = format!("{:b}", x).chars().map(|c| c.to_digit(10).unwrap() as i8).collect();
        let mut diff_array: Vec<i8> = vec![0; bit_size - bit_array.len()];
        diff_array.extend(bit_array);
        diff_array
    }

    fn active(&mut self, receiver: usize) {
        self.state = State::ACTIVE;
        let mut bits: Vec<Tensor> = vec![];
        let mut peer = self.peers.get(receiver).unwrap().clone();
        peer.change_id(format!("ObliviousTransferReceiver_{}", receiver));
        // todo don't create new OTSender for each bit
        for _ in 0..self.count_bit_size() {
            let m = Tensor::randint_like1(&self.a, 2, self.field_size as i64);
            let ot = ObliviousTransferSender::new(
                self.own_addr.clone().unwrap().recipient(),
                self.prime_size,
                self.field_size,
                self.n_triples,
                peer.clone(),
                Tensor::stack(&[m.copy(), m.copy() + self.a.as_ref()], 1)
            ).start();
            bits.push(m);
        }
        self.x[receiver] = -Mascot::compose_to_int(Tensor::stack(bits.as_slice(), 0));
    }

    fn passive(&mut self, sender: usize) {
        self.state = State::PASSIVE;
        let bit_array = Mascot::decompose_int2vec(self.b.int64_value(&[0]), self.count_bit_size());
        let mut peer = self.peers.get(sender).unwrap().clone();
        peer.change_id(format!("ObliviousTransferSender_{}", sender));
        for bi in bit_array.iter() {
            let _ot = ObliviousTransferReceiver::new(
                self.own_addr.clone().unwrap().recipient(),
                self.n_triples,
                peer.clone(),
                bi.clone() as u8
            ).start();
        }
    }

    fn next_round(&mut self) {
        let couple: Option<OneFactorCouple> = self.coupler.next();
        match couple {
            Some(couple) => {
                if couple.active == self.own_position {
                    self.active(couple.passive)
                } else if couple.passive == self.own_position {
                    self.passive(couple.active)
                }
            },
            None => self.return_beaver_triples()
        }
    }

    fn return_beaver_triples(&mut self) {
        let mut c = self.a.copy() * self.b.copy();
        let decrypt_term: i64 = self.x.iter().sum::<i64>() + self.y.iter().sum::<i64>();
        c = c + decrypt_term;
        self.parent.do_send(BeaverTriple {a: self.a.copy(), b: self.b.copy(), c});
    }

    fn check_counter(&mut self) {
        self.done_counter = self.done_counter + 1;
        if self.done_counter == self.peers.len() - 1 {
            self.done_counter = 0;
            self.next_round();
        }
    }

    fn sender_done(&mut self) {
        match self.state {
            State::ACTIVE => {
                self.check_counter()
            },
            _ => error!("Received OTDone::Sender in wrong state")
        }
    }

    fn receiver_done(&mut self, tensor: Tensor, sender: RemoteAddr) {
        match self.state {
            State::PASSIVE => {
                let y_position = self.peers.iter()
                    .position(|x| sender.socket_addr == x.socket_addr)
                    .expect("Sender is not part of peers sub-group");
                self.y[y_position] = Mascot::compose_to_int(tensor);
                self.check_counter()
            },
            _ => error!("Received OTDone::Receiver in wrong state")
        }
    }
}

impl Actor for Mascot {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.own_addr = Some(ctx.address());
    }
}

impl Handler<OTDone> for Mascot {
    type Result = ();

    fn handle(&mut self, msg: OTDone, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            OTDone::Sender => self.sender_done(),
            OTDone::Receiver(tensor, sender_address)
                => self.receiver_done(tensor, sender_address)
        }
    }
}


// ----------------------------
//            TESTS
// ----------------------------

#[cfg(test)]
mod tests {

    #[test]
    fn mascot_generates_correct_beaver_triples() {

    }
}
