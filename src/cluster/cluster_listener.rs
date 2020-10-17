use actix::prelude::*;
use crate::ClusterLog;
use log::*;

pub struct ClusterListener {
    callback: Box<dyn Fn(ClusterLog)>
}

impl ClusterListener{
    pub fn new(callback: Box<dyn Fn(ClusterLog)>) -> ClusterListener {
        ClusterListener {callback}
    }
}

impl Actor for ClusterListener {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Context<Self>) {
        debug!("ClusterListener started!");
    }
}

impl Handler<ClusterLog> for ClusterListener {
    type Result = ();

    fn handle(&mut self, msg: ClusterLog, ctx: &mut Context<Self>) -> Self::Result {
        (self.callback)(msg);
    }
}

impl Supervised for ClusterListener {}