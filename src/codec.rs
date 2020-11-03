use actix::prelude::*;
use std::io;
use serde_json;
use tokio_util::codec::{Encoder, Decoder};
use futures::io::Error;
use bytes::{BytesMut, BufMut};
use byteorder::{NetworkEndian, ByteOrder};
use serde::{Serialize, Deserialize};
use crate::remote::RemoteMessage;

const PREFIX: &[u8] = b"ACTIX/1.0\r\n";

#[derive(Message, Deserialize, Serialize)]
#[rtype(result = "()")]
pub enum ClusterMessage {
    Request(String),
    Response,
    Message(RemoteMessage)
}

pub struct ConnectCodec {
    prefix: bool
}

impl ConnectCodec {
    pub fn new() -> ConnectCodec {
        ConnectCodec {prefix: false}
    }
}

impl Decoder for ConnectCodec {
    type Item = ClusterMessage;
    type Error = Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if !self.prefix {
            if src.len() < 11 {
                return Ok(None)
            }
            if &src[..11] == PREFIX {
                src.split_to(11);
                self.prefix = true;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Prefix mismatch"))
            }
        }

        let size = {
            if src.len() < 2 {
                return Ok(None)
            }
            NetworkEndian::read_u16(src.as_ref()) as usize
        };

        if src.len() >= size + 2 {
            src.split_to(2);
            let buf = src.split_to(size);
            Ok(Some(serde_json::from_slice::<ClusterMessage>(&buf)?))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ClusterMessage> for ConnectCodec {
    type Error = Error;

    fn encode(&mut self, item: ClusterMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            ClusterMessage::Request(_) => dst.extend_from_slice(PREFIX),
            ClusterMessage::Response => dst.extend_from_slice(PREFIX),
            _ => {}
        }
        // todo: add global config for serialization
        let msg = serde_json::to_string(&item).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + 2);
        dst.put_u16(msg_ref.len() as u16);
        dst.put(msg_ref);
        Ok(())
    }
}
