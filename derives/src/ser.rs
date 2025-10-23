use syn::DeriveInput;

use crate::container::{Container, EleType, FieldsSummary, Generic, StructField};

pub fn get_ser_impl_block(input: DeriveInput) -> proc_macro2::TokenStream {
    let container = Container::from_ast(&input);
    container.validate();
    if container.is_enum() {
        get_ser_enum_impl_block(container)
    } else {
        get_ser_struct_impl_block(container)
    }
}

fn get_ser_enum_impl_block(container: Container) -> proc_macro2::TokenStream {
    let ident = &container.original.ident;
    let (impl_generics, type_generics, where_clause) = container.original.generics.split_for_impl();
    let branches = container.enum_variants.iter().map(|v| {
        let f = v.ident;
        let ele_ty = &v.ele_type;
        if v.ty.is_none() {
            let name = v.name.as_ref().expect("should have name");
            quote!{
                Self::#f => {
                    if _tag_ == b"" {
                        let _t = String::from_utf8_lossy(#name);
                        let _ = _writer_.write_event(Event::Empty(BytesStart::new(_t)));
                    } else {
                        let _ = _writer_.write_event(Event::Start(BytesStart::new(String::from_utf8_lossy(_tag_))));
                        let _t = String::from_utf8_lossy(#name);
                        let _ = _writer_.write_event(Event::Empty(BytesStart::new(_t)));
                        let _ = _writer_.write_event(Event::End(BytesEnd::new(String::from_utf8_lossy(_tag_))));
                    }
                }
            }
        } else {
            if matches!(ele_ty, EleType::Text) {
                quote!{
                    Self::#f(c) => {
                        let _ = _writer_.write_event(Event::Text(BytesText::new(&c.serialize())));
                    }
                }
            } else {
                let name = v.name.as_ref().expect("should have hame");
                quote! {
                    Self::#f(c) => {
                        if _tag_ == b"" {
                            c.serialize(#name, _writer_);
                        } else {
                            let _ = _writer_.write_event(Event::Start(BytesStart::new(String::from_utf8_lossy(_tag_))));
                            c.serialize(#name, _writer_);
                            let _ = _writer_.write_event(Event::End(BytesEnd::new(String::from_utf8_lossy(_tag_))));
                        }
                    },
                }
            }
        }
    });
    quote! {
        #[allow(unused_must_use)]
        impl #impl_generics ::xmlserde::XmlSerialize for #ident #type_generics #where_clause {
            fn serialize<W: std::io::Write>(
                &self,
                _tag_: &[u8],
                _writer_: &mut ::xmlserde::quick_xml::Writer<W>,
            ) {
                use ::xmlserde::quick_xml::events::*;
                match self {
                    #(#branches)*
                }
            }
        }
    }
}

