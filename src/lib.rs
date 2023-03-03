#[macro_export]
macro_rules! xml_serde_enum {
    (
         $(#[$outer:meta])*
        $name:ident {
            $($f:ident => $s:literal,)*
        }
    ) => {
        #[warn(dead_code)]
        $(#[$outer])*
        pub enum $name {
            $($f,)*
        }

        impl xmlserde::XmlValue for $name {
            fn serialize(&self) -> String {
                match &self {
                    $(Self::$f => String::from($s),)*
                }
            }
            fn deserialize(s: &str) -> Result<Self, String> {
                match s {
                    $($s => Ok(Self::$f),)*
                    _ => Err(String::from("")),
                }
            }
        }
    };
}

use std::io::{BufRead, Write};

use quick_xml::events::Event;

pub trait XmlSerialize {
    fn serialize<W: Write>(&self, tag: &[u8], writer: &mut quick_xml::Writer<W>);
    fn ser_root() -> Option<&'static [u8]> {
        None
    }
}

impl<T: XmlSerialize> XmlSerialize for Option<T> {
    fn serialize<W: Write>(&self, tag: &[u8], writer: &mut quick_xml::Writer<W>) {
        match self {
            Some(t) => t.serialize(tag, writer),
            None => {}
        }
    }
}

impl<T: XmlSerialize> XmlSerialize for Vec<T> {
    fn serialize<W: Write>(&self, tag: &[u8], writer: &mut quick_xml::Writer<W>) {
        self.iter().for_each(|c| {
            let _ = c.serialize(tag, writer);
        });
    }
}

pub trait XmlDeserialize {
    fn deserialize<B: BufRead>(
        tag: &[u8],
        reader: &mut quick_xml::Reader<B>,
        attrs: quick_xml::events::attributes::Attributes,
        is_empty: bool,
    ) -> Self;

    fn de_root() -> Option<&'static [u8]> {
        None
    }

    // Used when ty = `untag`.
    fn __get_children_tags() -> Vec<&'static [u8]> {
        vec![]
    }
}

///
/// Some structs are difficult to parse and Fortunately, those structs
/// have little affect to us. We just need to read and write them. We use `Unparsed`
/// to keep them.
#[derive(Debug)]
pub struct Unparsed {
    data: Vec<Event<'static>>,
    attrs: Vec<(String, String)>,
}

impl XmlSerialize for Unparsed {
    fn serialize<W: Write>(&self, tag: &[u8], writer: &mut quick_xml::Writer<W>) {
        use quick_xml::events::*;
        let mut start = BytesStart::borrowed_name(tag);
        self.attrs.iter().for_each(|(k, v)| {
            let k = k as &str;
            let v = v as &str;
            start.push_attribute((k, v));
        });
        if self.data.len() > 0 {
            let _ = writer.write_event(Event::Start(start));
            self.data.iter().for_each(|e| {
                let _ = writer.write_event(e);
            });
            let _ = writer.write_event(Event::End(BytesEnd::borrowed(tag)));
        } else {
            let _ = writer.write_event(Event::Empty(start));
        }
    }
}

impl XmlDeserialize for Unparsed {
    fn deserialize<B: BufRead>(
        tag: &[u8],
        reader: &mut quick_xml::Reader<B>,
        attrs: quick_xml::events::attributes::Attributes,
        is_empty: bool,
    ) -> Self {
        use quick_xml::events::*;
        let mut attrs_vec = Vec::<(String, String)>::new();
        let mut data = Vec::<Event<'static>>::new();
        let mut buf = Vec::<u8>::new();
        attrs.into_iter().for_each(|a| {
            if let Ok(attr) = a {
                let key = String::from_utf8(attr.key.to_vec()).unwrap_or(String::from(""));
                let value = String::from_utf8(attr.value.to_vec()).unwrap_or(String::from(""));
                attrs_vec.push((key, value))
            }
        });
        if is_empty {
            return Unparsed {
                data,
                attrs: attrs_vec,
            };
        }
        loop {
            match reader.read_event(&mut buf) {
                Ok(Event::End(e)) if e.name() == tag => break,
                Ok(Event::Eof) => break,
                Err(_) => break,
                Ok(e) => data.push(e.into_owned()),
            }
        }
        Unparsed {
            data,
            attrs: attrs_vec,
        }
    }
}

