//! https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html

// TODO: are type definitions in this crate available in machine readable format?

use serde::Deserialize;

impl PrototypeApi {
    pub fn get() -> Self {
        serde_json::from_str(include_str!("../prototype-api.json"))
            .expect("Failed to parse prototype API")
    }
}

#[test]
fn test_prototype_api_parsing() {
    dbg!(PrototypeApi::get());
}

/// A string, which can be an identifier for something, or a description-like text formatted in Markdown.
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#basic-types
#[allow(non_camel_case_types)]
pub type string = String;

/// A number, which could either be an integer or a floating point number, as JSON doesn't distinguish between those two.
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#basic-types
#[allow(non_camel_case_types)]
pub type number = f64;

/// A boolean value, which is either true or false.
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#basic-types
#[allow(non_camel_case_types)]
pub type boolean = bool;

/// The format has some top level members indicating the context of the format. These are:
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#top-level-members
#[derive(Debug, Deserialize)]
pub struct PrototypeApi {
    /// The application this documentation is for.
    /// Will always be "factorio".
    pub application: string,
    /// Indicates the stage this documentation is for.
    /// Will always be "prototype"
    /// (as opposed to "runtime"; see the [data lifecycle](https://lua-api.factorio.com/latest/auxiliary/data-lifecycle.html) for more detail).
    pub stage: string,
    /// The version of the game that this documentation is for. An example would be "1.1.90".
    pub application_version: string,
    /// The version of the machine-readable format itself.
    /// It is incremented every time the format changes.
    /// The version this documentation reflects is stated at the top.
    /// TODO: maybe this should be integer, who knows :)
    pub api_version: number,

    /// The list of prototypes that can be created. Equivalent to the prototypes page.
    pub prototypes: Vec<Prototype>,
    /// The list of types (concepts) that the format uses. Equivalent to the types page.
    pub types: Vec<ConceptType>,
}

/// KEKW theres a typo in the link
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Protoype
#[derive(Debug, Deserialize)]
pub struct Prototype {
    /// The name of the prototype.
    pub name: string,
    /// The order of the prototype as shown in the HTML.
    pub order: number,
    /// The text description of the prototype.
    pub description: String,
    /// A list of Markdown lists to provide additional information.
    /// Usually contained in a spoiler tag.
    pub lists: Option<Vec<String>>,
    /// A list of code-only examples about the prototype.
    pub examples: Option<Vec<String>>,
    /// A list of illustrative images shown next to the prototype.
    pub images: Option<Vec<Image>>,
    /// The name of the prototype's parent, if any.
    pub parent: Option<String>,
    /// Whether the prototype is abstract, and thus can't be created directly.
    pub r#abstract: boolean,
    /// The type name of the prototype, like "boiler". `null` for abstract prototypes.
    pub typename: Option<String>,
    /// The maximum number of instances of this prototype that can be created, if any.
    pub instance_limit: Option<number>,
    /// Whether the prototype is deprecated and shouldn't be used anymore.
    pub deprecated: boolean,
    /// The list of properties that the prototype has. May be an empty array.
    pub properties: Vec<Property>,
    /// A special set of properties that the user can add an arbitrary number of.
    /// Specifies the type of the key and value of the custom property.
    pub custom_properties: Option<CustomProperties>,
}

