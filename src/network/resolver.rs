//! DNS resolver and connector utility actor taken and adapted from https://github.com/actix/actix/blob/v0.10.0/src/actors/resolver.rs

use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{self, Poll};
use std::time::Duration;

use derive_more::Display;
use log::warn;
use tokio::net::TcpStream;

use actix::prelude::*;
use trust_dns_resolver::{TokioAsyncResolver as AsyncResolver};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::lookup_ip::LookupIp;
use trust_dns_resolver::error::ResolveError;
use futures::prelude::future::Either;
use tokio::time::{sleep, Sleep};

#[derive(Eq, PartialEq, Debug)]
pub struct Resolve {
    pub name: String,
    pub port: Option<u16>,
}

impl Message for Resolve {
    type Result = Result<VecDeque<SocketAddr>, ResolverError>;
}

#[derive(Eq, PartialEq, Debug)]
pub struct Connect {
    pub name: String,
    pub port: Option<u16>,
    pub timeout: Duration,
}

impl Connect {
    pub fn host<T: AsRef<str>>(host: T) -> Connect {
        Connect {
            name: host.as_ref().to_owned(),
            port: None,
            timeout: Duration::from_secs(1),
        }
    }
}

impl Message for Connect {
    type Result = Result<TcpStream, ResolverError>;
}

#[derive(Eq, PartialEq, Debug)]
pub struct ConnectAddr(pub SocketAddr);

impl Message for ConnectAddr {
    type Result = Result<TcpStream, ResolverError>;
}

#[derive(Debug, Display)]
pub enum ResolverError {
    /// Failed to resolve the hostname
    #[display(fmt = "Failed resolving hostname: {}", _0)]
    Resolver(String),

    /// Address is invalid
    #[display(fmt = "Invalid input: {}", _0)]
    InvalidInput(&'static str),

    /// Connecting took too long
    #[display(fmt = "Timeout out while establishing connection")]
    Timeout,

    /// Connection io error
    #[display(fmt = "{}", _0)]
    IoError(io::Error),
}

pub struct Resolver {
    resolver: Option<AsyncResolver>,
    cfg: Option<(ResolverConfig, ResolverOpts)>,
    err: Option<String>,
}


impl Actor for Resolver {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let cfg = self.cfg.take();
        ctx.wait(
            async move { cfg }
                .into_actor(self)
                .then(
                    |cfg, this, _| -> Pin<Box<dyn ActorFuture<Self, Output = _>>> {
                        if let Some(cfg) = cfg {
                            return Box::pin( async {
                                    AsyncResolver::tokio(cfg.0, cfg.1)
                                }.into_actor(this)
                            );
                        }
                        Box::pin(
                            async {
                                match AsyncResolver::tokio_from_system_conf() {
                                    Ok(resolver) => Ok(resolver),
                                    Err(err) => {
                                        warn!(
                                            "Can not create system dns resolver: {}",
                                            err
                                        );
                                        AsyncResolver::tokio(
                                            ResolverConfig::default(),
                                            ResolverOpts::default(),
                                        )
                                    }
                                }
                            }
                            .into_actor(this),
                        )
                    },
                )
                .map(|resolver_res, this, _| {
                    // Keep the resolver itself.
                    this.resolver = Some(resolver_res.unwrap());
                }),
        );
    }
}

impl Supervised for Resolver {}

impl SystemService for Resolver {}

impl Default for Resolver {
    fn default() -> Resolver {
        Resolver {
            resolver: None,
            cfg: None,
            err: None,
        }
    }
}

impl Handler<Resolve> for Resolver {
    type Result = ResponseActFuture<Self, Result<VecDeque<SocketAddr>, ResolverError>>;

    fn handle(&mut self, msg: Resolve, _: &mut Self::Context) -> Self::Result {
        if let Some(ref err) = self.err {
            Box::pin(ResolveFut::err(err.clone()))
        } else {
            Box::pin(ResolveFut::new(
                msg.name,
                msg.port.unwrap_or(0),
                self.resolver.as_ref().unwrap(),
            ))
        }
    }
}

impl Handler<Connect> for Resolver {
    type Result = ResponseActFuture<Self, Result<TcpStream, ResolverError>>;

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) -> Self::Result {
        let timeout = msg.timeout;
        Box::pin(
            ResolveFut::new(
                msg.name,
                msg.port.unwrap_or(0),
                self.resolver.as_ref().unwrap(),
            )
            .then(move |addrs, act, _| match addrs {
                Ok(a) => Either::Left(TcpConnector::with_timeout(a, timeout)),
                Err(e) => Either::Right(async move { Err(e) }.into_actor(act)),
            }),
        )
    }
}

impl Handler<ConnectAddr> for Resolver {
    type Result = ResponseActFuture<Self, Result<TcpStream, ResolverError>>;

    fn handle(&mut self, msg: ConnectAddr, _: &mut Self::Context) -> Self::Result {
        let mut v = VecDeque::new();
        v.push_back(msg.0);
        Box::pin(TcpConnector::new(v))
    }
}

type LookupIpFuture = Pin<Box<dyn Future<Output = Result<LookupIp, ResolveError>>>>;

/// A resolver future.
struct ResolveFut {
    lookup: Option<LookupIpFuture>,
    port: u16,
    addrs: Option<VecDeque<SocketAddr>>,
    error: Option<ResolverError>,
    error2: Option<String>,
}

