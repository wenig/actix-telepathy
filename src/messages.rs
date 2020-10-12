use actix::Message;
use tokio::net::TcpStream;
use std::net::SocketAddr;

#[derive(Message)]
#[rtype(result = "()")]
pub struct TcpConnect(pub TcpStream, pub SocketAddr);

#[derive(Message)]
#[rtype(result = "()")]
struct JoinCluster;


#[derive(Message)]
#[rtype(result = "()")]
struct AcceptJoin;


#[derive(Message)]
#[rtype(result = "()")]
pub struct RemoteMessage;


#[derive(Message)]
#[rtype(result = "()")]
pub struct LocalMessage;