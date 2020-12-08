mod ml;
mod security;
mod cluster_listener;

pub use security::random_additive;
use structopt::StructOpt;
use actix_rt;
use actix::prelude::*;
use actix_telepathy::prelude::*;
use security::{GroupingServer};
use crate::ml::{Training, Net, load_mnist, ScoreStorage, Subset, ParameterServer, FlattenModel};
use crate::cluster_listener::{OwnListener, ClusterAddr};
use tch::nn::VarStore;
use tch::{Device, Tensor};
use std::net::{ToSocketAddrs, SocketAddr};
use log::*;


fn from_addr(s: &str) -> SocketAddr {
    s.to_socket_addrs().expect("Could not parse seed node").next().unwrap()
}


#[derive(StructOpt, Debug, Clone)]
struct Parameters{
    #[structopt(parse(from_str = from_addr))]
    local_addr: SocketAddr,
    #[structopt(short, long, parse(from_str = from_addr))]
    server_addr: SocketAddr,
    #[structopt(short, long)]
    cluster_size: usize,
    #[structopt(long)]
    db_path: String,
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
    #[structopt(long, parse(from_str = from_addr))]
    seed_nodes: Vec<SocketAddr>,
    #[structopt(long, default_value = "0.0")]
    dropout: f64,
    #[structopt(long)]
    adversarial: bool,
    #[structopt(long)]
    krum: bool,
    #[structopt(long, default_value = "1992")]
    seed: u64,
    #[structopt(long, default_value = "0")]
    split: usize,
    #[structopt(long)]
    centralized: bool,
    #[structopt(long, default_value = "0")]
    n_clients: usize
}

fn build_model(vs: &VarStore, out_channels: i64, seed: i64) -> Net {
    Net::new_with_seed(&vs.root(), out_channels, seed)
}

fn evtl_build_grouping_server(args: Parameters) -> Option<Addr<GroupingServer>> {
    if args.local_addr.eq(&args.server_addr) {
        Some(GroupingServer::new(
            args.group_size,
            args.history_length
        ).start())
    } else {
        None
    }
}

fn evtl_build_parameter_server(args: Parameters, model: Tensor) -> Option<Addr<ParameterServer>> {
    if args.local_addr.eq(&args.server_addr) {
        Some(ParameterServer::new(
            model,
            args.n_clients,
            args.local_addr
        ).start())
    } else {
        None
    }
}

fn build_score_storage(args: Parameters) -> ScoreStorage {
    let mut score_storage = ScoreStorage::new(&args.db_path);
    let _r = score_storage.new_experiment(
        args.cluster_size as i16,
        args.lr,
        args.batch_size as i16,
        args.test_every as i16,
        args.update_every as i16,
        args.group_size as i16,
        args.history_length as i16,
        args.dropout,
        args.adversarial,
        args.krum,
        args.seed as i64,
        args.centralized
    );
    score_storage
}

fn build_training(args: Parameters) -> Addr<Training> {
    SyncArbiter::start(1, move || {
        tch::set_num_threads(1);
        let score_storage = build_score_storage(args.clone());
        let vs = VarStore::new(Device::Cpu);
        let mut dataset = load_mnist();
        dataset.partition(args.split - 1, (args.cluster_size - 1) as i64, args.seed); // split - 1 because split=0 is the server that does not participate in training
        let model = build_model(&vs, dataset.labels, args.seed as i64);

        Training::new(
            model,
            vs,
            dataset,
            args.lr,
            args.batch_size,
            args.test_every,
            args.update_every,
            score_storage
        )
    })
}

fn build_cluster_listener(args: Parameters, training: Option<Addr<Training>>, model: Option<Tensor>) -> Addr<OwnListener> {
    OwnListener::new(
        args.local_addr,
        args.server_addr,
        args.cluster_size,
        training,
        args.centralized,
        model
    ).start()
}

fn build_cluster(args: Parameters, cluster_listener: Addr<OwnListener>, servers: Vec<(Recipient<RemoteWrapper>, &str)>) -> Addr<Cluster> {
    Cluster::new(
        args.local_addr,
        args.seed_nodes,
        vec![cluster_listener.recipient()],
        servers
    )
}


#[actix_rt::main]
async fn main() {
    env_logger::init();

    let args = Parameters::from_args();

    let model = if args.centralized {
        let vs = VarStore::new(Device::Cpu);
        let dataset = load_mnist();
        let model = build_model(&vs, dataset.labels, args.seed as i64).to_flat_tensor();
        Some(model)
    } else {
        None
    };

    let server: Vec<(Recipient<RemoteWrapper>, &str)> = if args.centralized {
        match evtl_build_parameter_server(args.clone(), model.as_ref().unwrap().copy()) {
            Some(addr) => vec![(addr.recipient(), "ParameterServer")],
            None => vec![]
        }
    } else {
        match evtl_build_grouping_server(args.clone()) {
            Some(addr) => vec![(addr.recipient(), "GroupingServer")],
            None => vec![]
        }
    };

    let training = if server.len() == 0 {
        Some(build_training(args.clone()))
    } else {
        None
    };

    debug!("Parameter Server: {:?}", server);
    debug!("centralized: {}", args.centralized);

    let cluster_listener = build_cluster_listener(args.clone(), training, model);
    let cluster = build_cluster(
        args,
        cluster_listener.clone(),
        server
    );

    cluster_listener.do_send(ClusterAddr::new(cluster));

    tokio::signal::ctrl_c().await.unwrap();
    println!("Ctrl-C received, shutting down");
    System::current().stop();
}
