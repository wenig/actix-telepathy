#![recursion_limit = "128"]
use proc_macro::TokenStream;

mod remote_actor;
mod serializable;

/// Helper to prepare actors for remote messages
/// # Example
/// ```
/// #[derive(Message)]
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
#[proc_macro_derive(RemoteActor, attributes(remote_messages))]
pub fn remote_actor_macro(input: TokenStream) -> TokenStream {
    remote_actor::remote_actor_macro(input)
}

#[proc_macro_derive(Serializable, attributes(serialize_with))]
pub fn serializable_macro(input: TokenStream) -> TokenStream {
    serializable::serializable_macro(input)
}