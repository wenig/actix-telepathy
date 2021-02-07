use crate::{AddrResolver, AddrRequest, RemoteWrapper};
use actix::{SystemService, Context, AsyncContext, Recipient};

pub trait RemoteActor {
    fn register(&mut self, rec: Recipient<RemoteWrapper>, identifier: String) {
        AddrResolver::from_registry().do_send(
            AddrRequest::Register(rec, identifier)
        );
    }
}
