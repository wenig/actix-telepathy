use log::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Result};
use serde_derive::{Serialize, Deserialize};
use std::fs::File;

const TELEPATHY_CONFIG_FILE: &str = "telepathy.yaml";
const WITH_SOURCE: &str = "with_source";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    pub custom_serializer: String,
}

impl Default for Config {
    fn default() -> Self {
        Self { custom_serializer: "DefaultSerialization".to_string() }
    }
}

fn load_config_yaml() -> Config {
    let f = File::open(TELEPATHY_CONFIG_FILE);
    match f {
        Ok(file_reader) => {
            serde_yaml::from_reader(file_reader).expect("Config file is no valid YAML")
        },
        Err(e) => {
            error!("{}, using default Config", e.to_string());
            Config::default()
        }
    }
}

pub fn remote_message_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let s = name.to_string();
    let sources = get_with_source_attr(&input).expect("Expected correct syntax");

    let set_source = match sources.first() {
        Some(source) => {
            let attr = source.clone().unwrap();
            quote! {
                self.#attr.network_interface = Some(addr);
            }
        },
        None => quote! {}
    };


    let config: Config = load_config_yaml();
    let serializer = syn::parse_str::<syn::Type>(&config.custom_serializer)
        .expect(&format!("custom_serializer {} could not be found", &config.custom_serializer));

    let expanded = quote! {
        use log::*;

        impl RemoteMessage for #name {
            type Serializer = #serializer;
            const IDENTIFIER: &'static str = #s;

            fn get_serializer(&self) -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }

            fn generate_serializer() -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }

            fn set_source(&mut self, addr: RemoteAddr) {
                #set_source
            }
        }
    };

    TokenStream::from(expanded)
}


fn get_with_source_attr(ast: &DeriveInput) -> Result<Vec<Option<syn::Type>>> {
    let attr = ast
        .attrs
        .iter()
        .find_map(|a| {
            let a = a.parse_meta();
            match a {
                Ok(meta) => {
                    if meta.path().is_ident(WITH_SOURCE) {
                        Some(meta)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        });

    match attr {
        Some(a) => {
            if let syn::Meta::List(ref list) = a {
                Ok(list
                    .nested
                    .iter()
                    .map(|m| meta_item_to_struct(m).ok())
                    .collect())
            } else {
                Err(syn::Error::new_spanned(
                    a,
                    format!("The correct syntax is #[{}(Message, Message, ...)]", WITH_SOURCE),
                ))
            }
        },
        None => Ok(vec![])
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