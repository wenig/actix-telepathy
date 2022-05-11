use crate::{AddrRequest, AddrResolver, RemoteWrapper};
use actix::{Actor, Handler, Recipient, SystemService};

pub trait RemoteActor
where
    Self: Actor + Handler<RemoteWrapper>,
{
    const ACTOR_ID: &'static str;

    fn register(&mut self, rec: Recipient<RemoteWrapper>) {
        AddrResolver::from_registry()
            .do_send(AddrRequest::Register(rec, Self::ACTOR_ID.to_string()));
    }
}
