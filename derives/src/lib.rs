#[macro_use]
extern crate quote;

mod container;
mod de;
mod enum_value;
mod ser;
mod symbol;
mod utils;

use de::get_de_impl_block;
use ser::get_ser_impl_block;

use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(XmlDeserialize, attributes(xmlserde))]
pub fn derive_xml_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    get_de_impl_block(input).into()
}

#[proc_macro_derive(XmlSerialize, attributes(xmlserde))]
pub fn derive_xml_serialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    get_ser_impl_block(input).into()
}

#[proc_macro_derive(XmlEnumValue, attributes(xmlserde))]
pub fn derive_xml_enum_value(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    enum_value::get_enum_value_impl_block(input).into()
}
