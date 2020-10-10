use actix::prelude::*;

#[derive(Message)]
#[rtype(result = ())]
struct JoinCluster;


#[derive(Message)]
#[rtype(result = ())]
struct AcceptJoin;
