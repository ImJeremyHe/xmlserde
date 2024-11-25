use crate::symbol::{
    DEFAULT, DENY_UNKNOWN, NAME, ROOT, SKIP_SERIALIZING, TYPE, VEC_SIZE, WITH_CUSTOM_NS, WITH_NS,
    XML_SERDE,
};
use proc_macro2::{Group, Span, TokenStream, TokenTree};
use syn::parse::{self, Parse};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Meta::Path;
use syn::Meta::{self, NameValue};
use syn::Variant;

pub struct Container<'a> {
    pub struct_fields: Vec<StructField<'a>>, // Struct fields
    pub enum_variants: Vec<EnumVariant<'a>>,
    pub original: &'a syn::DeriveInput,
    pub with_ns: Option<syn::LitByteStr>,
    pub custom_ns: Vec<(syn::LitByteStr, syn::LitByteStr)>,
    pub root: Option<syn::LitByteStr>,
    pub deny_unknown: bool,
}

impl<'a> Container<'a> {
    pub fn is_enum(&self) -> bool {
        self.enum_variants.len() > 0
    }

    pub fn validate(&self) {
        if self.root.is_some() && self.is_enum() {
            panic!("for clarity, enum should not have the root attribute. please use a struct to wrap the enum and set its type to untag")
        }
        if self.deny_unknown && self.is_enum() {
            panic!("`deny_unknown_fields` is not supported in enum type")
        }

        self.struct_fields.iter().for_each(|f| f.validate());
    }

    pub fn from_ast(item: &'a syn::DeriveInput, _derive: Derive) -> Container<'a> {
        let mut with_ns = Option::<syn::LitByteStr>::None;
        let mut custom_ns = Vec::<(syn::LitByteStr, syn::LitByteStr)>::new();
        let mut root = Option::<syn::LitByteStr>::None;
        let mut deny_unknown = false;
        for meta_item in item
            .attrs
            .iter()
            .flat_map(|attr| get_xmlserde_meta_items(attr))
            .flatten()
        {
            match meta_item {
                NameValue(m) if m.path == WITH_NS => {
                    if let Ok(s) = get_lit_byte_str(&m.value) {
                        with_ns = Some(s.clone());
                    }
                }
                NameValue(m) if m.path == ROOT => {
                    let s = get_lit_byte_str(&m.value).expect("parse root failed");
                    root = Some(s.clone());
                }
                Meta::Path(p) if p == DENY_UNKNOWN => {
                    deny_unknown = true;
                }
                Meta::List(l) if l.path == WITH_CUSTOM_NS => {
                    let strs = l
                        .parse_args_with(Punctuated::<syn::LitByteStr, Comma>::parse_terminated)
                        .unwrap();
                    let mut iter = strs.iter();
                    let first = iter.next().expect("with_custom_ns should have 2 arguments");
                    let second = iter.next().expect("with_custom_ns should have 2 arguments");
                    if iter.next().is_some() {
                        panic!("with_custom_ns should have 2 arguments")
                    }
                    custom_ns.push((first.clone(), second.clone()));
                }
                _ => panic!("unexpected"),
            }
        }
        match &item.data {
            syn::Data::Struct(ds) => {
                let fields = ds
                    .fields
                    .iter()
                    .map(|f| StructField::from_ast(f))
                    .filter(|f| f.is_some())
                    .map(|f| f.unwrap())
                    .collect::<Vec<_>>();
                Container {
                    struct_fields: fields,
                    enum_variants: vec![],
                    original: item,
                    with_ns,
                    custom_ns,
                    root,
                    deny_unknown,
                }
            }
            syn::Data::Enum(e) => {
                let variants = e
                    .variants
                    .iter()
                    .map(|v| EnumVariant::from_ast(v))
                    .collect::<Vec<_>>();
                Container {
                    struct_fields: vec![],
                    enum_variants: variants,
                    original: item,
                    with_ns,
                    custom_ns,
                    root,
                    deny_unknown,
                }
            }
            syn::Data::Union(_) => panic!("Only support struct and enum type, union is found"),
        }
    }
}

pub struct FieldsSummary<'a> {
    pub children: Vec<StructField<'a>>,
    pub text: Option<StructField<'a>>,
    pub attrs: Vec<StructField<'a>>,
    pub self_closed_children: Vec<StructField<'a>>,
    pub untagged_enums: Vec<StructField<'a>>,
    pub untagged_structs: Vec<StructField<'a>>,
}

impl<'a> FieldsSummary<'a> {
    pub fn from_fields(fields: Vec<StructField<'a>>) -> Self {
        let mut result = FieldsSummary {
            children: vec![],
            text: None,
            attrs: vec![],
            self_closed_children: vec![],
            untagged_enums: vec![],
            untagged_structs: vec![],
        };
        fields.into_iter().for_each(|f| match f.ty {
            EleType::Attr => result.attrs.push(f),
            EleType::Child => result.children.push(f),
            EleType::Text => result.text = Some(f),
            EleType::SelfClosedChild => result.self_closed_children.push(f),
            EleType::Untag => result.untagged_enums.push(f),
            EleType::UntaggedEnum => result.untagged_enums.push(f),
            EleType::UntaggedStruct => result.untagged_structs.push(f),
        });
        result
    }
}

