use crate::symbol::XML_SERDE;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::Meta;

pub fn get_xmlserde_meta_items(attr: &syn::Attribute) -> Result<Vec<syn::Meta>, ()> {
    if attr.path() != XML_SERDE {
        return Ok(Vec::new());
    }

    match attr.parse_args_with(Punctuated::<Meta, Comma>::parse_terminated) {
        Ok(meta) => Ok(meta.into_iter().collect()),
        Err(_) => Err(()),
    }
}

pub fn get_lit_byte_str<'a>(expr: &syn::Expr) -> Result<&syn::LitByteStr, ()> {
    if let syn::Expr::Lit(lit) = expr {
        if let syn::Lit::ByteStr(l) = &lit.lit {
            return Ok(l);
        }
    }
    Err(())
}

pub fn get_lit_str<'a>(lit: &syn::Expr) -> Result<&syn::LitStr, ()> {
    if let syn::Expr::Lit(lit) = lit {
        if let syn::Lit::Str(l) = &lit.lit {
            return Ok(&l);
        }
    }
    Err(())
}

pub fn get_array_lit_str<'a>(expr: &syn::Expr) -> Result<Vec<&syn::LitStr>, ()> {
    if let syn::Expr::Array(array) = expr {
        array.elems.iter().map(get_lit_str).collect()
    } else {
        Err(())
    }
}
