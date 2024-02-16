use heck::*;
use std::collections::HashMap;
use std::fmt::Write;
use std::path::Path;
use templing::*;

#[derive(Debug)]
enum TypeFormat<'a> {
    Normal(String),
    Literal(&'a factorio_prototype_api::Literal),
}

impl TypeFormat<'_> {
    fn unwrap_normal(self) -> String {
        if let Self::Normal(normal) = self {
            normal
        } else {
            panic!("not normal... is {self:?}")
        }
    }
}

fn format_type<'a>(
    r#type: &'a factorio_prototype_api::Type,
    make_union_name: &mut impl FnMut(&[factorio_prototype_api::Type]) -> String,
) -> TypeFormat<'a> {
    TypeFormat::Normal(match r#type {
        factorio_prototype_api::Type::Simple(simple) => simple.clone(),
        factorio_prototype_api::Type::Complex(r#type) => match &**r#type {
            factorio_prototype_api::ComplexType::Array { value } => {
                format!(
                    "Vec<{}>",
                    format_type(value, make_union_name).unwrap_normal()
                )
            }
            factorio_prototype_api::ComplexType::Dictionary { key, value } => {
                format!(
                    "HashMap<{}, {}>",
                    format_type(key, make_union_name).unwrap_normal(),
                    format_type(value, make_union_name).unwrap_normal(),
                )
            }
            factorio_prototype_api::ComplexType::Tuple { values } => {
                let mut s = String::new();
                s.push('(');
                for (index, value) in values.iter().enumerate() {
                    if index != 0 {
                        s.push(' ');
                    }
                    s.push_str(&format_type(value, make_union_name).unwrap_normal());
                    s.push(',');
                }
                s.push(')');
                s
            }
            factorio_prototype_api::ComplexType::Union {
                options,
                full_format,
            } => make_union_name(options),
            factorio_prototype_api::ComplexType::Literal(literal) => {
                return TypeFormat::Literal(literal);
            }
            factorio_prototype_api::ComplexType::Type { value, description } => match value {
                factorio_prototype_api::Type::Simple(value) => value.clone(),
                factorio_prototype_api::Type::Complex(_) => todo!(),
            },
            factorio_prototype_api::ComplexType::Struct => "()".to_owned(), // "serde_json::Value".to_owned(),
        },
    })
}

