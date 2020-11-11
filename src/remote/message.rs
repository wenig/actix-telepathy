use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::{RemoteAddr, CustomSerialization};
use crate::network::NetworkInterface;


/// Wrapper for messages to be sent to remote actor
#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoteWrapper {
    pub destination: RemoteAddr,
    pub message_buffer: Vec<u8>,
    pub identifier: String,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub source: Option<Addr<NetworkInterface>>
}

impl RemoteWrapper {
    pub fn new<T: Remotable + Serialize>(destination: RemoteAddr, message: Box<T>) -> RemoteWrapper {
        let serializer = message.get_serializer();
        RemoteWrapper {
            destination,
            message_buffer: serializer.serialize(message.as_ref()).expect("Cannot serialize message"),
            identifier: message.get_identifier().to_string(),
            source: None
        }
    }
}


/// Helper Trait to prepare messages to be sent over the network
pub trait Remotable {
    type Serializer: CustomSerialization;
    const IDENTIFIER: &'static str;

    fn get_identifier(&self) -> &str {
        Self::IDENTIFIER
    }

    fn get_serializer(&self) -> Box<Self::Serializer>;

    fn generate_serializer() -> Box<Self::Serializer>;

    fn set_source(&mut self, source: Addr<NetworkInterface>);
}
