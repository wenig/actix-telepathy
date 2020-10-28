use log::*;
use actix::prelude::*;
use std::collections::HashMap;
use uuid::Uuid;
use crate::remote::RemoteMessage;
use std::str::FromStr;
use serde::{Deserialize, Serialize};


const NETWORKINTERFACE: &str = "networkinterface";
const GOSSIP: &str = "gossip";

#[derive(Serialize, Deserialize)]
pub enum AddrRepresentation {
    NetworkInterface,
    Gossip,
    Uuid(String)
}

impl ToString for AddrRepresentation {
    fn to_string(&self) -> String {
        match self {
            AddrRepresentation::NetworkInterface => String::from(NETWORKINTERFACE),
            AddrRepresentation::Gossip => String::from(GOSSIP),
            AddrRepresentation::Uuid(id) => id.clone(),
        }
    }
}

impl FromStr for AddrRepresentation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            NETWORKINTERFACE => AddrRepresentation::NetworkInterface,
            GOSSIP => AddrRepresentation::Gossip,
            _ => AddrRepresentation::Uuid(String::from(s))
        })
    }
}

impl Clone for AddrRepresentation {
    fn clone(&self) -> Self {
        AddrRepresentation::from_str(&self.to_string()).unwrap()
    }
}


#[derive(Message)]
#[rtype(result = "()")]
pub struct AddressRegistering {
    addr: Recipient<RemoteMessage>,
}

pub struct AddressResolver {
    str2rec: HashMap<String, Recipient<RemoteMessage>>,
    rec2str: HashMap<Recipient<RemoteMessage>, String>
}

impl AddressResolver {
    pub fn resolve_str(&mut self, id: String) -> Option<&Recipient<RemoteMessage>> {
        match self.str2rec.get(&id) {
            Some(rec) => Some(rec),
            None => {
                error!("ID {} is not registered", id);
                None
            },
        }
    }

    pub fn resolve_rec(&mut self, rec: &Recipient<RemoteMessage>) -> Option<&String> {
        match self.rec2str.get(rec) {
            Some(str) => Some(str),
            None => {
                error!("Recipient is not registered");
                None
            },
        }
    }
}

impl Actor for AddressResolver {
    type Context = Context<Self>;
}

impl Handler<AddressRegistering> for AddressResolver {
    type Result = ();

    fn handle(&mut self, msg: AddressRegistering, ctx: &mut Context<Self>) -> Self::Result {
        let is_new = match self.rec2str.get(&msg.addr) {
            Some(_) => false,
            None => true,
        };

        if is_new {
            let id = Uuid::new_v4().to_string();
            self.str2rec.insert(id.clone(), msg.addr.clone());
            self.rec2str.insert(msg.addr, id);
        } else {
            debug!("Recipient is already added");
        }
    }
}