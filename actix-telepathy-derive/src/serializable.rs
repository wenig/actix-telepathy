use log::*;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Error, Result, Type};
use syn::export::Span;

const SERIALIZE_WITH: &str = "serialize_with";

pub fn serializable_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let serializer_result = get_message_types_attr(&input, SERIALIZE_WITH, false);

    let serializer = match serializer_result {
        Ok(attributes) => attributes.get(0).expect("Expected exactly one value").as_ref().unwrap().clone(),
        Err(_) => syn::parse_str::<syn::Type>("DefaultSerialization").unwrap()
    };

    let expanded = quote! {
        impl Sendable for #name {
            const IDENTIFIER: &'static str = "#name";
        }

        impl ToString for #name {
            fn to_string(&self) -> String {
                #serializer::serialize(self).expect("Could not serialize #name message!")
            }
        }

        impl FromStr for #name {
            type Err = ();

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let deserialized: #name = #serializer::deserialize(s).expect("Could not deserialize #name message!");
                Ok(deserialized)
            }
        }
    };

    TokenStream::from(expanded)
}

fn get_message_types_attr(ast: &DeriveInput, attr_name: &str, multiple: bool) -> Result<Vec<Option<syn::Type>>> {
    let attr = ast
        .attrs
        .iter()
        .find_map(|a| {
            let a = a.parse_meta();
            match a {
                Ok(meta) => {
                    if meta.path().is_ident(attr_name) {
                        Some(meta)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .ok_or_else(|| {
            syn::Error::new(Span::call_site(), format!("Expect an attribute `{}`", attr_name))
        })?;

    if let syn::Meta::List(ref list) = attr {
        Ok(list
            .nested
            .iter()
            .map(|m| meta_item_to_struct(m).ok())
            .collect())
    } else {
        let error_text = match multiple {
            true => format!("The correct syntax is #[{}(Value, Value, ...)]", attr_name),
            false => format!("The correct syntax is #[{}(Value)]", attr_name),
        };
        Err(syn::Error::new_spanned(
            attr,
            error_text,
        ))
    }
}


fn meta_item_to_struct(meta_item: &syn::NestedMeta) -> syn::Result<syn::Type> {
    match meta_item {
        syn::NestedMeta::Meta(syn::Meta::Path(ref path)) => match path.get_ident() {
            Some(ident) => syn::parse_str::<syn::Type>(&ident.to_string())
                .map_err(|_| syn::Error::new_spanned(ident, "Expect Value")),
            None => Err(syn::Error::new_spanned(path, "Expect Value")),
        },
        syn::NestedMeta::Meta(syn::Meta::NameValue(val)) =>
            Err(syn::Error::new_spanned(&val.lit, "Expect Value")),
        syn::NestedMeta::Lit(syn::Lit::Str(ref s)) =>
            Err(syn::Error::new_spanned(s, "Expect Value")),
        meta => Err(syn::Error::new_spanned(meta, "Expect type")),
    }
}