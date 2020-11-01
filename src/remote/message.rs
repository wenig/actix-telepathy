use log::*;
use actix::prelude::*;
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
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
        RemoteMessage {destination, message: message.to_string_with_id()}
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
        let deserialized: RemoteMessage = serde_json::from_str(s).expect("Could not deserialize RemoteMessage!");
        Ok(deserialized)
    }
}

pub trait Sendable: ToString + FromStr {
    const IDENTIFIER: &'static str;

    fn is_message(serialized_msg: &String) -> bool {
        let v: Value = serde_json::from_str(serialized_msg).expect("String is no valid JSON");
        v["identifier"] == Self::IDENTIFIER
    }

    fn from_packed(serialized_msg: &String) -> Result<Self, Self::Err> {
        let mut v: Value = serde_json::from_str(serialized_msg).expect("String is no valid JSON");
        let raw = &v["raw"];
        Self::from_str(&raw.to_string())
    }

    fn to_string_with_id(&self) -> String {
        let self_value: Value = serde_json::from_str(&self.to_string()).unwrap();
        json!({
            "identifier": Self::IDENTIFIER,
            "raw": self_value
        }).to_string()
    }
}
