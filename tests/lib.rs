#[cfg(test)]
mod tests {

    use xmlserde::xml_serde_enum;
    use xmlserde::{xml_deserialize_from_str, xml_serialize, Unparsed, XmlValue};
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
            <f age="15">Tom</f>
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
                assert_eq!(first.name, String::from("Tom"));
                let second = child_iter.next().unwrap();
                assert_eq!(second.age, 9);
                assert_eq!(second.name, String::from("Jerry"));
            }
            Err(_) => panic!(),
        }
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
}
