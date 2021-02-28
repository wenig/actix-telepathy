#![recursion_limit = "128"]
use proc_macro::TokenStream;

mod remote_actor;
mod remote_message;

// todo remove log dependency
// todo rename remotable to remoteactor

/// Helper to prepare actors for remote messages
/// # Example
/// ```
/// #[derive(Message, RemoteMessage)]
/// struct MyMessage {}
/// impl Sendable for MyMessage {}
/// // ...
///
/// #[derive(RemoteActor)]
/// #[remote_messages(MyMessage)]
/// struct MyActor {}
/// // ...
/// ```
///
/// # Background
///
/// In the previous example, the MyActor struct gets extended the following way:
///
/// ```
/// impl Handler<RemoteMessage> for MyActor {
///     type Result = ();
///
///     fn handle(&mut self, mut msg: RemoteMessage, ctx: &mut actix::prelude::Context<Self>) -> Self::Result {
///         if MyMessage::is_message(&(msg.message)) {
///             let deserialized_msg: MyMessage = MyMessage::from_packed(&(msg.message))
///                 .expect("Cannot deserialized #name message");
///             ctx.address().do_send(deserialized_msg);
///         } else {
///             warn!("Message dropped because identifier of {} is unknown", &(msg.message));
///         }
///     }
/// }
/// ```
///
/// `remote_messages` can take multiple Message Types which get checked for their identifiers.
#[proc_macro_derive(RemoteActor, attributes(remote_messages, remote_ask_messages))]
pub fn remote_actor_macro(input: TokenStream) -> TokenStream {
    remote_actor::remote_actor_macro(input)
}


/// Helper to make messages sendable over network
/// # Example
/// ```
/// #[derive(Message, Serialize, Deserialize, RemoteMessage)]
/// struct MyMessage {}
///
/// // ...
///
/// #[derive(RemoteActor)]
/// #[remote_messages(MyMessage)]
/// struct MyActor {}
/// // ...
/// ```
///
/// # Background
///
/// In the previous example, the MyMessage struct gets extended the following way:
///
/// ```
/// impl RemoteMessage for MyMessage {
///     type Serializer = DefaultSerialization;
///     const IDENTIFIER: &'static str = "MyMessage";
///
///     fn get_serializer(&self) -> Box<Self::Serializer> {
///         Box::new(DefaultSerialization {})
///     }
///
///     fn generate_serializer() -> Box<Self::Serializer> {
///         Box::new(DefaultSerialization {})
///     }
///
///     fn set_source(&mut self, addr: Addr<NetworkInterface>) {
///
///     }
/// }
/// ```
///
/// # Options
///
/// If you add the derive attribute `with_source`, you have the possibility to set the source remote address on a predefined struct attribute.
/// That attribute needs to have the following type: `RemoteAddr`
///
/// ## Example
/// ```
/// #[derive(Message, Serialize, Deserialize, RemoteMessage)]
/// #[with_source(source)]
/// struct MyMessage {
///     source: RemoteAddr
/// }
/// ```
///
#[proc_macro_derive(RemoteMessage, attributes(with_source))]
pub fn remote_message_macro(input: TokenStream) -> TokenStream {
    remote_message::remote_message_macro(input)
}