pub struct StructField<'a> {
    pub ty: EleType,
    pub name: Option<syn::LitByteStr>,
    pub skip_serializing: bool,
    pub default: Option<syn::ExprPath>,
    pub original: &'a syn::Field,
    pub vec_size: Option<syn::Lit>,
    pub generic: Generic<'a>,
}

impl<'a> StructField<'a> {
    pub fn validate(&self) {
        let untagged = match self.ty {
            EleType::Untag => true,
            EleType::UntaggedEnum => true,
            EleType::UntaggedStruct => true,
            _ => false,
        };
        if untagged && self.name.is_some() {
            panic!("untagged types doesn't need a name")
        }
    }

    pub fn from_ast(f: &'a syn::Field) -> Option<Self> {
        let mut name = Option::<syn::LitByteStr>::None;
        let mut skip_serializing = false;
        let mut default = Option::<syn::ExprPath>::None;
        let mut ty = Option::<EleType>::None;
        let mut vec_size = Option::<syn::Lit>::None;
        let generic = get_generics(&f.ty);
        for meta_item in f
            .attrs
            .iter()
            .flat_map(|attr| get_xmlserde_meta_items(attr))
            .flatten()
        {
            match meta_item {
                NameValue(m) if m.path == NAME => {
                    if let Ok(s) = get_lit_byte_str(&m.value) {
                        name = Some(s.clone());
                    }
                }
                NameValue(m) if m.path == TYPE => {
                    if let Ok(s) = get_lit_str(&m.value) {
                        let t = match s.value().as_str() {
                            "attr" => EleType::Attr,
                            "child" => EleType::Child,
                            "text" => EleType::Text,
                            "sfc" => EleType::SelfClosedChild,
                            "untag" => EleType::Untag, // todo: generate a deprecate function to let users know
                            "untagged_enum" => EleType::UntaggedEnum,
                            "untagged_struct" => EleType::UntaggedStruct,
                            _ => panic!("invalid type"),
                        };
                        ty = Some(t);
                    }
                }
                NameValue(m) if m.path == VEC_SIZE => {
                    if let syn::Expr::Lit(lit) = m.value {
                        match lit.lit {
                            syn::Lit::Str(_) | syn::Lit::Int(_) => {
                                vec_size = Some(lit.lit);
                            }
                            _ => panic!(),
                        }
                    } else {
                        panic!()
                    }
                }
                Path(word) if word == SKIP_SERIALIZING => {
                    skip_serializing = true;
                }
                NameValue(m) if m.path == DEFAULT => {
                    let path = parse_lit_into_expr_path(&m.value)
                        .expect("parse default path")
                        .clone();
                    default = Some(path);
                }
                _ => panic!("unexpected"),
            }
        }
        if ty.is_none() {
            None
        } else {
            Some(StructField {
                ty: ty.expect("should has a ty"),
                name,
                skip_serializing,
                default,
                original: f,
                vec_size,
                generic,
            })
        }
    }

    pub fn is_required(&self) -> bool {
        if matches!(self.ty, EleType::Untag) || matches!(self.ty, EleType::UntaggedEnum) {
            return match self.generic {
                Generic::Vec(_) => false,
                Generic::Opt(_) => false,
                Generic::None => true,
            };
        }
        self.default.is_none()
            && matches!(self.generic, Generic::None)
            && !matches!(self.ty, EleType::SelfClosedChild)
    }
}

pub struct EnumVariant<'a> {
    pub name: Option<syn::LitByteStr>,
    pub ident: &'a syn::Ident,
    pub ty: Option<&'a syn::Type>,
    pub ele_type: EleType,
}

impl<'a> EnumVariant<'a> {
    pub fn from_ast(v: &'a Variant) -> Self {
        let mut name = Option::<syn::LitByteStr>::None;
        let mut ele_type = EleType::Child;
        for meta_item in v
            .attrs
            .iter()
            .flat_map(|attr| get_xmlserde_meta_items(attr))
            .flatten()
        {
            match meta_item {
                NameValue(m) if m.path == NAME => {
                    if let Ok(s) = get_lit_byte_str(&m.value) {
                        name = Some(s.clone());
                    }
                }
                NameValue(m) if m.path == TYPE => {
                    if let Ok(s) = get_lit_str(&m.value) {
                        let t = match s.value().as_str() {
                            "child" => EleType::Child,
                            "text" => EleType::Text,
                            _ => panic!("invalid type in enum, should be `text` or `child` only"),
                        };
                        ele_type = t;
                    }
                }
                _ => panic!("unexpected attribute"),
            }
        }
        if v.fields.len() > 1 {
            panic!("only support 1 field");
        }
        if matches!(ele_type, EleType::Text) {
            if name.is_some() {
                panic!("should omit the `name`");
            }
        } else if name.is_none() {
            panic!("should have name")
        }
        let field = &v.fields.iter().next();
        let ty = field.map(|t| &t.ty);
        let ident = &v.ident;
        EnumVariant {
            name,
            ty,
            ident,
            ele_type,
        }
    }
}