fn make_union(name: &str, options: &[factorio_prototype_api::Type]) -> String {
    let mut extras = Vec::new();
    let mut result = String::new();
    macro_rules! line {
        ($($t:tt)*) => {{
            writeln!(result, $($t)*).unwrap();
        }};
    }
    #[derive(Clone)]
    struct Variant {
        name: String,
        description: Option<String>,
        untagged: bool,
        aliases: Vec<String>,
        r#type: Option<String>,
    }
    let mut variants = Vec::new();
    for (index, option) in options.iter().enumerate() {
        let type_format = format_type(option, &mut |options| {
            let name = format!("{name}Option{index}");
            extras.push(make_union(&name, options));
            name
        });
        match type_format {
            TypeFormat::Normal(r#type) => {
                variants.push(Variant {
                    name: format!("Option{index}"),
                    r#type: Some(r#type),
                    untagged: true,
                    aliases: vec![],
                    description: None,
                });
            }
            TypeFormat::Literal(literal) => {
                let description = literal.description.clone();
                match &literal.value {
                    factorio_prototype_api::LiteralValue::String(literal) => {
                        variants.push(Variant {
                            name: literal.to_upper_camel_case(),
                            r#type: None,
                            untagged: false,
                            aliases: vec![literal.clone()],
                            description,
                        });
                    }
                    factorio_prototype_api::LiteralValue::Number(value) => {
                        variants.push(Variant {
                            name: format!("Option{index}"),
                            r#type: Some("f64".into()),
                            untagged: true,
                            aliases: vec![],
                            description,
                        });
                    }
                    factorio_prototype_api::LiteralValue::Boolean(value) => {
                        variants.push(Variant {
                            name: format!("Option{index}"),
                            r#type: Some("bool".into()),
                            untagged: true,
                            aliases: vec![],
                            description,
                        });
                    }
                };
            }
        };
    }

    let by_name: HashMap<String, usize> = variants
        .iter()
        .enumerate()
        .map(|(index, variant)| (variant.name.clone(), index))
        .collect();

    for (index, variant) in variants.clone().into_iter().enumerate() {
        let merge_into = by_name[&variant.name];
        if merge_into != index {
            let merge_into = &mut variants[merge_into];
            assert_eq!(merge_into.description, variant.description);
            assert_eq!(merge_into.r#type, variant.r#type);
            assert_eq!(merge_into.untagged, variant.untagged);
            merge_into.aliases.extend(variant.aliases);
        }
    }

    let mut variants: Vec<_> = by_name
        .into_values()
        .map(|index| &variants[index])
        .collect();

    // all variants with the #[serde(untagged)] attribute must be placed at the end of the enum
    variants.sort_by_key(|variant| variant.untagged);

    line!("#[derive(Debug, Deserialize)]");
    line!("#[derive(Clone, PartialEq, Eq, Hash)]");

    line!("pub enum {name} {{");
    for variant in variants {
        if let Some(description) = &variant.description {
            line!("\t#[doc = {description:?}]");
        }
        if variant.untagged {
            line!("\t#[serde(untagged)]")
        }
        for (index, alias) in variant.aliases.iter().enumerate() {
            if index == 0 {
                line!("\t#[serde(rename = {alias:?})]");
            } else {
                line!("\t#[serde(alias = {alias:?})]");
            }
        }
        if let Some(r#type) = &variant.r#type {
            line!("\t{}({type}),", variant.name);
        } else {
            line!("\t{},", variant.name);
        }
    }
    line!("}}");
    for extra in extras {
        let extra = extra.trim();
        line!("{extra}");
    }
    result
}

fn main() {
    let prototype_api = factorio_prototype_api::PrototypeApi::get();
    let mut lib = String::new();

    macro_rules! line {
        ($($t:tt)*) => {{
            writeln!(lib, $($t)*).unwrap();
        }};
    }

    for concept_type in &prototype_api.types {
        let mut extras = Vec::new();
        match &concept_type.r#type {
            factorio_prototype_api::Type::Simple(simple) => {
                let simple = if simple == "builtin" {
                    match concept_type.name.as_str() {
                        "uint8" => "u8",
                        "uint16" => "u16",
                        "uint32" => "u32",
                        "uint64" => "u64",
                        "int8" => "i8",
                        "int16" => "i16",
                        "int32" => "i32",
                        "int64" => "i64",
                        "bool" => {
                            // Because "type bool = bool";
                            continue;
                        }
                        "float" => "R32",
                        "double" => "R64",
                        "string" => "String",
                        "DataExtendMethod" => {
                            // TODO wat
                            "()"
                        }
                        name => panic!("Builtin {name} is wat?"),
                    }
                } else {
                    simple
                };
                line!("#[allow(non_camel_case_types)]");
                line!("pub type {} = {};", concept_type.name, simple);
            }
            factorio_prototype_api::Type::Complex(r#type) => {
                match &**r#type {
                    factorio_prototype_api::ComplexType::Struct => {
                        line!("#[doc = {:?}]", concept_type.description);
                        line!("#[derive(Debug, Deserialize, Deref)]");
                        line!("#[derive(Clone, PartialEq, Eq, Hash)]");
                        line!(
                            "pub struct {}(Box<{}Impl>);",
                            concept_type.name,
                            concept_type.name,
                        );

                        let properties = concept_type.properties.as_ref().unwrap();
                        let tag = properties.iter().find_map(|property| {
                            if property.name != "type" {
                                // Hmmmm
                                return None;
                            }
                            let factorio_prototype_api::Type::Complex(r#type) = &property.r#type
                            else {
                                return None;
                            };
                            match &**r#type {
                                factorio_prototype_api::ComplexType::Literal(
                                    factorio_prototype_api::Literal {
                                        value,
                                        description: _,
                                    },
                                ) => Some((property.name.as_str(), value)),
                                _ => None,
                            }
                        });

                        line!("#[derive(Debug, Deserialize)]");
                        line!("#[derive(Clone, PartialEq, Eq, Hash)]");
                        if concept_type.parent.is_some() {
                            line!("#[derive(Deref)]");
                        }
                        if let Some((tag_name, tag_value)) = tag {
                            let tag_value = match tag_value {
                                factorio_prototype_api::LiteralValue::String(value) => {
                                    value.as_str()
                                }
                                other => panic!("tag value is {other:?}"),
                            };
                            line!("#[serde(tag = {tag_name:?})]");
                            line!("#[serde(rename = {tag_value:?})]");
                        }
                        let mut default_fns = Vec::new();
                        line!("pub struct {}Impl {{", concept_type.name);
                        if let Some(parent) = &concept_type.parent {
                            line!("\t#[deref]");
                            line!("\t#[serde(flatten)]");
                            line!("\tpub parent: {parent},");
                        }
                        for property in properties {
                            if let Some((tag_name, _)) = tag {
                                if property.name == tag_name {
                                    continue;
                                }
                            }
                            if property.r#override {
                                // TODO
                            }
                            let r#type = format_type(&property.r#type, &mut |options| {
                                let name = format!(
                                    "{}{}Union",
                                    concept_type.name,
                                    property.name.to_upper_camel_case(),
                                );
                                let extra = make_union(&name, options);
                                extras.push(extra);
                                name
                            });
                            let mut r#type = match r#type {
                                TypeFormat::Normal(r#type) => r#type,
                                TypeFormat::Literal(..) => {
                                    // TODO https://github.com/serde-rs/serde/issues/760
                                    continue;
                                }
                            };
                            line!("\t#[doc = {:?}]", property.description);
                            if let Some(default) = &property.default {
                                // this can have values in plain english lol
                                if false {
                                    let default_fn = format!(
                                        "{}_{}_default_value",
                                        concept_type.name.to_snake_case(),
                                        property.name,
                                    );
                                    line!("\t#[serde(default = {default_fn:?})]");
                                    default_fns.push((
                                        default_fn,
                                        r#type.clone(),
                                        property.optional,
                                        default,
                                    ));
                                }
                            }
                            if property.optional {
                                r#type = format!("Option<{type}>");
                            }
                            if let Some(alt_name) = &property.alt_name {
                                line!("\t#[serde(alias = {alt_name:?})]");
                            }
                            line!("\tpub r#{}: {},", property.name, r#type);
                        }
                        line!("}}");
                        for (default_fn, r#type, optional, value) in default_fns {
                            if optional {
                                line!("fn {default_fn}() -> Option<{type}> {{");
                            } else {
                                line!("fn {default_fn}() -> {type} {{");
                            }
                            let value = match value {
                                factorio_prototype_api::PropertyDefaultValue::String(repr) => {
                                    format!("{repr:?}.into()")
                                }
                                factorio_prototype_api::PropertyDefaultValue::Literal(literal) => {
                                    match &literal.value {
                                        factorio_prototype_api::LiteralValue::String(s) => {
                                            format!("{s:?}.into()")
                                        }
                                        factorio_prototype_api::LiteralValue::Number(x) => {
                                            format!("{x:?}.try_into().expect(\"LOL\")")
                                        }
                                        factorio_prototype_api::LiteralValue::Boolean(b) => {
                                            format!("{b:?}.into()")
                                        }
                                    }
                                }
                            };
                            if optional {
                                line!("\tSome({value})");
                            } else {
                                line!("\t{value}");
                            }
                            line!("}}");
                        }
                    }
                    _ => {
                        fn make_union_name(
                            concept_type: &factorio_prototype_api::ConceptType,
                            options: &[factorio_prototype_api::Type],
                            extras: &mut Vec<String>,
                        ) -> String {
                            let name = format!("{}Union", concept_type.name);
                            let extra = make_union(&name, options);
                            extras.push(extra);
                            name
                        }

                        let r#type = format_type(&concept_type.r#type, &mut |options| {
                            make_union_name(concept_type, options, &mut extras)
                        })
                        .unwrap_normal();
                        line!("#[allow(non_camel_case_types)]");
                        line!("pub type {} = {};", concept_type.name, r#type);
                    }
                }
            }
        }

        for extra in extras {
            let extra = extra.trim();
            line!("{extra}");
        }
    }
    std::fs::write(
        Path::new(&std::env::var("OUT_DIR").expect("Expected OUT_DIR env var"))
            .join("generated.rs"),
        lib,
    )
    .expect("Failed to write output");
}
