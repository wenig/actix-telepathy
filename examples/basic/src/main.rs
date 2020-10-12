use actix_rt;
use actix_telepathy::*;
use actix::System;
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Option<String>,
}


#[actix_rt::main]
async fn main() {
    let args = Parameters::from_args();
    let local_ip = args.local_ip.to_lowercase().trim().to_owned();
    let seed_nodes = args.seed_nodes.map(|n| n.to_lowercase().trim().to_owned());

    let cluster = Cluster::new(local_ip, seed_nodes);
}
