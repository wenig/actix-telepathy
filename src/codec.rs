use actix::prelude::*;
use std::io;
use flexbuffers;
use futures::io::Error;
use bytes::{BytesMut, BufMut, Buf};
use byteorder::{NetworkEndian, ByteOrder};
use serde::{Serialize, Deserialize};
use crate::remote::RemoteWrapper;
use tokio_util::codec::{Decoder, Encoder};

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

impl ClusterMessage {
    pub fn split(&self) -> (Vec<u8>, Vec<u8>) {
        (match &self {
            Self::Message(wrapper) => wrapper.message_buffer.clone(),
            _ => panic!("split should not be used if not ClusterMessage::Message")
        }, flexbuffers::to_vec(&self).unwrap())
    }

    pub fn set_buffer(&mut self, bytes: Vec<u8>) {
        match self {
            Self::Message(ref mut wrapper) => { wrapper.message_buffer = bytes },
            _ => panic!("set_buffer should not be used if not ClusterMessage::Message")
        }
    }
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

        if src.len() >= size + (ENDIAN_LENGTH * 2) {
            src.advance(ENDIAN_LENGTH);
            let header_size = NetworkEndian::read_u32(src.as_ref()) as usize;
            src.advance(ENDIAN_LENGTH);

            if size > header_size {
                let header = src.split_to(header_size);
                let buf = src.split_to(size - header_size);
                let mut cluster_message = flexbuffers::from_slice::<ClusterMessage>(&header).unwrap();
                cluster_message.set_buffer(buf.to_vec());
                Ok(Some(cluster_message))
            } else {
                let buf = src.split_to(size);
                Ok(Some(flexbuffers::from_slice::<ClusterMessage>(&buf).unwrap()))
            }
        } else {
            Ok(None)
        }
    }
}

impl Encoder<ClusterMessage> for ConnectCodec {
    type Error = Error;

    fn encode(&mut self, item: ClusterMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        match &item {
            ClusterMessage::Request(_, _) => dst.extend_from_slice(PREFIX),
            ClusterMessage::Response => dst.extend_from_slice(PREFIX),
            ClusterMessage::Message(_) => {
                let (buffer, header) = item.split();
                let buffer_ref: &[u8] = buffer.as_ref();
                let header_ref: &[u8] = header.as_ref();

                dst.reserve(header_ref.len() + buffer_ref.len() + (ENDIAN_LENGTH * 2));
                dst.put_u32((header_ref.len() + buffer_ref.len()) as u32);
                dst.put_u32(header_ref.len() as u32);
                dst.put(header_ref);
                dst.put(buffer_ref);

                return Ok(());
            },
            _ => {}
        }

        let msg = flexbuffers::to_vec(&item).unwrap();

        let msg_ref: &[u8] = msg.as_ref();

        dst.reserve(msg_ref.len() + (ENDIAN_LENGTH * 2));
        dst.put_u32(msg_ref.len() as u32);
        dst.put_u32(msg_ref.len() as u32);
        dst.put(msg_ref);
        Ok(())
    }
}
