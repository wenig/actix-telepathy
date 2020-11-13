use log::*;
use actix::prelude::*;
use actix_telepathy::*;
use crate::security::protocols::grouping::messages::{GroupingRequest, GroupingResponse};
use crate::security::protocols::grouping::FindGroup;


#[derive(RemoteActor)]
#[remote_messages(GroupingResponse)]
pub struct GroupingClient {
    parent: Recipient<FindGroup>,
    socket_addr: String,
    server: RemoteAddr,
    asked: bool
}


impl GroupingClient {
    pub fn new(parent: Recipient<FindGroup>, socket_addr: String, server: RemoteAddr) -> Self {
        Self {parent, socket_addr, server, asked: false}
    }

    fn receive_partners(&self, partners: Vec<RemoteAddr>) {
        let _r = self.parent.do_send(FindGroup::Response(partners));
    }

    fn initiate_grouping(&mut self) {
        self.asked = true;
        self.server.do_send(Box::new(GroupingRequest {
            source: RemoteAddr::new_from_id( self.socket_addr.clone(), "GroupingClient")
        }));
    }
}


impl Actor for GroupingClient {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("started")
    }
}


impl Handler<GroupingResponse> for GroupingClient {
    type Result = ();

    fn handle(&mut self, msg: GroupingResponse, _ctx: &mut Self::Context) -> Self::Result {
        self.receive_partners(msg.group)
    }
}


impl Handler<FindGroup> for GroupingClient {
    type Result = ();

    fn handle(&mut self, msg: FindGroup, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            FindGroup::Request => self.initiate_grouping(),
            _ => ()
        }
    }
}
