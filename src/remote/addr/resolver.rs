use log::*;
use actix::prelude::*;
use std::collections::HashMap;
use crate::remote::RemoteWrapper;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};


const NETWORKINTERFACE: &str = "networkinterface";
const GOSSIP: &str = "gossip";

#[derive(Serialize, Deserialize, Hash, Debug)]
pub enum AddrRepresentation {
    NetworkInterface,
    Gossip,
    Key(String)
}

impl ToString for AddrRepresentation {
    fn to_string(&self) -> String {
        match self {
            AddrRepresentation::NetworkInterface => String::from(NETWORKINTERFACE),
            AddrRepresentation::Gossip => String::from(GOSSIP),
            AddrRepresentation::Key(id) => id.clone(),
        }
    }
}

impl FromStr for AddrRepresentation {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            NETWORKINTERFACE => AddrRepresentation::NetworkInterface,
            GOSSIP => AddrRepresentation::Gossip,
            _ => AddrRepresentation::Key(String::from(s))
        })
    }
}

impl Clone for AddrRepresentation {
    fn clone(&self) -> Self {
        AddrRepresentation::from_str(&self.to_string()).unwrap()
    }
}

impl PartialEq for AddrRepresentation {
    fn eq(&self, other: &Self) -> bool {
        let self_key = self.to_string();
        (self_key == other.to_string() && self_key != "Key")
        || (self_key == "Key" && match self {
            AddrRepresentation::Key(key) => match other {
                AddrRepresentation::Key(other_key) => key == other_key,
                _ => false
            } ,
            _ => false
        })
    }
}

impl Eq for AddrRepresentation {}


#[derive(Message)]
#[rtype(result = "Result<AddrResponse, ()>")]
pub enum AddrRequest {
    Register(Recipient<RemoteWrapper>, String),
    ResolveStr(String),
    ResolveRec(Recipient<RemoteWrapper>)
}

pub enum AddrResponse {
    Register,
    ResolveStr(Recipient<RemoteWrapper>),
    ResolveRec(String)
}

pub struct AddrResolver {
    str2rec: HashMap<String, Recipient<RemoteWrapper>>,
    rec2str: HashMap<Recipient<RemoteWrapper>, String>,
}

impl Default for AddrResolver {
    fn default() -> Self {
        Self {
            str2rec: HashMap::new(),
            rec2str: HashMap::new(),
        }
    }
}

pub struct NotAvailableError {}

impl Debug for NotAvailableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotAvailableError")
            .finish()
    }
}

impl AddrResolver {
    pub fn new() -> Self {
        AddrResolver::default()
    }

    pub fn resolve_str(&mut self, id: String) -> Result<&Recipient<RemoteWrapper>, NotAvailableError> {
        match self.str2rec.get(&id) {
            Some(rec) => Ok(rec),
            None => {
                error!("ID {} is not registered", id);
                Err(NotAvailableError {})
            },
        }
    }

    pub fn resolve_rec(&mut self, rec: &Recipient<RemoteWrapper>) -> Result<&String, NotAvailableError> {
        match self.rec2str.get(rec) {
            Some(str) => Ok(str),
            None => {
                error!("Recipient is not registered");
                Err(NotAvailableError {})
            },
        }
    }

    pub fn resolve_rec_from_addr_representation(&mut self, addr_representation: AddrRepresentation) -> Result<&Recipient<RemoteWrapper>, NotAvailableError> {
        self.resolve_str(addr_representation.to_string())
    }
}

impl Actor for AddrResolver {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("AddressResolver actor started");
    }
}

impl Handler<RemoteWrapper> for AddrResolver {
    type Result = ();

    fn handle(&mut self, msg: RemoteWrapper, _ctx: &mut Context<Self>) -> Self::Result {
        let recipient = self.resolve_rec_from_addr_representation(msg.destination.id.clone()).expect("Could not resolve Recipient for RemoteMessage");
        let _r = recipient.do_send(msg);
    }
}

impl Handler<AddrRequest> for AddrResolver {
    type Result = Result<AddrResponse, ()>;

    fn handle(&mut self, msg: AddrRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            AddrRequest::Register(rec, identifier) => {
                let is_new = match self.rec2str.get(&rec) {
                    Some(_) => false,
                    None => true,
                };

                if is_new {
                    self.str2rec.insert(identifier.clone(), rec.clone());
                    self.rec2str.insert(rec, identifier.clone());
                    debug!("Actor '{}' registered", identifier);
                    Ok(AddrResponse::Register)
                } else {
                    debug!("Recipient is already added");
                    Err(())
                }
            },
            AddrRequest::ResolveStr(id) => {
                let rec = self.resolve_str(id);
                match rec {
                    Ok(r) => Ok(AddrResponse::ResolveStr((*r).clone())),
                    Err(_) => Err(())
                }
            },
            AddrRequest::ResolveRec(rec) => {
                let id = self.resolve_rec(&rec);
                match id {
                    Ok(i) => Ok(AddrResponse::ResolveRec(i.clone())),
                    Err(_) => Err(())
                }
            }
        }
    }
}

impl Supervised for AddrResolver {}
impl SystemService for AddrResolver {}