/// Specify where this field is in the xml.
pub enum EleType {
    Attr,
    Child,
    Text,
    ///
    /// ```
    /// struct Font {
    ///     bold: bool,
    ///     italic: bool,
    /// }
    /// ```
    /// In the xml, it is like
    /// <font>
    ///     <b/>
    ///     <i/>
    /// </font>
    /// In this case, </b> indicates the field *bold* is true and <i/> indicates *italic* is true.
    SelfClosedChild,
    /// Deprecated, use `UntaggedEnum`
    Untag,

    UntaggedEnum,
    UntaggedStruct,
}

pub enum Derive {
    Serialize,
    Deserialize,
}

fn get_xmlserde_meta_items(attr: &syn::Attribute) -> Result<Vec<syn::Meta>, ()> {
    if attr.path() != XML_SERDE {
        return Ok(Vec::new());
    }

    match attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
        Ok(meta) => Ok(meta.into_iter().collect()),
        Err(_) => Err(()),
    }
}

fn get_lit_byte_str<'a>(expr: &syn::Expr) -> Result<&syn::LitByteStr, ()> {
    if let syn::Expr::Lit(lit) = expr {
        if let syn::Lit::ByteStr(l) = &lit.lit {
            return Ok(l);
        }
    }
    Err(())
}

fn get_lit_str<'a>(lit: &syn::Expr) -> Result<&syn::LitStr, ()> {
    if let syn::Expr::Lit(lit) = lit {
        if let syn::Lit::Str(l) = &lit.lit {
            return Ok(&l);
        }
    }
    Err(())
}

pub fn parse_lit_into_expr_path(value: &syn::Expr) -> Result<syn::ExprPath, ()> {
    let l = get_lit_str(value)?;
    parse_lit_str(l).map_err(|_| ())
}

pub fn parse_lit_str<T>(s: &syn::LitStr) -> parse::Result<T>
where
    T: Parse,
{
    let tokens = spanned_tokens(s)?;
    syn::parse2(tokens)
}

fn spanned_tokens(s: &syn::LitStr) -> parse::Result<TokenStream> {
    let stream = syn::parse_str(&s.value())?;
    Ok(respan(stream, s.span()))
}

fn respan(stream: TokenStream, span: Span) -> TokenStream {
    stream
        .into_iter()
        .map(|token| respan_token(token, span))
        .collect()
}

fn respan_token(mut token: TokenTree, span: Span) -> TokenTree {
    if let TokenTree::Group(g) = &mut token {
        *g = Group::new(g.delimiter(), respan(g.stream(), span));
    }
    token.set_span(span);
    token
}

fn get_generics(t: &syn::Type) -> Generic {
    match t {
        syn::Type::Path(p) => {
            let path = &p.path;
            match path.segments.last() {
                Some(seg) => {
                    if seg.ident.to_string() == "Vec" {
                        match &seg.arguments {
                            syn::PathArguments::AngleBracketed(a) => {
                                let args = &a.args;
                                if args.len() != 1 {
                                    Generic::None
                                } else {
                                    if let Some(syn::GenericArgument::Type(t)) = args.first() {
                                        Generic::Vec(t)
                                    } else {
                                        Generic::None
                                    }
                                }
                            }
                            _ => Generic::None,
                        }
                    } else if seg.ident.to_string() == "Option" {
                        match &seg.arguments {
                            syn::PathArguments::AngleBracketed(a) => {
                                let args = &a.args;
                                if args.len() != 1 {
                                    Generic::None
                                } else {
                                    if let Some(syn::GenericArgument::Type(t)) = args.first() {
                                        Generic::Opt(t)
                                    } else {
                                        Generic::None
                                    }
                                }
                            }
                            _ => Generic::None,
                        }
                    } else {
                        Generic::None
                    }
                }
                None => Generic::None,
            }
        }
        _ => Generic::None,
    }
}

pub enum Generic<'a> {
    Vec(&'a syn::Type),
    Opt(&'a syn::Type),
    None,
}

impl<'a> Generic<'a> {
    pub fn is_vec(&self) -> bool {
        match self {
            Generic::Vec(_) => true,
            _ => false,
        }
    }

    pub fn is_opt(&self) -> bool {
        match self {
            Generic::Opt(_) => true,
            _ => false,
        }
    }

    pub fn get_vec(&self) -> Option<&syn::Type> {
        match self {
            Generic::Vec(v) => Some(v),
            _ => None,
        }
    }

    pub fn get_opt(&self) -> Option<&syn::Type> {
        match self {
            Generic::Opt(v) => Some(v),
            _ => None,
        }
    }
}
