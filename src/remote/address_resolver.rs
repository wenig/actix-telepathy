use log::*;
use actix::prelude::*;
use std::collections::HashMap;
use crate::remote::RemoteWrapper;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use serde::export::Formatter;


const NETWORKINTERFACE: &str = "networkinterface";
const GOSSIP: &str = "gossip";

#[derive(Serialize, Deserialize, Hash)]
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


#[derive(Message)]
#[rtype(result = "Result<AddressResponse, ()>")]
pub enum AddressRequest {
    Register(Recipient<RemoteWrapper>, String),
    ResolveStr(String),
    ResolveRec(Recipient<RemoteWrapper>)
}

pub enum AddressResponse {
    Register,
    ResolveStr(Recipient<RemoteWrapper>),
    ResolveRec(String)
}

pub struct AddressResolver {
    str2rec: HashMap<String, Recipient<RemoteWrapper>>,
    rec2str: HashMap<Recipient<RemoteWrapper>, String>
}

pub struct NotAvailableError {}

impl Debug for NotAvailableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NotAvailableError")
            .finish()
    }
}

impl AddressResolver {
    pub fn new() -> Self {
        AddressResolver {str2rec: HashMap::new(), rec2str: HashMap::new()}
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

impl Actor for AddressResolver {
    type Context = Context<Self>;
}

impl Handler<RemoteWrapper> for AddressResolver {
    type Result = ();

    fn handle(&mut self, msg: RemoteWrapper, _ctx: &mut Context<Self>) -> Self::Result {
        let recipient = self.resolve_rec_from_addr_representation(msg.destination.id.clone()).expect("Could not resolve Recipient for RemoteMessage");
        let _r = recipient.do_send(msg);
    }
}

impl Handler<AddressRequest> for AddressResolver {
    type Result = Result<AddressResponse, ()>;

    fn handle(&mut self, msg: AddressRequest, _ctx: &mut Context<Self>) -> Self::Result {
        match msg {
            AddressRequest::Register(rec, identifier) => {
                let is_new = match self.rec2str.get(&rec) {
                    Some(_) => false,
                    None => true,
                };

                if is_new {
                    self.str2rec.insert(identifier.clone(), rec.clone());
                    self.rec2str.insert(rec, identifier);
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
                    Ok(r) => Ok(AddressResponse::ResolveStr((*r).clone())),
                    Err(_) => Err(())
                }
            },
            AddressRequest::ResolveRec(rec) => {
                let id = self.resolve_rec(&rec);
                match id {
                    Ok(i) => Ok(AddressResponse::ResolveRec(i.clone())),
                    Err(_) => Err(())
                }
            }
        }
    }
}

impl Supervised for AddressResolver {}
