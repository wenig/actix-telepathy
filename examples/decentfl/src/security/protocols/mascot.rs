use actix::prelude::*;
use actix_telepathy::*;


#[derive(RemoteActor)]
#[remote_messages()]
pub struct Mascot {

}