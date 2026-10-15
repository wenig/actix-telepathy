use log::*;
use proc_macro::{Span, TokenStream};
use quote::{ToTokens, quote};
use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use syn::{
    DeriveInput, Result, parse::Parser as _, parse_macro_input
};
type AttributeArgs = syn::punctuated::Punctuated<syn::Meta, syn::Token![,]>;


const TELEPATHY_CONFIG_FILE: &str = "telepathy.yaml";
const WITH_SOURCE: &str = "with_source";

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    pub custom_serializer: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            custom_serializer: "DefaultSerialization".to_string(),
        }
    }
}

fn load_config_yaml() -> Config {
    let f = File::open(TELEPATHY_CONFIG_FILE);
    match f {
        Ok(file_reader) => {
            serde_yaml::from_reader(file_reader).expect("Config file is no valid YAML")
        }
        Err(e) => {
            error!("{}, using default Config", e);
            Config::default()
        }
    }
}

pub fn remote_message_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = &input.generics.split_for_impl();
    let s = name.to_string();
    let sources = get_with_source_attr(&input).expect("Expected correct syntax");

    let set_source = match sources.first() {
        Some(source) => {
            let attr = source.clone().unwrap();
            quote! {
                self.#attr.node.network_interface = Some(addr);
            }
        }
        None => quote! {},
    };

    // https://docs.rs/actix_derive/0.6.2/src/actix_derive/message.rs.html
    let result = {
        match get_attribute_type_multiple(&input, "rtype") {
            Ok(ty) => match ty.len() {
                1 => ty[0].clone(),
                _ => {
                    return syn::Error::new(
                        Span::call_site().into(),
                        format!(
                            "#[{}(type)] takes 1 parameters, given {}",
                            "rtype",
                            ty.len()
                        ),
                    )
                    .to_compile_error().into()
                }
            },
            Err(err) => return err.to_compile_error().into(),
        }
    }.unwrap_or(syn::parse_quote!(()));

    let config: Config = load_config_yaml();
    let serializer = syn::parse_str::<syn::Type>(&config.custom_serializer).unwrap_or_else(|_| {
        panic!(
            "custom_serializer {} could not be found",
            &config.custom_serializer
        )
    });

    let expanded = quote! {
        use log::*;

        impl #impl_generics RemoteMessage for #name #ty_generics #where_clause {
            type Serializer = #serializer;
            const IDENTIFIER: &'static str = #s;

            fn get_serializer(&self) -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }

            fn generate_serializer() -> Box<Self::Serializer> {
                Box::new(#serializer {})
            }

            fn set_source(&mut self, addr: Addr<NetworkInterface>) {
                #set_source
            }
        }

        impl #impl_generics Message for #name #ty_generics #where_clause {
            type Result = #result;
        }
    };

    TokenStream::from(expanded)
}

fn get_with_source_attr(ast: &DeriveInput) -> Result<Vec<Option<syn::Type>>> {
    let attr = ast.attrs.iter().find_map(|attr| {
        if attr.path().is_ident(WITH_SOURCE) {
            attr.parse_args().ok()
        } else {
            None
        }
    });

    match attr {
        Some(a) => match a {
            syn::Meta::Path(path) => match path.get_ident() {
                Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                    .map(|ty| vec![Some(ty)])
                    .map_err(|_| syn::Error::new_spanned(ident, "Expect type")),
                None => Err(syn::Error::new_spanned(path, "Expect type")),
            },
            _ => Err(syn::Error::new_spanned(
                a,
                format!("The correct syntax is #[{}(<RemoteAddr>)]", WITH_SOURCE),
            )),
        },
        None => Ok(vec![]),
    }
}

fn get_attribute_type_multiple(
    ast: &syn::DeriveInput,
    name: &str,
) -> syn::Result<Vec<Option<syn::Type>>> {
    let mut target_attrs = ast.attrs.iter().filter(|attr| attr.path().is_ident(name));

    if let Some(Ok(typ)) = target_attrs
        .clone()
        .find_map(|attr| attr.parse_args::<syn::LitStr>().ok())
        .map(|lit_str| lit_str.parse::<syn::Type>())
    {
        return Ok(vec![Some(typ)]);
    }

    let attr = target_attrs
        .find_map(|attr| attr.parse_args().ok())
        .ok_or_else(|| {
            syn::Error::new(Span::call_site().into(), format!("Expected an attribute `{name}`"))
        })?;

    match attr {
        syn::Meta::List(ref list) => {
            let parser = AttributeArgs::parse_terminated;
            let args = match parser.parse2(list.tokens.clone()) {
                Ok(args) => args,
                Err(_) => {
                    return Err(syn::Error::new_spanned(
                        attr,
                        format!("The correct syntax is #[{name}(type, type, ...)]"),
                    ))
                }
            };

            Ok(args.iter().map(|m| meta_item_to_ty(m).ok()).collect())
        }

        syn::Meta::NameValue(ref nv) => match nv.path.get_ident() {
            Some(ident) if ident == "result" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = nv.value.clone()
                {
                    if let Ok(ty) = syn::parse_str::<syn::Type>(&lit.value()) {
                        return Ok(vec![Some(ty)]);
                    }
                }

                if let syn::Expr::Path(path) = nv.value.clone() {
                    if let Ok(ty) = syn::parse2::<syn::Type>(path.into_token_stream()) {
                        return Ok(vec![Some(ty)]);
                    }
                }

                Err(syn::Error::new_spanned(&nv.value, "expected type"))
            }
            _ => Err(syn::Error::new_spanned(
                &nv.value,
                r#"expected `result = TYPE`"#,
            )),
        },

        syn::Meta::Path(path) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map(|ty| vec![Some(ty)])
                .map_err(|_| syn::Error::new_spanned(ident, "expected type")),
            None => Err(syn::Error::new_spanned(path, "expected type")),
        },
    }
}

fn meta_item_to_ty(meta_item: &syn::Meta) -> syn::Result<syn::Type> {
    match meta_item {
        syn::Meta::Path(path) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map_err(|_| syn::Error::new_spanned(ident, "Expect type")),
            None => Err(syn::Error::new_spanned(path, "Expect type")),
        },
        syn::Meta::NameValue(nv) => match nv.path.get_ident() {
            Some(ident) if ident == "result" => {
                if let syn::Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(lit),
                    ..
                }) = nv.value.clone()
                {
                    if let Ok(ty) = syn::parse_str::<syn::Type>(&lit.value()) {
                        return Ok(ty);
                    }
                }
                Err(syn::Error::new_spanned(&nv.value, "Expect type"))
            }
            _ => Err(syn::Error::new_spanned(
                &nv.value,
                r#"Expect `result = "TYPE"`"#,
            )),
        },
        syn::Meta::List(list) => {
            let lit_str = syn::parse2::<syn::LitStr>(list.tokens.clone())
                .map_err(|_| syn::Error::new_spanned(list, "Expect type"))?;

            syn::parse_str(&lit_str.value())
                .map_err(|_| syn::Error::new_spanned(list, "Expect type"))
        }
    }
}