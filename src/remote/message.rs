use log::*;
use actix::prelude::*;
use serde::{Serialize, Deserialize, };
use std::str::FromStr;
use crate::RemoteAddr;


#[derive(Message, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct RemoteMessage {
    pub destination: RemoteAddr,
    pub message: String
}

impl RemoteMessage {
    pub fn new<T: Sendable>(destination: RemoteAddr, message: Box<T>) -> RemoteMessage {
        RemoteMessage {destination, message: message.to_string()}
    }
}

impl ToString for RemoteMessage {
    fn to_string(&self) -> String {
        serde_json::to_string(self).expect("Could not serialize RemoteMessage!")
    }
}

impl FromStr for RemoteMessage {
    type Err = ();

    fn from_str(s: &str) -> Result<RemoteMessage, Self::Err> {
        debug!("'{}'", s);
        let deserialized: RemoteMessage = serde_json::from_str(s).expect("Could not deserialize RemoteMessage!");
        Ok(deserialized)
    }
}

pub trait Sendable: ToString + FromStr {}