/// Type/Concept
///
/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Concept
#[derive(Debug, Deserialize)]
pub struct ConceptType {
    /// The name of the type.
    pub name: string,
    /// The order of the type as shown in the HTML.
    pub order: number,
    /// The text description of the type.
    pub description: string,
    /// A list of Markdown lists to provide additional information. Usually contained in a spoiler tag.
    pub lists: Option<Vec<string>>,
    /// A list of code-only examples about the type.
    pub examples: Option<Vec<string>>,
    /// A list of illustrative images shown next to the type.
    pub images: Option<Vec<Image>>,
    /// The name of the type's parent, if any.
    pub parent: Option<string>,
    /// Whether the type is abstract, and thus can't be created directly.
    pub r#abstract: boolean,
    /// Whether the type is inlined inside another property's description.
    pub inline: boolean,
    /// The type of the type/concept (Yes, this naming is confusing).
    /// Either a proper [Type](https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Type),
    /// or the string "builtin", indicating a fundamental type like string or number.
    pub r#type: Type,
    /// The list of properties that the type has, if its type includes a struct. null otherwise.
    pub properties: Option<Vec<Property>>,
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Property
#[derive(Debug, Deserialize)]
pub struct Property {
    /// The name of the property.
    pub name: string,
    /// The order of the property as shown in the HTML.
    pub order: number,
    /// The text description of the property.
    pub description: string,
    /// A list of Markdown lists to provide additional information. Usually contained in a spoiler tag.
    pub lists: Option<Vec<string>>,
    /// A list of code-only examples about the property.
    pub examples: Option<Vec<string>>,
    /// A list of illustrative images shown next to the property.
    pub images: Option<Vec<Image>>,
    /// An alternative name for the property. Either this or name can be used to refer to the property.
    pub alt_name: Option<string>,
    /// Whether the property overrides a property of the same name in one of its parents.
    pub r#override: boolean,
    /// The type of the property.
    pub r#type: Type,
    /// Whether the property is optional and can be omitted. If so, it falls back to a default value.
    pub optional: boolean,
    /// The default value of the property. Either a textual description or a literal value.
    pub default: Option<PropertyDefaultValue>,
}

/// The default value of the property. Either a textual description or a literal value.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum PropertyDefaultValue {
    String(string),
    Literal(Literal),
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Type
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum Type {
    /// A type is either a string, in which case that string is the simple type
    Simple(string),
    /// Otherwise, a type is
    Complex(Box<ComplexType>),
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Type
#[derive(Debug, Deserialize)]
#[serde(tag = "complex_type")]
#[serde(rename_all = "snake_case")]
pub enum ComplexType {
    Array {
        /// The type of the elements of the array.
        value: Type,
    },
    Dictionary {
        /// The type of the keys of the dictionary.
        key: Type,
        /// The type of the values of the dictionary.
        value: Type,
    },
    Tuple {
        /// The types of the members of this tuple in order.
        values: Vec<Type>,
    },
    Union {
        /// A list of all compatible types for this type.
        options: Vec<Type>,
        /// Whether the options of this union have a description or not.
        full_format: boolean,
    },
    Literal(Literal),
    Type {
        /// The actual type. This format for types is used when they have descriptions attached to them.
        value: Type,
        /// The text description of the type.
        description: string,
    },
    /// Special type with no additional members. The properties themselves are listed on the API member that uses this type.
    Struct,
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Literal
#[derive(Debug, Deserialize)]
#[serde(tag = "complex_type", rename = "literal")]
pub struct Literal {
    /// The value of the literal.
    pub value: LiteralValue,
    /// The text description of the literal, if any.
    pub description: Option<string>,
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#Image
#[derive(Debug, Deserialize)]
pub struct Image {
    /// The name of the image file to display. These files are placed into the `/static/images/` directory.
    pub filename: string,
    /// The explanatory text to show attached to the image.
    pub caption: Option<string>,
}

/// https://lua-api.factorio.com/latest/auxiliary/json-docs-prototype.html#CustomProperties
#[derive(Debug, Deserialize)]
pub struct CustomProperties {
    /// The text description of the property.
    pub description: string,
    /// A list of Markdown lists to provide additional information. Usually contained in a spoiler tag.
    pub lists: Option<Vec<string>>,
    /// A list of code-only examples about the property.
    pub examples: Option<Vec<string>>,
    /// A list of illustrative images shown next to the property.
    pub images: Option<Vec<Image>>,
    /// The type of the key of the custom property.
    pub key_type: Type,
    /// The type of the value of the custom property.
    pub value_type: Type,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum LiteralValue {
    String(string),
    Number(number),
    Boolean(boolean),
}
