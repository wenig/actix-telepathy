#[macro_use] extern crate log;

use actix_rt;
use actix_telepathy::*;
use actix::System;
use structopt::StructOpt;
use log::Level;


#[derive(StructOpt, Debug)]
struct Parameters {
    local_ip: String,
    seed_nodes: Option<String>,
}


fn main() {
    env_logger::init();

    let args = Parameters::from_args();
    let local_ip = args.local_ip.to_lowercase().trim().to_owned();
    let seed_nodes = args.seed_nodes.map(|n| n.to_lowercase().trim().to_owned());

    //let sys = actix::System::new("remote-example");

    actix::System::run(|| {
        Cluster::new(local_ip, seed_nodes);
    });

    //let _ = sys.run();
}
