use log::*;
use actix::prelude::*;
use std::collections::HashMap;
use crate::remote::RemoteWrapper;
use std::str::FromStr;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use serde::export::Formatter;
use crate::remote::message::AskRemoteWrapper;


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
    rec2str: HashMap<Recipient<RemoteWrapper>, String>,
    str2ask: HashMap<String, Recipient<AskRemoteWrapper>>,
    ask2str: HashMap<Recipient<AskRemoteWrapper>, String>
}

impl Default for AddressResolver {
    fn default() -> Self {
        Self {
            str2rec: HashMap::new(),
            rec2str: HashMap::new(),
            str2ask: HashMap::new(),
            ask2str: HashMap::new()
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

impl AddressResolver {
    pub fn new() -> Self {
        AddressResolver::default()
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

    pub fn resolve_str_ask(&mut self, id: String) -> Result<&Recipient<AskRemoteWrapper>, NotAvailableError> {
        match self.str2ask.get(&id) {
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

    pub fn resolve_rec_ask(&mut self, rec: &Recipient<AskRemoteWrapper>) -> Result<&String, NotAvailableError> {
        match self.ask2str.get(rec) {
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

    fn started(&mut self, _ctx: &mut Context<Self>) {
        debug!("AddressResolver actor started");
    }
}

impl Handler<RemoteWrapper> for AddressResolver {
    type Result = ();

    fn handle(&mut self, msg: RemoteWrapper, _ctx: &mut Context<Self>) -> Self::Result {
        let recipient = self.resolve_rec_from_addr_representation(msg.destination.id.clone()).expect("Could not resolve Recipient for RemoteMessage");
        let _r = recipient.do_send(msg);
    }
}

impl Handler<AskRemoteWrapper> for AddressResolver {
    type Result = ResponseActFuture<Self, Result<RemoteWrapper, ()>>;

    fn handle(&mut self, msg: AskRemoteWrapper, _ctx: &mut Context<Self>) -> Self::Result {
        let destination_id = msg.remote_wrapper.destination.id.clone();
        let recipient = self.resolve_str_ask(destination_id.to_string()).expect("Could not resolve Recipient for RemoteMessage");
        let forwarded = recipient.send(msg);
        let forwarded = actix::fut::wrap_future::<_, Self>(forwarded);

        let update_self = forwarded.map(|res, _act, _ctx| {
            match res {
                Ok(v) => {
                    v
                },
                Err(_e) => Err(())
            }
        });

        Box::pin(update_self)
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
impl SystemService for AddressResolver {}
