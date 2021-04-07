use actix::prelude::*;
use serde::{Serialize, Deserialize};
use crate::{RemoteAddr, CustomSerialization, NetworkInterface};
use uuid::Uuid;


/// Wrapper for messages to be sent to remote actor
#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoteWrapper {
    pub destination: RemoteAddr,
    pub message_buffer: Vec<u8>,
    pub identifier: String,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub source: Option<Addr<NetworkInterface>>,
    pub conversation_id: Option<Uuid>
}

impl RemoteWrapper {
    pub fn new<T: RemoteMessage + Serialize>(destination: RemoteAddr, message: T, conversation_id: Option<Uuid>) -> RemoteWrapper {
        let serializer = message.get_serializer();
        RemoteWrapper {
            destination,
            message_buffer: serializer.serialize(&message).expect("Cannot serialize message"),
            identifier: message.get_identifier().to_string(),
            source: None,
            conversation_id
        }
    }
}


impl Clone for RemoteWrapper {
    fn clone(&self) -> Self {
        RemoteWrapper {
            destination: self.destination.clone(),
            message_buffer: self.message_buffer.clone(),
            identifier: self.identifier.clone(),
            source: self.source.clone(),
            conversation_id: self.conversation_id.clone()
        }
    }
}


/// Helper Trait to prepare messages to be sent over the network
pub trait RemoteMessage {
    type Serializer: CustomSerialization;
    const IDENTIFIER: &'static str;

    fn get_identifier(&self) -> &str {
        Self::IDENTIFIER
    }

    fn get_serializer(&self) -> Box<Self::Serializer>;

    fn generate_serializer() -> Box<Self::Serializer>;

    fn set_source(&mut self, source: Addr<NetworkInterface>);
}