pub fn xml_serialize_with_decl<T>(obj: T) -> String
where
    T: XmlSerialize,
{
    use quick_xml::events::BytesDecl;
    let mut writer = quick_xml::Writer::new(Vec::new());
    let decl = BytesDecl::new(
        b"1.0".as_ref(),
        Some(b"UTF-8".as_ref()),
        Some(b"yes".as_ref()),
    );
    let _ = writer.write_event(Event::Decl(decl));
    obj.serialize(
        T::ser_root().expect(r#"Expect a root element to serialize: #[xmlserde(root=b"tag")]"#),
        &mut writer,
    );
    String::from_utf8(writer.into_inner()).unwrap()
}

pub fn xml_serialize<T>(obj: T) -> String
where
    T: XmlSerialize,
{
    let mut writer = quick_xml::Writer::new(Vec::new());
    obj.serialize(T::ser_root().expect("Expect root"), &mut writer);
    String::from_utf8(writer.into_inner()).unwrap()
}

pub fn xml_deserialize_from_reader<T, R>(reader: R) -> Result<T, String>
where
    T: XmlDeserialize,
    R: BufRead,
{
    let mut reader = quick_xml::Reader::from_reader(reader);
    reader.trim_text(false);
    let mut buf = Vec::<u8>::new();
    let root = T::de_root().expect(r#"#[xmlserde(root = b"tag")]"#);
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(start)) => {
                if start.name() == root {
                    let result = T::deserialize(root, &mut reader, start.attributes(), false);
                    return Ok(result);
                }
            }
            Ok(Event::Empty(start)) => {
                if start.name() == root {
                    let result = T::deserialize(root, &mut reader, start.attributes(), true);
                    return Ok(result);
                }
            }
            Ok(Event::Eof) => {
                return Err(format!(
                    "Cannot find the element: {}",
                    String::from_utf8(root.to_vec()).unwrap()
                ))
            }
            Err(e) => return Err(e.to_string()),
            _ => {}
        }
    }
}

/// Keep reading event until meeting the Start Event named `root` and start to deserialize.
pub fn xml_deserialize_from_str<T>(xml_str: &str) -> Result<T, String>
where
    T: XmlDeserialize,
{
    xml_deserialize_from_reader(xml_str.as_bytes())
}

pub trait XmlValue: Sized {
    fn serialize(&self) -> String;
    fn deserialize(s: &str) -> Result<Self, String>;
}

impl XmlValue for bool {
    fn serialize(&self) -> String {
        if *self {
            String::from("1")
        } else {
            String::from("0")
        }
    }

    fn deserialize(s: &str) -> Result<Self, String> {
        if s == "1" || s == "true" {
            Ok(true)
        } else if s == "0" || s == "false" {
            Ok(false)
        } else {
            Err(format!("Cannot parse {} into a boolean", s))
        }
    }
}

impl XmlValue for String {
    fn serialize(&self) -> String {
        self.to_owned()
    }

    fn deserialize(s: &str) -> Result<Self, String> {
        Ok(s.to_owned())
    }
}

macro_rules! impl_xml_value_for_num {
    ($num:ty) => {
        impl XmlValue for $num {
            fn serialize(&self) -> String {
                self.to_string()
            }

            fn deserialize(s: &str) -> Result<Self, String> {
                let r = s.parse::<$num>();
                match r {
                    Ok(f) => Ok(f),
                    Err(e) => Err(e.to_string()),
                }
            }
        }
    };
}

impl_xml_value_for_num!(u8);
impl_xml_value_for_num!(u16);
impl_xml_value_for_num!(u32);
impl_xml_value_for_num!(u64);
impl_xml_value_for_num!(f64);
impl_xml_value_for_num!(i8);
impl_xml_value_for_num!(i16);
impl_xml_value_for_num!(i32);
impl_xml_value_for_num!(i64);
