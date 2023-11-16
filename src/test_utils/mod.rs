use std::net::SocketAddr;

use port_scanner::request_open_port;

pub(crate) mod cluster_listener;


pub(crate) fn get_n_local_socket_addrs(n: usize) -> Vec<SocketAddr> {
    let mut addrs = Vec::new();
    for _ in 0..n {
        addrs.push(format!("127.0.0.1:{}", request_open_port().unwrap_or(8000)).parse().unwrap());
    }
    addrs
}