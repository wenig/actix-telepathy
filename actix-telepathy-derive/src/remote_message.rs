use log::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, Type};
use syn::export::Span;
use serde_derive::{Serialize, Deserialize};
use std::fs::File;

const TELEPATHY_CONFIG_FILE: &str = "telepathy.yaml";

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

    let config: Config = load_config_yaml();
    let serializer = syn::parse_str::<syn::Type>(&config.custom_serializer)
        .expect(&format!("custom_serializer {} could not be found", &config.custom_serializer));

    let expanded = quote! {
        impl Remotable for #name {
            type Serializer = #serializer;
            const IDENTIFIER: &'static str = #s;

            fn get_serializer(&self) -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }

            fn generate_serializer() -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }
        }
    };

    TokenStream::from(expanded)
}