impl ResolveFut {
    pub fn new<S: AsRef<str>>(
        addr: S,
        port: u16,
        resolver: &AsyncResolver,
    ) -> ResolveFut {
        // try to parse as a regular SocketAddr first
        if let Ok(addr) = addr.as_ref().parse() {
            let mut addrs = VecDeque::new();
            addrs.push_back(addr);

            ResolveFut {
                port,
                lookup: None,
                addrs: Some(addrs),
                error: None,
                error2: None,
            }
        } else {
            // we need to do dns resolution
            match ResolveFut::parse(addr.as_ref(), port) {
                Ok((host, port)) => ResolveFut {
                    port,
                    lookup: {
                        // Clone data so it can be moved to async block
                        let resolver_clone = resolver.clone();
                        let host = host.to_string();
                        Some(Box::pin(async move {
                            let resolver = resolver_clone;
                            resolver.lookup_ip(host).await
                        }))
                    },
                    addrs: None,
                    error: None,
                    error2: None,
                },
                Err(err) => ResolveFut {
                    port,
                    lookup: None,
                    addrs: None,
                    error: Some(err),
                    error2: None,
                },
            }
        }
    }

    pub fn err(err: String) -> ResolveFut {
        ResolveFut {
            port: 0,
            lookup: None,
            addrs: None,
            error: None,
            error2: Some(err),
        }
    }

    fn parse(addr: &str, port: u16) -> Result<(&str, u16), ResolverError> {
        macro_rules! try_opt {
            ($e:expr, $msg:expr) => {
                match $e {
                    Some(r) => r,
                    None => return Err(ResolverError::InvalidInput($msg)),
                }
            };
        }

        // split the string by ':' and convert the second part to u16
        let mut parts_iter = addr.splitn(2, ':');
        let host = try_opt!(parts_iter.next(), "invalid socket address");
        let port_str = parts_iter.next().unwrap_or("");
        let port: u16 = port_str.parse().unwrap_or(port);

        Ok((host, port))
    }
}

impl<A: Actor> ActorFuture<A> for ResolveFut {
    type Output = Result<VecDeque<SocketAddr>, ResolverError>;
    fn poll(
        mut self: Pin<&mut Self>,
        _: &mut A,
        _: &mut A::Context,
        task: &mut task::Context<'_>,
    ) -> Poll<Self::Output> {
        if let Some(err) = self.error.take() {
            Poll::Ready(Err(err))
        } else if let Some(err) = self.error2.take() {
            Poll::Ready(Err(ResolverError::Resolver(err)))
        } else if let Some(addrs) = self.addrs.take() {
            Poll::Ready(Ok(addrs))
        } else {
            match Pin::new(self.lookup.as_mut().unwrap()).poll(task) {
                Poll::Pending => Poll::Pending,
                Poll::Ready(Ok(ips)) => {
                    let addrs: VecDeque<_> = ips
                        .iter()
                        .map(|ip| SocketAddr::new(ip, self.port))
                        .collect();
                    if addrs.is_empty() {
                        Poll::Ready(Err(ResolverError::Resolver(
                            "Expect at least one A dns record".to_owned(),
                        )))
                    } else {
                        Poll::Ready(Ok(addrs))
                    }
                }
                Poll::Ready(Err(err)) => {
                    Poll::Ready(Err(ResolverError::Resolver(format!("{}", err))))
                }
            }
        }
    }
}

struct HasSleep {
    sleep: Pin<Box<Sleep>>
}

impl HasSleep {
    pub fn new(duration: Duration) -> Self {
        Self {
            sleep: Pin::from(Box::new(sleep(duration)))
        }
    }
}

impl Future for HasSleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut task::Context<'_>) -> Poll<Self::Output> {
        self.get_mut().sleep.as_mut().poll(cx)
    }
}

/// A TCP stream connector.
#[allow(clippy::type_complexity)]
pub struct TcpConnector {
    addrs: VecDeque<SocketAddr>,
    timeout: HasSleep,
    stream: Option<Pin<Box<dyn Future<Output = Result<TcpStream, io::Error>>>>>,
}

impl TcpConnector {
    pub fn new(addrs: VecDeque<SocketAddr>) -> TcpConnector {
        TcpConnector::with_timeout(addrs, Duration::from_secs(1))
    }

    pub fn with_timeout(addrs: VecDeque<SocketAddr>, timeout: Duration) -> TcpConnector {
        TcpConnector {
            addrs,
            stream: None,
            timeout: HasSleep::new(timeout)
        }
    }
}

impl<A: Actor> ActorFuture<A> for TcpConnector {
    type Output = Result<TcpStream, ResolverError>;

    fn poll(
        self: Pin<&mut Self>,
        _: &mut A,
        _: &mut A::Context,
        cx: &mut task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.get_mut();

        // timeout
        if let Poll::Ready(_) = Pin::new(&mut this.timeout).poll(cx) {
            return Poll::Ready(Err(ResolverError::Timeout));
        }

        // connect
        loop {
            if let Some(ref mut fut) = this.stream {
                match Pin::new(fut).poll(cx) {
                    Poll::Ready(Ok(sock)) => return Poll::Ready(Ok(sock)),
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Err(err)) => {
                        if this.addrs.is_empty() {
                            return Poll::Ready(Err(ResolverError::IoError(err)));
                        }
                    }
                }
            }
            // try to connect
            let addr = this.addrs.pop_front().unwrap();
            this.stream = Some(Box::pin(TcpStream::connect(addr)));
        }
    }
}