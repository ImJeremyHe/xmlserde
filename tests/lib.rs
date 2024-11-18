#[cfg(test)]
mod tests {

    use xmlserde::{xml_deserialize_from_str, xml_serialize, Unparsed, XmlValue};
    use xmlserde::{xml_serde_enum, XmlDeserialize, XmlSerialize};
    use xmlserde_derives::{XmlDeserialize, XmlSerialize};

    #[test]
    fn xml_serde_enum_test() {
        xml_serde_enum! {
            T {
                A => "a",
                B => "b",
                C => "c",
            }
        }

        assert!(matches!(T::deserialize("c"), Ok(T::C)));
        assert!(matches!(T::deserialize("b"), Ok(T::B)));
        assert_eq!((T::A).serialize(), "a");
    }

    #[test]
    fn default_for_child() {
        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"property")]
        struct Property {
            #[xmlserde(name = b"name", ty = "attr")]
            name: String,
        }

        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"properties")]
        struct InnerProperties {
            #[xmlserde(name = b"property", ty = "child")]
            properties: Vec<Property>,
        }

        #[derive(Default)]
        struct Properties(Vec<Property>);

        impl XmlDeserialize for Properties {
            fn deserialize<B: std::io::prelude::BufRead>(
                tag: &[u8],
                reader: &mut xmlserde::quick_xml::Reader<B>,
                attrs: xmlserde::quick_xml::events::attributes::Attributes,
                is_empty: bool,
            ) -> Self {
                let inner = InnerProperties::deserialize(tag, reader, attrs, is_empty);
                Self(inner.properties)
            }
        }

        #[derive(XmlDeserialize)]
        #[xmlserde(root = b"namespace")]
        struct Namespace {
            #[xmlserde(name = b"properties", ty = "child", default = "Properties::default")]
            properties: Properties,
        }

        let xml = r#"<namespace>
        </namespace>"#;
        let result = xml_deserialize_from_str::<Namespace>(xml).unwrap();
        assert!(result.properties.0.is_empty(),);

        let xml = r#"<namespace>
            <properties>
                <property name="test" />
            </properties>
        </namespace>"#;
        let result = xml_deserialize_from_str::<Namespace>(xml).unwrap();
        assert_eq!(result.properties.0[0].name, "test",);
    }

    #[test]
    fn self_closed_boolean_child() {
        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"font")]
        struct Font {
            #[xmlserde(name = b"b", ty = "sfc")]
            bold: bool,
            #[xmlserde(name = b"i", ty = "sfc")]
            italic: bool,
            #[xmlserde(name = b"size", ty = "attr")]
            size: f64,
        }
        let xml = r#"<font size="12.2">
            <b/>
            <i/>
        </font>"#;
        let result = xml_deserialize_from_str::<Font>(xml);
        match result {
            Ok(f) => {
                assert_eq!(f.bold, true);
                assert_eq!(f.italic, true);
                assert_eq!(f.size, 12.2);
            }
            Err(_) => panic!(),
        }
    }

    #[test]
    fn derive_deserialize_vec_with_init_size_from_attr() {
        #[derive(XmlDeserialize, Default)]
        pub struct Child {
            #[xmlserde(name = b"age", ty = "attr")]
            pub age: u16,
            #[xmlserde(ty = "text")]
            pub name: String,
        }
        fn default_zero() -> u32 {
            0
        }
        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"root")]
        pub struct Aa {
            #[xmlserde(name = b"f", ty = "child", vec_size = "cnt")]
            pub f: Vec<Child>,
            #[xmlserde(name = b"cnt", ty = "attr", default = "default_zero")]
            pub cnt: u32,
        }
        let xml = r#"<root cnt="2">
            <f age="15"> Tom</f>
            <f age="9">Jerry</f>
        </root>"#;
        let result = xml_deserialize_from_str::<Aa>(xml);
        match result {
            Ok(result) => {
                assert_eq!(result.f.len(), 2);
                assert_eq!(result.cnt, 2);
                let mut child_iter = result.f.into_iter();
                let first = child_iter.next().unwrap();
                assert_eq!(first.age, 15);
                assert_eq!(first.name, String::from(" Tom"));
                let second = child_iter.next().unwrap();
                assert_eq!(second.age, 9);
                assert_eq!(second.name, String::from("Jerry"));
            }
            Err(_) => panic!(),
        }
    }

    #[test]
    fn derive_deserialize_vec_with_init_size() {
        #[derive(XmlDeserialize, Default)]
        pub struct Child {
            #[xmlserde(name = b"age", ty = "attr")]
            pub _age: u16,
            #[xmlserde(ty = "text")]
            pub _name: String,
        }
        fn default_zero() -> u32 {
            0
        }
        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"root")]
        pub struct Aa {
            #[xmlserde(name = b"f", ty = "child", vec_size = 10)]
            pub f: Vec<Child>,
            #[xmlserde(name = b"cnt", ty = "attr", default = "default_zero")]
            pub _cnt: u32,
        }
        let xml = r#"<root cnt="2">
            <f age="15">Tom</f>
            <f age="9">Jerry</f>
        </root>"#;
        let result = xml_deserialize_from_str::<Aa>(xml).unwrap();
        assert_eq!(result.f.capacity(), 10);
    }

    #[test]
    fn serialize_attr_and_text() {
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
            #[xmlserde(name = b"male", ty = "attr")]
            male: bool,
            #[xmlserde(name = b"name", ty = "text")]
            name: String,
        }
        let result = xml_serialize(Person {
            age: 12,
            male: true,
            name: String::from("Tom"),
        });
        assert_eq!(result, "<Person age=\"12\" male=\"1\">Tom</Person>");
    }

    #[test]
    fn serialize_attr_and_sfc() {
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
            #[xmlserde(name = b"male", ty = "sfc")]
            male: bool,
            #[xmlserde(name = b"lefty", ty = "sfc")]
            lefty: bool,
        }
        let p1 = Person {
            age: 16,
            male: false,
            lefty: true,
        };
        let result = xml_serialize(p1);
        assert_eq!(result, "<Person age=\"16\"><lefty/></Person>");
    }

    #[test]
    fn serialize_children() {
        #[derive(XmlSerialize)]
        struct Skills {
            #[xmlserde(name = b"eng", ty = "attr")]
            english: bool,
            #[xmlserde(name = b"jap", ty = "sfc")]
            japanese: bool,
        }
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
            #[xmlserde(name = b"skills", ty = "child")]
            skills: Skills,
        }

        let p = Person {
            age: 32,
            skills: Skills {
                english: false,
                japanese: true,
            },
        };
        let result = xml_serialize(p);
        assert_eq!(
            result,
            "<Person age=\"32\"><skills eng=\"0\"><jap/></skills></Person>"
        );
    }

    #[test]
    fn serialize_opt_attr() {
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: Option<u16>,
        }
        let p = Person { age: Some(2) };
        let result = xml_serialize(p);
        assert_eq!(result, "<Person age=\"2\"/>");
        let p = Person { age: None };
        let result = xml_serialize(p);
        assert_eq!(result, "<Person/>");
    }

    #[test]
    fn deserialize_opt_attr() {
        #[derive(XmlDeserialize, Default)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: Option<u16>,
        }
        let xml = r#"<Person age="2"></Person>"#;
        let result = xml_deserialize_from_str::<Person>(xml);
        match result {
            Ok(p) => assert_eq!(p.age, Some(2)),
            Err(_) => panic!(),
        }
    }

    #[test]
    fn deserialize_default() {
        fn default_age() -> u16 {
            12
        }
        #[derive(XmlDeserialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr", default = "default_age")]
            age: u16,
            #[xmlserde(name = b"name", ty = "text")]
            name: String,
        }
        let xml = r#"<Person>Tom</Person>"#;
        let result = xml_deserialize_from_str::<Person>(xml);
        match result {
            Ok(p) => {
                assert_eq!(p.age, 12);
                assert_eq!(p.name, "Tom");
            }
            Err(_) => panic!(),
        }
    }

    #[test]
    fn serialize_skip_default() {
        fn default_age() -> u16 {
            12
        }
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr", default = "default_age")]
            age: u16,
            #[xmlserde(name = b"name", ty = "text")]
            name: String,
        }

        let p = Person {
            age: 12,
            name: String::from("Tom"),
        };
        let result = xml_serialize(p);
        assert_eq!(result, "<Person>Tom</Person>")
    }

    #[test]
    fn serialize_with_ns() {
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"Person")]
        #[xmlserde(with_ns = b"namespace")]
        struct Person {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
            #[xmlserde(name = b"name", ty = "text")]
            name: String,
        }
        let p = Person {
            age: 12,
            name: String::from("Tom"),
        };
        let result = xml_serialize(p);
        assert_eq!(
            result,
            "<Person xmlns=\"namespace\" age=\"12\">Tom</Person>"
        );
    }

    #[test]
    fn scf_and_child_test() {
        #[derive(XmlDeserialize, XmlSerialize)]
        struct Child {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
        }

        #[derive(XmlDeserialize, XmlSerialize)]
        #[xmlserde(root = b"Person")]
        struct Person {
            #[xmlserde(name = b"lefty", ty = "sfc")]
            lefty: bool,
            #[xmlserde(name = b"c", ty = "child")]
            c: Child,
        }

        let xml = r#"<Person><lefty/><c age="12"/></Person>"#;
        let p = xml_deserialize_from_str::<Person>(xml).unwrap();
        let result = xml_serialize(p);
        assert_eq!(xml, result);
    }

    #[test]
    fn custom_ns_test() {
        #[derive(XmlDeserialize, XmlSerialize)]
        #[xmlserde(root = b"Child")]
        #[xmlserde(with_custom_ns(b"a", b"c"))]
        struct Child {
            #[xmlserde(name = b"age", ty = "attr")]
            age: u16,
        }
        let c = Child { age: 12 };
        let p = xml_serialize(c);
        assert_eq!(p, "<Child xmlns:a=\"c\" age=\"12\"/>");
    }

    #[test]
    fn enum_serialize_test() {
        #[derive(XmlDeserialize, XmlSerialize)]
        struct TestA {
            #[xmlserde(name = b"age", ty = "attr")]
            pub age: u16,
        }

        #[derive(XmlDeserialize, XmlSerialize)]
        struct TestB {
            #[xmlserde(name = b"name", ty = "attr")]
            pub name: String,
        }

        #[derive(XmlSerialize, XmlDeserialize)]
        enum TestEnum {
            #[xmlserde(name = b"testA")]
            TestA(TestA),
            #[xmlserde(name = b"testB")]
            TestB(TestB),
        }

        #[derive(XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"Child")]
        struct Child {
            #[xmlserde(name = b"dummy", ty = "child")]
            pub c: TestEnum,
        }

        let obj = Child {
            c: TestEnum::TestA(TestA { age: 23 }),
        };
        let xml = xml_serialize(obj);
        let p = xml_deserialize_from_str::<Child>(&xml).unwrap();
        match p.c {
            TestEnum::TestA(a) => assert_eq!(a.age, 23),
            TestEnum::TestB(_) => panic!(),
        }
    }

    #[test]
    fn unparsed_serde_test() {
        #[derive(XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"TestA")]
        pub struct TestA {
            #[xmlserde(name = b"others", ty = "child")]
            pub others: Unparsed,
        }

        let xml = r#"<TestA><others age="16" name="Tom"><gf/><parent><f/><m name="Lisa">1999</m></parent></others></TestA>"#;
        let p = xml_deserialize_from_str::<TestA>(&xml).unwrap();
        let ser = xml_serialize(p);
        assert_eq!(xml, ser);
    }

    #[test]
    fn untag_serde_test() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"Root")]
        pub struct Root {
            #[xmlserde(ty = "untag")]
            pub dummy: EnumA,
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub enum EnumA {
            #[xmlserde(name = b"a")]
            A1(Astruct),
            #[xmlserde(name = b"b")]
            B1(Bstruct),
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Astruct {
            #[xmlserde(name = b"aAttr", ty = "attr")]
            pub a_attr1: u32,
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Bstruct {
            #[xmlserde(name = b"bAttr", ty = "attr")]
            pub b_attr1: u32,
        }

        let xml = r#"<Root><a aAttr="3"/></Root>"#;
        let p = xml_deserialize_from_str::<Root>(&xml).unwrap();
        match p.dummy {
            EnumA::A1(ref a) => assert_eq!(a.a_attr1, 3),
            EnumA::B1(_) => panic!(),
        }
        let ser = xml_serialize(p);
        assert_eq!(xml, &ser);
    }

    #[test]
    fn vec_untag_serde_test() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"Root")]
        pub struct Root {
            #[xmlserde(ty = "untag")]
            pub dummy: Vec<EnumA>,
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub enum EnumA {
            #[xmlserde(name = b"a")]
            A1(Astruct),
            #[xmlserde(name = b"b")]
            B1(Bstruct),
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Astruct {
            #[xmlserde(name = b"aAttr", ty = "attr")]
            pub a_attr1: u32,
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Bstruct {
            #[xmlserde(name = b"bAttr", ty = "attr")]
            pub b_attr1: u32,
        }

        let xml = r#"<Root><a aAttr="3"/><b bAttr="5"/><a aAttr="4"/></Root>"#;
        let p = xml_deserialize_from_str::<Root>(&xml).unwrap();
        assert_eq!(p.dummy.len(), 3);
        let ser = xml_serialize(p);
        assert_eq!(xml, &ser);
    }

    #[test]
    fn option_untag_serde_test() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"Root")]
        pub struct Root {
            #[xmlserde(ty = "untag")]
            pub dummy: Option<EnumA>,
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub enum EnumA {
            #[xmlserde(name = b"a")]
            A1(Astruct),
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Astruct {
            #[xmlserde(name = b"aAttr", ty = "attr")]
            pub a_attr1: u32,
        }
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct Bstruct {
            #[xmlserde(name = b"bAttr", ty = "attr")]
            pub b_attr1: u32,
        }

        let xml = r#"<Root/>"#;
        let p = xml_deserialize_from_str::<Root>(&xml).unwrap();
        assert!(matches!(p.dummy, None));
        let xml = r#"<Root><a aAttr="3"/></Root>"#;
        let p = xml_deserialize_from_str::<Root>(&xml).unwrap();
        match p.dummy {
            Some(EnumA::A1(ref a)) => assert_eq!(a.a_attr1, 3),
            None => panic!(),
        }
        let ser = xml_serialize(p);
        assert_eq!(xml, &ser);
    }

    #[test]
    fn ser_opt_text() {
        #[derive(Debug, XmlSerialize)]
        #[xmlserde(root = b"ttt")]
        pub struct AStruct {
            #[xmlserde(ty = "text")]
            pub text: Option<String>,
        }

        let instance = AStruct {
            text: Some(String::from("hello world!")),
        };
        let expect = xml_serialize(instance);
        assert_eq!(expect, "<ttt>hello world!</ttt>");
    }

    #[test]
    fn test_generics() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"Root")]
        pub struct Root<T: XmlSerialize + XmlDeserialize> {
            #[xmlserde(ty = "untag")]
            pub dummy: Option<T>,
        }

        #[derive(XmlSerialize)]
        pub enum EnumB<T: XmlSerialize> {
            #[xmlserde(name = b"a")]
            #[allow(dead_code)]
            A1(T),
        }

        #[derive(Debug, XmlSerialize)]
        #[xmlserde(root = b"ttt")]
        pub struct AStruct {
            #[xmlserde(ty = "text")]
            pub text: Option<String>,
        }
    }

    #[test]
    fn test_untag_enum_no_type_child_and_text() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        struct Type {
            #[xmlserde(name = b"name", ty = "attr")]
            name: String,
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"parameter")]
        struct Parameter {
            #[xmlserde(ty = "untag")]
            ty: ParameterType,
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        enum ParameterType {
            #[xmlserde(name = b"varargs")]
            VarArgs,
            #[xmlserde(name = b"type")]
            Type(Type),
            #[xmlserde(ty = "text")]
            Text(String),
        }

        let xml = r#"<parameter><varargs /></parameter>"#;
        let p = xml_deserialize_from_str::<Parameter>(&xml).unwrap();
        assert!(matches!(p.ty, ParameterType::VarArgs));

        let expect = xml_serialize(p);
        assert_eq!(expect, "<parameter><varargs/></parameter>");

        let xml = r#"<parameter><type name="n"/></parameter>"#;
        let p = xml_deserialize_from_str::<Parameter>(&xml).unwrap();
        if let ParameterType::Type(t) = &p.ty {
            assert_eq!(t.name, "n")
        } else {
            panic!("")
        }
        let expect = xml_serialize(p);
        assert_eq!(expect, xml);

        let xml = r#"<parameter>ttttt</parameter>"#;
        let p = xml_deserialize_from_str::<Parameter>(&xml).unwrap();
        assert!(matches!(p.ty, ParameterType::Text(_)));
        let expect = xml_serialize(p);
        assert_eq!(expect, xml);
    }

    #[test]
    fn test_untag_enum_vec_and_text() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"text:p")]
        pub struct TextP {
            #[xmlserde(ty = "untag")]
            pub text_p_content: Vec<TextPContent>,
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub enum TextPContent {
            #[xmlserde(ty = "text")]
            Text(String),
            #[xmlserde(name = b"text:span", ty = "child")]
            TextSpan(TextSpan),
        }

        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        pub struct TextSpan {
            #[xmlserde(ty = "text", name = b"p")]
            pub t: String,
        }

        let xml = r#"<text:p>
            <text:span> text1 </text:span>
            <text:span>text2</text:span>
        </text:p>"#;
        let text_p = xml_deserialize_from_str::<TextP>(&xml).unwrap();
        let content = &text_p.text_p_content;
        assert_eq!(content.len(), 2);
        if let TextPContent::TextSpan(span) = content.get(0).unwrap() {
            assert_eq!(&span.t, " text1 ")
        } else {
            panic!("")
        }
        if let TextPContent::TextSpan(span) = content.get(1).unwrap() {
            assert_eq!(&span.t, "text2")
        } else {
            panic!("")
        }

        let expect = xml_serialize(text_p);
        assert_eq!(
            expect,
            "<text:p><text:span> text1 </text:span><text:span>text2</text:span></text:p>"
        );

        let xml = r#"<text:p>abcdefg</text:p>"#;
        let text_p = xml_deserialize_from_str::<TextP>(&xml).unwrap();
        let content = &text_p.text_p_content;
        assert_eq!(content.len(), 1);
        if let TextPContent::Text(s) = content.get(0).unwrap() {
            assert_eq!(s, "abcdefg")
        } else {
            panic!("")
        };
        let expect = xml_serialize(text_p);
        assert_eq!(expect, xml);
    }

    #[test]
    #[should_panic]
    fn test_unknown_fields_in_struct_deny_unknown_attr() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"pet")]
        #[xmlserde(deny_unknown_fields)]
        pub struct Pet {
            #[xmlserde(ty = "attr", name = b"name")]
            pub name: String,
        }
        let xml = r#"<pet name="Chaplin" age="1"/>"#;
        let _ = xml_deserialize_from_str::<Pet>(&xml).unwrap();
    }

    #[test]
    fn test_unknown_fields_in_struct_accept_unknown_attr() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"pet")]
        pub struct Pet {
            #[xmlserde(ty = "attr", name = b"name")]
            pub name: String,
        }
        let xml = r#"<pet name="Chaplin" age="1"/>"#;
        let _ = xml_deserialize_from_str::<Pet>(&xml).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_unknown_fields_in_struct_deny_unknown_field() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"pet")]
        #[xmlserde(deny_unknown_fields)]
        pub struct Pet {
            #[xmlserde(ty = "attr", name = b"name")]
            pub name: String,
        }
        let xml = r#"<pet name="Chaplin"><weight/></pet>"#;
        let _ = xml_deserialize_from_str::<Pet>(&xml).unwrap();
    }

    #[test]
    fn test_unknown_fields_in_struct_accept_unknown_field() {
        #[derive(Debug, XmlSerialize, XmlDeserialize)]
        #[xmlserde(root = b"pet")]
        pub struct Pet {
            #[xmlserde(ty = "attr", name = b"name")]
            pub name: String,
        }
        let xml = r#"<pet name="Chaplin"><weight/></pet>"#;
        let _ = xml_deserialize_from_str::<Pet>(&xml).unwrap();
    }

    // https://github.com/ImJeremyHe/xmlserde/issues/52
    #[test]
    fn test_issue_52() {
        #[derive(XmlSerialize)]
        #[xmlserde(root = b"root")]
        struct Wrapper<T: XmlSerialize> {
            #[xmlserde(name = b"header", ty = "attr")]
            header: String,
            #[xmlserde(ty = "untag")]
            body: T,
        }

        #[derive(XmlSerialize)]
        struct Foo {
            #[xmlserde(name = b"Bar", ty = "child")]
            bar: Bar,
        }

        #[derive(XmlSerialize)]
        struct Bar {}

        let wrapper = Wrapper {
            header: "".to_string(),
            body: Foo { bar: Bar {} },
        };

        let r = xml_serialize(wrapper);
        assert_eq!(r, r#"<root header=""><Bar/></root>"#);
    }

    #[test]
    fn test_de_untagged_struct() {
        #[derive(XmlDeserialize)]
        #[xmlserde(root = b"foo")]
        struct Foo {
            #[xmlserde(ty = "untagged_struct")]
            bar: Bar,
        }

        #[derive(XmlDeserialize)]
        struct Bar {
            #[xmlserde(name = b"a", ty = "child")]
            a: A,
            #[xmlserde(name = b"c", ty = "child")]
            c: C,
        }

        #[derive(XmlDeserialize)]
        struct A {
            #[xmlserde(name = b"attr1", ty = "attr")]
            attr1: u16,
        }

        #[derive(XmlDeserialize)]
        struct C {
            #[xmlserde(name = b"attr2", ty = "attr")]
            attr2: u16,
        }

        let xml = r#"<foo><a attr1="12"/><c attr2="200"/></foo>"#;
        let foo = xml_deserialize_from_str::<Foo>(&xml).unwrap();
        assert_eq!(foo.bar.a.attr1, 12);
        assert_eq!(foo.bar.c.attr2, 200);

        #[derive(XmlDeserialize)]
        #[xmlserde(root = b"foo")]
        struct FooOption {
            #[xmlserde(ty = "untagged_struct")]
            bar: Option<Bar>,
        }
        let xml = r#"<foo><a attr1="12"/><c attr2="200"/></foo>"#;
        let foo = xml_deserialize_from_str::<FooOption>(&xml).unwrap();
        let bar = foo.bar.unwrap();
        assert_eq!(bar.a.attr1, 12);
        assert_eq!(bar.c.attr2, 200);

        let xml = r#"<foo>></foo>"#;
        let foo = xml_deserialize_from_str::<FooOption>(&xml).unwrap();
        assert!(foo.bar.is_none());
    }
}
