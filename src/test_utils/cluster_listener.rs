use actix::io::SinkWrite;
use actix::{Actor, Addr};
use actix_rt::System;
use futures_sink::Sink;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc::UnboundedSender;

/// When using the `TestClusterListener` make sure to implement the following Traits
///
/// ```rust
/// use actix::prelude::*;
/// use actix_broker::BrokerSubscribe;
/// use actix_telepathy::{ClusterLog};
///
/// impl<T> Actor for TestClusterListener<T> where T: Unpin + 'static + Clone {
///     type Context = Context<TestClusterListener<T>>;
///
///     fn started(&mut self, ctx: &mut Self::Context) {
///         self.subscribe_system_async::<ClusterLog>(ctx);
///     }
/// }
///
/// impl<T> Handler<ClusterLog> for TestClusterListener<T> where T: Unpin + 'static + Clone {
///     type Result = ();
///
///     fn handle(&mut self, msg: ClusterLog, ctx: &mut Self::Context) -> Self::Result {
///         match msg {
///             ClusterLog::NewMember(addr, remote_addr) => {
///                 // self.sink.unwrap().write(T).unwrap() <-- Do whatever you like here!
///             }
///             ClusterLog::MemberLeft(_addr) => {}
///         }
///     }
/// }
/// ```

pub(crate) struct TestClusterListener<T>
where
    T: Unpin + Clone + 'static,
    Self: Actor,
{
    #[allow(dead_code)]
    pub sink: Option<SinkWrite<T, TestSink<T>>>,
    pub content: Option<T>,
}

impl<T> TestClusterListener<T>
where
    T: Unpin + 'static + Clone,
    Self: Actor<Context = actix::Context<Self>>,
{
    pub fn new(sink: Option<SinkWrite<T, TestSink<T>>>, content: Option<T>) -> Self {
        Self { sink, content }
    }

    #[allow(dead_code)]
    pub fn new_with_sink(sink: SinkWrite<T, TestSink<T>>) -> Self {
        Self::new(Some(sink), None)
    }

    pub fn new_with_content(content: T) -> Self {
        Self::new(None, Some(content))
    }

    #[allow(dead_code)]
    pub fn create_from_sender(sender: UnboundedSender<T>) -> Addr<Self> {
        Self::create(move |ctx| {
            let sink = TestSink::new(sender);
            TestClusterListener::new_with_sink(SinkWrite::new(sink, ctx))
        })
    }
}

impl<T> actix::io::WriteHandler<()> for TestClusterListener<T>
where
    T: Unpin + 'static + Clone,
    Self: Actor,
{
    fn finished(&mut self, _ctx: &mut Self::Context) {
        System::current().stop();
    }
}

pub(crate) struct TestSink<T>
where
    T: Unpin,
{
    sender: UnboundedSender<T>,
    content: Option<T>,
}

impl<T> TestSink<T>
where
    T: Unpin,
{
    #[allow(dead_code)]
    pub fn new(sender: UnboundedSender<T>) -> Self {
        TestSink {
            sender,
            content: None,
        }
    }
}

impl<T> Sink<T> for TestSink<T>
where
    T: Unpin + Clone,
{
    type Error = ();

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let this = self.get_mut();
        match &this.content {
            Some(content) => {
                let _r = this.sender.send(content.clone());
                Poll::Ready(Ok(()))
            }
            None => Poll::Ready(Ok(())),
        }
    }

    fn start_send(self: Pin<&mut Self>, item: T) -> Result<(), Self::Error> {
        self.get_mut().content = Some(item);
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.poll_ready(cx)
    }
}