fn get_ser_struct_impl_block(container: Container) -> proc_macro2::TokenStream {
    let write_ns = match container.with_ns {
        Some(ns) => quote! {
            _attrs_.push(Attribute::from((b"xmlns".as_ref(), #ns.as_ref())));
        },
        None => quote! {},
    };
    let write_custom_ns = if container.custom_ns.len() == 0 {
        quote! {}
    } else {
        let cns = container.custom_ns.into_iter().map(|(ns, value)| {
            quote! {
                let mut __vec = b"xmlns:".to_vec();
                __vec.extend(#ns.to_vec());
                _attrs_.push(Attribute::from((__vec.as_ref(), #value.as_ref())));
            }
        });
        quote! {#(#cns)*}
    };
    let FieldsSummary {
        children,
        text,
        attrs,
        self_closed_children,
        untagged_enums: untags,
        untagged_structs: _,
    } = FieldsSummary::from_fields(container.struct_fields);
    if text.is_some() && (children.len() > 0 || self_closed_children.len() > 0 || untags.len() > 0)
    {
        panic!("Cannot have the text and children at the same time.")
    }
    let init = init_is_empty(&children, &self_closed_children, &untags, &text);
    let build_attr_and_push = attrs.into_iter().map(|attr| {
        let name = attr.name.as_ref().unwrap();
        let ident = attr.original.ident.as_ref().unwrap();
        match &attr.generic {
            Generic::Vec(_) => panic!("cannot use a vector in attribute"),
            Generic::Opt(_) => {
                quote! {
                    let mut sr: String;
                    match &self.#ident {
                        Some(v) => {
                            sr = v.serialize();
                            _attrs_.push(Attribute::from((#name.as_ref(), sr.as_bytes())));
                        },
                        None => {},
                    }
                }
            }
            Generic::None => match &attr.default {
                Some(path) => quote! {
                    let mut ser;
                    if #path() != self.#ident {
                        ser = self.#ident.serialize();
                        _attrs_.push(Attribute::from((#name.as_ref(), ser.as_bytes())));
                    }
                },
                None => quote! {
                    let ser = self.#ident.serialize();
                    _attrs_.push(Attribute::from((#name.as_ref(), ser.as_bytes())));
                },
            },
        }
    });
    let write_text_or_children = if let Some(t) = text {
        let ident = t.original.ident.as_ref().unwrap();
        if t.generic.is_opt() {
            quote! {
                match &self.#ident {
                    None => {},
                    Some(__d) => {
                        let r = __d.serialize();
                        let event = BytesText::new(&r);
                        _writer_.write_event(Event::Text(event));
                    }
                }
            }
        } else {
            quote! {
                let r = self.#ident.serialize();
                let event = BytesText::new(&r);
                _writer_.write_event(Event::Text(event));
            }
        }
    } else {
        let write_scf = self_closed_children.into_iter().map(|f| {
            let ident = f.original.ident.as_ref().unwrap();
            let name = f.name.as_ref().expect("should have name");
            quote! {
                if self.#ident {
                    let event = BytesStart::new(String::from_utf8_lossy(#name));
                    _writer_.write_event(Event::Empty(event));
                }
            }
        });
        let write_children = children.into_iter().map(|f| {
            if f.skip_serializing {
                quote! {}
            } else {
                let ident = f.original.ident.as_ref().unwrap();
                let name = f.name.as_ref().expect("should have name");
                quote! {
                    self.#ident.serialize(#name, _writer_);
                }
            }
        });
        let write_untags = untags.into_iter().map(|f| {
            let ident = f.original.ident.as_ref().expect("should have name");
            quote! {
                self.#ident.serialize(b"", _writer_);
            }
        });
        quote! {
            #(#write_scf)*
            #(#write_children)*
            #(#write_untags)*
        }
    };
    let ident = &container.original.ident;
    let (impl_generics, type_generics, where_clause) = container.original.generics.split_for_impl();
    let write_event = quote! {
        if is_empty {
            _writer_.write_event(Event::Empty(start));
        } else if _is_untagged_ {
            // Not to write the start event
            #write_text_or_children
        } else {
            _writer_.write_event(Event::Start(start));
            #write_text_or_children
            let end = BytesEnd::new(String::from_utf8_lossy(_tag_));
            _writer_.write_event(Event::End(end));
        }
    };
    let get_root = if let Some(r) = &container.root {
        quote! {
            fn ser_root() -> Option<&'static [u8]> {
                Some(#r)
            }
        }
    } else {
        quote! {}
    };
    quote! {
        #[allow(unused_must_use)]
        impl #impl_generics ::xmlserde::XmlSerialize for #ident #type_generics #where_clause {
            fn serialize<W: std::io::Write>(
                &self,
                _tag_: &[u8],
                _writer_: &mut ::xmlserde::quick_xml::Writer<W>,
            ) {
                use ::xmlserde::quick_xml::events::*;
                use ::xmlserde::quick_xml::events::attributes::Attribute;
                use ::xmlserde::XmlValue;
                let start = BytesStart::new(String::from_utf8_lossy(_tag_));
                let mut _attrs_ = Vec::<Attribute>::new();
                let _is_untagged_ = _tag_.len() == 0;
                #write_ns
                #write_custom_ns
                #(#build_attr_and_push)*
                let start = start.with_attributes(_attrs_);
                #init
                #write_event
            }
            #get_root
        }
    }
}

fn init_is_empty(
    children: &Vec<StructField>,
    scf: &Vec<StructField>,
    untags: &Vec<StructField>,
    text: &Option<StructField>,
) -> proc_macro2::TokenStream {
    let children_init = children.iter().map(|c| {
        let ident = c.original.ident.as_ref().unwrap();
        match &c.generic {
            Generic::Vec(_) => quote! {
                let #ident = self.#ident.len() > 0;
            },
            Generic::Opt(_) => quote! {
                let #ident = self.#ident.is_some();
            },
            Generic::None => match &c.default {
                Some(d) => quote! {
                    let #ident = self.#ident != #d();
                },
                None => quote! {let #ident = true;},
            },
        }
    });
    let has_untag_fields = untags.len() > 0;
    let scf_init = scf.iter().map(|s| {
        let ident = s.original.ident.as_ref().unwrap();
        quote! {
            let #ident = self.#ident;
        }
    });
    let text_init = match text {
        Some(tf) => {
            let ident = tf.original.ident.as_ref().unwrap();
            if tf.generic.is_opt() {
                quote! {
                    let mut has_text = true;
                    if self.#ident.is_none() {
                        has_text = false;
                    }
                }
            } else if tf.default.is_none() {
                quote! {let has_text = true;}
            } else {
                let path = tf.default.as_ref().unwrap();
                quote! {
                    let mut has_text = true;
                    if self.#ident == #path() {
                        has_text = false;
                    }
                }
            }
        }
        None => quote! {let has_text = false;},
    };
    let is_empty = {
        let idents = children.iter().chain(scf.iter()).map(|c| {
            let ident = c.original.ident.as_ref().unwrap();
            quote! {#ident}
        });
        quote! {
            let has_child_to_write = #(#idents ||)* has_text;
            let is_empty = !has_child_to_write && !#has_untag_fields;
        }
    };
    quote! {
        #(#children_init)*
        #(#scf_init)*
        #text_init
        #is_empty
    }
}
