use syn::DeriveInput;
use syn::Meta;

use crate::symbol::OTHER;
use crate::symbol::{MAP, RENAME};
use crate::utils::get_array_lit_str;
use crate::utils::get_lit_str;
use crate::utils::get_xmlserde_meta_items;

pub fn get_enum_value_impl_block(input: DeriveInput) -> proc_macro2::TokenStream {
    let data = match input.data {
        syn::Data::Enum(e) => e,
        _ => panic!("expect enum type"),
    };
    let ident = input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    let variants = data
        .variants
        .iter()
        .filter_map(|v| EnumValueVariant::from_variant(v))
        .collect::<Vec<_>>();

    let ser_branches = variants.iter().map(get_ser_branch).collect::<Vec<_>>();
    let de_branches = variants.iter().map(get_de_branch).collect::<Vec<_>>();
    quote! {
        impl #impl_generics ::xmlserde::XmlValue for #ident #type_generics #where_clause {
            fn serialize(&self) -> String {
                match &self {
                    #(#ser_branches),*
                }
            }
            fn deserialize(s: &str) -> Result<Self, String> {
                match s {
                    #(#de_branches),*
                }
            }
        }
    }
}

fn get_de_branch(var: &EnumValueVariant) -> proc_macro2::TokenStream {
    let ident = var.ident;
    if var.is_other {
        if let Some(field) = &var.is_other_field {
            let ty = &field.ty;
            return quote! {
                _ => Ok(Self::#ident(#ty::deserialize(s)?))
            };
        }
    }
    if !var.map.is_empty() {
        let values = var.map.iter().map(|s| quote! {#s => Ok(Self::#ident)});
        return quote! {
            #(#values),*
        };
    }
    if let Some(rename) = &var.rename {
        return quote! {
            #rename => Ok(Self::#ident)
        };
    }
    panic!("unexpected situation")
}

fn get_ser_branch(var: &EnumValueVariant) -> proc_macro2::TokenStream {
    let ident = var.ident;
    if var.is_other {
        if let Some(field) = &var.is_other_field {
            let ty = &field.ty;
            return quote! {
                Self::#ident(v) => {
                    <#ty as ::xmlserde::XmlValue>::serialize(v)
                }
            };
        }
    }
    if !var.map.is_empty() {
        let first = var.map.first().unwrap();
        return quote! {
            Self::#ident => #first.to_string()
        };
    }
    if let Some(rename) = &var.rename {
        return quote! {
            Self::#ident => #rename.to_string()
        };
    }

    panic!("unexpected situation")
}

struct EnumValueVariant<'a> {
    ident: &'a syn::Ident,
    is_other: bool,
    is_other_field: Option<syn::Field>,
    rename: Option<syn::LitStr>,
    map: Vec<syn::LitStr>,
}

impl<'a> EnumValueVariant<'a> {
    pub fn from_variant(v: &'a syn::Variant) -> Option<Self> {
        for meta_item in v
            .attrs
            .iter()
            .flat_map(|attr| get_xmlserde_meta_items(attr))
            .flatten()
        {
            match meta_item {
                Meta::Path(path) if path == OTHER => {
                    let field = match &v.fields {
                        syn::Fields::Named(_) => panic!("unsupported named fields"),
                        syn::Fields::Unnamed(fields_unnamed) => {
                            fields_unnamed.unnamed.iter().next().cloned()
                        }
                        syn::Fields::Unit => None,
                    };
                    if field.is_none() {
                        panic!("other field should not have no field!")
                    }
                    return Some(Self {
                        rename: None,
                        ident: &v.ident,
                        is_other: true,
                        is_other_field: field,
                        map: Vec::new(),
                    });
                }
                Meta::NameValue(m) if m.path == RENAME => {
                    if let Ok(s) = get_lit_str(&m.value) {
                        return Some(Self {
                            rename: Some(s.clone()),
                            ident: &v.ident,
                            is_other: false,
                            map: Vec::new(),
                            is_other_field: None,
                        });
                    }
                    panic!(r#"please use `#[rename = "..."]`"#);
                }
                Meta::NameValue(m) if m.path == MAP => {
                    if let Ok(s) = get_array_lit_str(&m.value) {
                        return Some(Self {
                            rename: None,
                            ident: &v.ident,
                            is_other: false,
                            map: s.into_iter().map(|s| s.clone()).collect(),
                            is_other_field: None,
                        });
                    }
                    panic!(r#"please use `#[map = ["..."]`"#);
                }
                _ => panic!("unexpected attribute"),
            }
        }
        None
    }
}
