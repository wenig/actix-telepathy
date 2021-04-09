use crate::{AddrResolver, AddrRequest, RemoteWrapper};
use actix::{SystemService, Recipient, Actor, Handler};

pub trait RemoteActor
where
    Self: Actor + Handler<RemoteWrapper>
{
    fn register(&mut self, rec: Recipient<RemoteWrapper>, identifier: String) {
        AddrResolver::from_registry().do_send(
            AddrRequest::Register(rec, identifier)
        );
    }
}
