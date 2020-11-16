mod ml;
mod security;
mod cluster_listener;

use structopt::StructOpt;
use actix_rt;
use actix::prelude::*;
use actix_telepathy::*;
use security::{GroupingServer};
use crate::ml::{Training, Net, load_mnist};
use crate::cluster_listener::{OwnListener, ClusterAddr};
use tch::nn::VarStore;
use tch::{Device};
use serde::{Deserialize, Serialize};


#[derive(StructOpt, Debug, Clone)]
struct Parameters{
    local_addr: String,
    #[structopt(short, long)]
    server_addr: String,
    #[structopt(short, long)]
    cluster_size: usize,
    #[structopt(long, default_value = "0.01")]
    lr: f64,
    #[structopt(short, long, default_value = "8")]
    batch_size: usize,
    #[structopt(long, default_value = "1")]
    test_every: usize,
    #[structopt(long, default_value = "1")]
    update_every: usize,
    #[structopt(long, default_value = "2")]
    group_size: usize,
    #[structopt(long, default_value = "1")]
    history_length: usize,
    #[structopt(long)]
    seed_nodes: Vec<String>,
    #[structopt(long, default_value = "0.0")]
    dropout: f64,
    #[structopt(long)]
    adversarial: bool,
}

fn evtl_build_grouping_server(args: Parameters) -> Option<Addr<GroupingServer>> {
    if args.local_addr.eq(args.server_addr.as_str()) {
        Some(GroupingServer::new(
            args.group_size,
            args.history_length
        ).start())
    } else {
        None
    }
}

fn build_training(args: Parameters) -> Addr<Training> {
    let vs = VarStore::new(Device::Cpu);
    let dataset = load_mnist();
    let model = Net::new(&vs.root(), dataset.labels);

    Training::new(
        model,
        vs,
        dataset,
        args.lr,
        args.batch_size,
        args.test_every,
        args.update_every
    ).start()
}

fn build_cluster_listener(args: Parameters, training: Option<Addr<Training>>) -> Addr<OwnListener> {
    OwnListener::new(
        args.local_addr.clone(),
        "127.0.0.1:8000".to_string(),
        args.cluster_size,
        training
    ).start()
}

fn build_cluster(args: Parameters, cluster_listener: Addr<OwnListener>, group_server: Vec<(Recipient<RemoteWrapper>, &str)>) -> Addr<Cluster> {
    Cluster::new(
        args.local_addr,
        args.seed_nodes,
        vec![cluster_listener.recipient()],
        group_server
    )
}


#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args = Parameters::from_args();

    let grouping_server = evtl_build_grouping_server(args.clone());

    let training = match grouping_server {
        None => Some(build_training(args.clone())),
        Some(_) => None
    };

    let cluster_listener = build_cluster_listener(args.clone(), training);
    let cluster = build_cluster(
        args,
        cluster_listener.clone(),
        match grouping_server {
            Some(addr) => vec![(addr.recipient(), "GroupingServer")],
            None => vec![]
        }
    );

    cluster_listener.do_send(ClusterAddr::new(cluster));

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
