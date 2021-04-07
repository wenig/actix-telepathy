use crate::{AddrResolver, AddrRequest, RemoteWrapper};
use actix::{SystemService, Recipient, Actor};

pub trait RemoteActor
where
    Self: Actor
{
    fn register(&mut self, rec: Recipient<RemoteWrapper>, identifier: String) {
        AddrResolver::from_registry().do_send(
            AddrRequest::Register(rec, identifier)
        );
    }
}
