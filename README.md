# xmlserde

[![MIT/Apache 2.0](https://img.shields.io/badge/license-MIT/Mit-blue.svg)](./LICENSE)

`xmlserde` is a tool for serializing or deserializing xml struct.
It is designed for [LogiSheets](https://github.com/proclml/LogiSheets), which is a spreadsheets application working on the browser.

You can check the detail of usage in the `workbook` directory or [here](https://github.com/logisky/LogiSheets/tree/master/crates/workbook).

## How to use `xmlserde`

`xmlserde` offers a range of macros that should suffice for most use cases. To utilize them, you need to include the following crates in your `Cargo.toml` file:

```toml
xmlserde = 0.7.1
xmlserde_derives = 0.7.1
```

### Deserialize

Start from deserializing would be easier to get closer to `xmlserde`.

Given the xml struct as below,

```xml
<person age ="16">Tom</person>
```

We can deserialize with these code:

```rs
use xmlserde_derives: XmlDerserialize
#[derive(XmlDeserialize)]
#[xmlserde(root = b"person")]
pub struct Person {
    #[xmlserde(name=b"age", ty="attr")]
    pub age: u8,
    #[xmlserde(ty ="text")]
    pub name: String,
}

fn deserialize_person() {
    use xmlserde::xml_deserialize_from_str;

    let s = r#"<person age ="16">Tom</person>"#;
    let p = xml_deserialize_from_str(s).unwrap();
    assert_eq!(p.age, 16);
    assert_eq!(p.name, "Tom");
}
```

You are supposed to declare that where the deserializer is to look for the values.

The commonly available *type*s are **attr**, **text** and **child**. In the above example, we instruct to program to navigate into the tag named `person` (using `xml_deserialize_from_str`), and to search for an attribute
with the key `age`. Additionally it specifies that the content of the text element represents the value of the field `name``.

You can specify the entry element for serialization/deserialization with xmlserde by using the annotation like `#[xmlserde(root = b"person")]`, thereby telling the program that the `person` element is the root for serde operations.

Below is an example illustrating how to deserialize a nested XML element:

```rs
#[derive(XmlDeserialize)]
#[xmlserde(root = b"person")]
pub struct Person {
    #[xmlserde(name=b"age", ty="attr")]
    pub age: u8,
    #[xmlserde(name = b"lefty", ty ="attr", default = "default_lefty")]
    pub lefty: bool,
    #[xmlserde(name = b"name", ty ="child")]
    pub name: Name,
}

#[derive(XmlDeserialize)]
pub struct Name {
    #[xmlserde(name = b"zh", ty ="attr")]
    pub zh: String,
    #[xmlserde(name = b"en", ty ="attr")]
    pub en: String,
}

fn deserialize_person() {
    use xmlserde::xml_deserialize_from_str;

    let s = r#"<person age ="16"><name zh="汤姆", en="Tom"/></person>"#;
    let p = xml_deserialize_from_str(s).unwrap();
    assert_eq!(p.age, 16);
    assert_eq!(p.name.en, "Tom");
    assert_eq!(p.lefty, false);
}

fn default_lefty() -> bool { false }
```

In the given example, we specify that the value of the `name` field is to be extracted from a child element tagged as `<name>`.
Consequently, the program will navigate into the `<name>` element and proceed with recursive deserialization.

Additionally, we specify that if the deserializer does not find a value for `lefty`, the default value for `lefty` should be set to false.

#### Vec

We support deserialize the fields whose types are `std::Vec<T: XmlDeserialize>`.

```rs
#[derive(XmlDeserialize)]
pub struct Pet {
    // Fields
}

#[derive(XmlDeserialize)]
#[xmlserde(root = b"person")]
pub struct Person {
    #[xmlserde(name = b"petCount", ty = "attr")]
    pub pet_count: u8,
    #[xmlserde(name = b"pet", ty = "child")]
    pub pets: Vec<Pet>
}
```

When the deserializer find the *pet* element, and it will know that this is an element of **pets**. You can even assign the capacity of the `Vec` with following:

```xml
#[xmlserde(name = b"pet", ty="child", vec_size=3)]
```

If the capacity is from an **attr**, you can:

```xml
#[xmlserde(name = b"pet", ty="child", vec_size="pet_count")]
```

#### Enum

We provide 2 patterns for deserializing `Enum`.

```rs
#[derive(XmlSerialize, XmlDeserialize)]
enum TestEnum {
    #[xmlserde(name = b"testA")]
    TestA(TestA),
    #[xmlserde(name = b"testB")]
    TestB(TestB),
}

#[derive(XmlSerialize, XmlDeserialize)]
#[xmlserde(root = b"personA")]
pub struct PersonA {
    #[xmlserde(name = b"e", ty = "child")]
    pub e: TestEnum
    // Other fields
}

#[derive(XmlSerialize, XmlDeserialize)]
#[xmlserde(root = b"personB")]
pub struct PersonB {
    #[xmlserde(ty = "untag")]
    pub dummy: TestEnum
    // Other fields
}
```

**PersonA** can be used to deserialize the xml struct like below:

```xml
<personA><e><testA/></e></personA>
```

or

```xml
<personA><e><testB/></e></personA>
```

And **PersonB** can be used to deserialize the xml which looks like:

```xml
<personB><testA/></personB>
```

or

```xml
<personB><testB/></personB>
```

You can use **untag** to **Option\<T\>** or **Vec\<T\>** where **T** is an **Enum**.

It means that the example below is legal:

```rust
#[derive(XmlSerialize, XmlDeserialize)]
#[xmlserde(root = b"personB")]
pub struct PersonB {
    #[xmlserde(ty = "untag")]
    pub dummy1: Enum1,
    #[xmlserde(ty = "untag")]
    pub dummy2: Option<Enum2>,
    #[xmlserde(ty = "untag")]
    pub dummy3: Vec<Enum3>,
    // Other fields
}
```

#### Unparsed

In situations where certain XML elements are not immediately relevant, but you wish to retain them for future serialization, we offer the `Unparsed` struct
for this purpose.

```rs
use xmlserde::Unparsed;

#[derive(XmlDeserialize)]
pub struct Person {
    #[xmlserde(name = b"educationHistory", ty = "child")]
    pub education_history: Unparsed,
}
```

### Serialize

Serialization is largely similar to deserialization. However, there are several key features that require consideration.

- default values will be skipped serializing.
If it is a **struct**, it should be implemented `Eq` trait.
- If a struct has no **child** or **text**, the result of serializing will
  look like this:

  ```xml
  <tag attr1="value1"/>
  ```

### Custom xmlserde

`xmlserde` offers the trait `XmlSerialize` and `XmlDeserialize`, allowing you
to dictate a struct's serialization and deserialization behavior by implementing
these traits.
At present, only built-in types are permitted for use as attributes. To enable custom types for use in attributes, you can implement the `XmlValue` trait on those types.

### Enum for string type

`xmlserde` also provides a macro called `xml_serde_enum` to serde `enum` for string type.

`xml_serde_enum` defines an `enum` and specifies the behavior of serialization and deserialization.

```rust
use xmlserde::xml_serde_enum;

xml_serde_enum!{
    #[derive(Debug)]
    Gender {
        Male => "male",
        Female => "female",
    }
}
```
