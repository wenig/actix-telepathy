use proc_macro::TokenStream;
use quote::quote;
use syn::__private::Span;
use syn::parse::Parser;
use syn::{parse_macro_input, DeriveInput, Result};
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;

const REMOTE_MESSAGES: &str = "remote_messages";

pub fn remote_actor_macro(input: TokenStream) -> TokenStream {
    //let ask_remote = proc_macro2::TokenStream::from(remote_actor_remote_ask_messages_macro(input.clone()));
    remote_actor_remote_messages_macro(input)
}

pub fn remote_actor_remote_messages_macro(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();
    let messages =
        get_message_types_attr(&input, REMOTE_MESSAGES).expect("Expected at least one Message");

    let mut match_statement = quote! {};

    for attr in messages.iter() {
        let name = attr.as_ref().unwrap();
        let matching = quote! {
            #name::IDENTIFIER => {
                let mut deserialized_msg: #name = #name::generate_serializer().deserialize(&(msg.message_buffer)[..]).expect("Cannot deserialized #name message");
                if msg.source.clone().is_some() {
                    deserialized_msg.set_source(msg.source.unwrap());
                }
                ctx.address().do_send(deserialized_msg);
            },
        };
        match_statement = quote! {
            #match_statement
            #matching
        };
    }
    match_statement = quote! {
        match msg.identifier.as_str() {
            #match_statement
            _ => warn!("Message dropped because identifier {} is unknown", &(msg.identifier))
        }
    };

    let name_str = name.to_string();

    // Build the output, possibly using quasi-quotation
    let expanded = quote! {
        use log::*;

        impl #impl_generics RemoteActor for #name #ty_generics #where_clause {
            const ACTOR_ID: &'static str = #name_str;
        }

        impl #impl_generics Handler<RemoteWrapper> for #name #ty_generics #where_clause {
            type Result = ();

            fn handle(&mut self, mut msg: RemoteWrapper, ctx: &mut Self::Context) -> Self::Result {
                #match_statement
            }
        }
    };

    // Hand the output tokens back to the compiler
    TokenStream::from(expanded)
}

fn get_message_types_attr(ast: &DeriveInput, ident: &str) -> Result<Vec<Option<syn::Type>>> {
    let attr = ast
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident(ident) {
                Some(attr.meta.clone())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), format!("Expect an attribute `{ident}`"))
        })?;

    match attr {
        syn::Meta::List(ref list) => {
            let parser = AttributeArgs::parse_terminated;
            let args = match parser.parse2(list.tokens.clone()) {
                Ok(args) => args,
                Err(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!("The correct syntax is #[{}(Message, Message, ...)]", ident),
                    ))
                }
            };
            Ok(args.iter().map(|m| meta_item_to_struct(m).ok()).collect())
        }
        syn::Meta::Path(path) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map(|ty| vec![Some(ty)])
                .map_err(|_| syn::Error::new_spanned(ident, "Expect type")),
            None => Err(syn::Error::new_spanned(path, "Expect type")),
        },
        _ => Err(syn::Error::new_spanned(
            attr,
            format!("The correct syntax is #[{}(Message, Message, ...)]", ident),
        )),
    }
}

fn meta_item_to_struct(meta_item: &syn::Meta) -> syn::Result<syn::Type> {
    match meta_item {
        syn::Meta::Path(ref path) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map_err(|_| syn::Error::new_spanned(ident, "Expect Message")),
            None => Err(syn::Error::new_spanned(path, "Expect Message")),
        },
        syn::Meta::NameValue(val) => Err(syn::Error::new_spanned(val, "Expect Message")),
        meta => Err(syn::Error::new_spanned(meta, "Expect type")),
    }
}
