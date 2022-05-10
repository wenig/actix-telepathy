use crate::codec::ConnectCodec;
use crate::ClusterMessage;
use actix::io::{FramedWrite, WriteHandler};
use actix::prelude::*;
use std::io::Error;
use tokio::net::tcp::OwnedWriteHalf;

pub struct Writer {
    framed: Vec<FramedWrite<ClusterMessage, OwnedWriteHalf, ConnectCodec>>,
}

impl Writer {
    pub fn new(framed: FramedWrite<ClusterMessage, OwnedWriteHalf, ConnectCodec>) -> Self {
        Self {
            framed: vec![framed],
        }
    }

    fn transmit_message(&mut self, msg: ClusterMessage) {
        self.framed[0].write(msg);
    }
}

impl Actor for Writer {
    type Context = Context<Self>;
}

impl Handler<ClusterMessage> for Writer {
    type Result = ();

    fn handle(&mut self, msg: ClusterMessage, _ctx: &mut Self::Context) -> Self::Result {
        self.transmit_message(msg);
    }
}

impl WriteHandler<Error> for Writer {}
impl Supervised for Writer {}
