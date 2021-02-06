use actix::prelude::*;
use std::io;
use flexbuffers;
use tokio_util::codec::{Encoder, Decoder};
use futures::io::Error;
use bytes::{BytesMut, BufMut};
use byteorder::{NetworkEndian, ByteOrder};
use serde::{Serialize, Deserialize};
use crate::remote::RemoteWrapper;

const PREFIX: &[u8] = b"ACTIX/1.0\r\n";
const ENDIAN_LENGTH: usize = 4;

#[derive(Message, Deserialize, Serialize)]
#[rtype(result = "()")]
pub enum ClusterMessage {
    Request(u16, bool), // bool = is_seed?
    Response,
    Message(RemoteWrapper),
    Decline
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
                let _s = src.split_to(11);
                self.prefix = true;
            } else {
                return Err(io::Error::new(io::ErrorKind::Other, "Prefix mismatch"))
            }
        }

        let size = {
            if src.len() < ENDIAN_LENGTH {
                return Ok(None)
            }
            NetworkEndian::read_u32(src.as_ref()) as usize
        };

        if src.len() >= size + ENDIAN_LENGTH {
            let _s = src.split_to(ENDIAN_LENGTH);
            let buf = src.split_to(size);
            Ok(Some(flexbuffers::from_slice::<ClusterMessage>(&buf).unwrap()))
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ClusterMessage> for ConnectCodec {
    type Error = Error;

    fn encode(&mut self, item: ClusterMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match item {
            ClusterMessage::Request(_, _) => dst.extend_from_slice(PREFIX),
            ClusterMessage::Response => dst.extend_from_slice(PREFIX),
            _ => {}
        }

        let msg = flexbuffers::to_vec(&item).unwrap();
        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + ENDIAN_LENGTH);
        dst.put_u32(msg_ref.len() as u32);
        dst.put(msg_ref);
        Ok(())
    }
}
