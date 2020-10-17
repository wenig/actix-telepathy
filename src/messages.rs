use actix::Message;
use tokio::net::TcpStream;
use std::net::SocketAddr;
use crate::remote_addr::RemoteAddr;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);


#[derive(Message)]
#[rtype(result = "()")]
struct AcceptJoin;


#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoteMessage;


#[derive(Message)]
#[rtype(result = "()")]
pub struct LocalMessage;


#[derive(Message)]
#[rtype(result = "()")]
pub enum ClusterLog {
    NewMember(RemoteAddr),
}