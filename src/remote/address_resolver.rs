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
#[rtype(result = "Result<AddressResponse, ()>")]
pub enum AddressRequest {
    Register(Recipient<RemoteMessage>),
    ResolveStr(String),
    ResolveRec(Recipient<RemoteMessage>)
}

pub enum AddressResponse {
    Register,
    ResolveStr(Recipient<RemoteMessage>),
    ResolveRec(String)
}

pub struct AddressResolver {
    str2rec: HashMap<String, Recipient<RemoteMessage>>,
    rec2str: HashMap<Recipient<RemoteMessage>, String>
}

impl AddressResolver {
    pub fn new() -> Self {
        AddressResolver {str2rec: HashMap::new(), rec2str: HashMap::new()}
    }

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

impl Handler<AddressRequest> for AddressResolver {
    type Result = Result<AddressResponse, ()>;

    fn handle(&mut self, msg: AddressRequest, ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            AddressRequest::Register(rec) => {
                let is_new = match self.rec2str.get(&rec) {
                    Some(_) => false,
                    None => true,
                };

                if is_new {
                    let id = Uuid::new_v4().to_string();
                    self.str2rec.insert(id.clone(), rec.clone());
                    self.rec2str.insert(rec, id);
                    debug!("Actor registered");
                    Ok(AddressResponse::Register)
                } else {
                    debug!("Recipient is already added");
                    Err(())
                }
            },
            AddressRequest::ResolveStr(id) => {
                let rec = self.resolve_str(id);
                match rec {
                    Some(r) => Ok(AddressResponse::ResolveStr((*r).clone())),
                    None => Err(())
                }
            },
            AddressRequest::ResolveRec(rec) => {
                let id = self.resolve_rec(&rec);
                match id {
                    Some(i) => Ok(AddressResponse::ResolveRec(i.clone())),
                    None => Err(())
                }
            }
        }
    }
}

impl Supervised for AddressResolver {}
