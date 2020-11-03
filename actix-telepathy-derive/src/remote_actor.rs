use log::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, Result};
use syn::export::Span;

const REMOTE_MESSAGES: &str = "remote_messages";

pub fn remote_actor_macro(input: TokenStream) -> TokenStream {

    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let messages = get_message_types_attr(&input).expect("Expected at least on Message");

    let mut chained_if = quote! {};
    let mut first = true;

    for attr in messages.iter() {
        let name = attr.as_ref().unwrap();
        if first {
            chained_if = quote! {
                if #name::is_message(&(msg.message)) {
                    let deserialized_msg: #name = #name::from_packed(&(msg.message)).expect("Cannot deserialized #name message");
                    ctx.address().do_send(deserialized_msg);
                }
            };
            first = false;
        } else {
            chained_if = quote! {
                #chained_if
                else if #name::is_message(&(msg.message)) {
                    let deserialized_msg: #name = #name::from_packed(&(msg.message)).expect("Cannot deserialized #name message");
                    ctx.address().do_send(deserialized_msg);
                }
            };
        }
    }
    if !first {
        chained_if = quote! {
            #chained_if
            else {
                warn!("Message dropped because identifier of {} is unknown", &(msg.message));
            }
        }
    }

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        impl Handler<RemoteMessage> for #name {
            type Result = ();

            fn handle(&mut self, mut msg: RemoteMessage, ctx: &mut Context<Self>) -> Self::Result {
                #chained_if
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}


fn get_message_types_attr(ast: &DeriveInput) -> Result<Vec<Option<syn::Type>>> {
    let attr = ast
        .attrs
        .iter()
        .find_map(|a| {
            let a = a.parse_meta();
            match a {
                Ok(meta) => {
                    if meta.path().is_ident(REMOTE_MESSAGES) {
                        Some(meta)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), format!("Expect a attribute `{}`", REMOTE_MESSAGES))
        })?;

    if let syn::Meta::List(ref list) = attr {
        Ok(list
            .nested
            .iter()
            .map(|m| meta_item_to_struct(m).ok())
            .collect())
    } else {
        Err(syn::Error::new_spanned(
            attr,
            format!("The correct syntax is #[{}(Message, Message, ...)]", REMOTE_MESSAGES),
        ))
    }
}

fn meta_item_to_struct(meta_item: &syn::NestedMeta) -> syn::Result<syn::Type> {
    match meta_item {
        syn::NestedMeta::Meta(syn::Meta::Path(ref path)) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map_err(|_| syn::Error::new_spanned(ident, "Expect Message")),
            None => Err(syn::Error::new_spanned(path, "Expect Message")),
        },
        syn::NestedMeta::Meta(syn::Meta::NameValue(val)) =>
            Err(syn::Error::new_spanned(&val.lit, "Expect Message")),
        syn::NestedMeta::Lit(syn::Lit::Str(ref s)) =>
            Err(syn::Error::new_spanned(s, "Expect Message")),
        meta => Err(syn::Error::new_spanned(meta, "Expect type")),
    }
}