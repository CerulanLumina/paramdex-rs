use std::str::FromStr;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "deserialize/fielddef.pest"]
struct DefParser;

use pest::iterators::Pairs;
use pest::{Parser, Span};
use crate::{ParamFieldDef, ParamFieldType};

fn tokenize(input: &str) -> Result<Pairs<Rule>, DefParseError> {
    DefParser::parse(Rule::def, input.as_ref()).map_err(|a| a.into())
}

#[derive(Debug)]
pub struct ErrSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Error, Debug)]
pub enum DefParseError {
    #[error("Failed to parse def line")]
    ParseError(#[from] pest::error::Error<Rule>),

    #[error("Bit size not supported on this type")]
    BitSizeUnsupported(ErrSpan),
}

use thiserror::Error;

pub fn parse_param_field_def<S: AsRef<str>>(input_str: S) -> Result<ParamFieldDef, DefParseError> {
    let tokenized = tokenize(input_str.as_ref())?.next().expect("First pair");
    if let Rule::def = tokenized.as_rule() {
        let inner = tokenized.into_inner().next().expect("def type pair");
        match inner.as_rule() {
            Rule::def_simple => {
                let mut inner = inner.into_inner();
                let simple_field_type = inner.next().expect("getting simple field type");

                let field_type = match simple_field_type.as_str() {
                    "s8" => ParamFieldType::s8,
                    "u8" => ParamFieldType::u8 { bit_size: None },
                    "s16" => ParamFieldType::s16,
                    "u16" => ParamFieldType::u16 { bit_size: None },
                    "s32" => ParamFieldType::s32,
                    "u32" => ParamFieldType::u32 { bit_size: None },
                    "f32" => ParamFieldType::f32,
                    "f64" => ParamFieldType::f64,
                    "a32" | "angle32" => ParamFieldType::a32,
                    "b32" => ParamFieldType::b32,
                    _ => unreachable!()
                };

                let field_name = {
                    let inner = inner.next().expect("getting field name");
                    inner.as_str().to_owned()
                };
                let mut compiled_field_def = ParamFieldDef {
                    field_type,
                    name: field_name,
                    default_value: None,
                };

                for suffix in inner {
                    match suffix.as_rule() {
                        Rule::suffix_bitsize => {
                            let span = suffix.as_span();
                            let number = suffix.into_inner().next().expect("number");
                            let bit_size = u8::from_str(number.as_str()).expect("number_str to u8");
                            if compiled_field_def.field_type.supports_bit_size() {
                                compiled_field_def.field_type.set_bit_size(bit_size);
                            } else {
                                return Err(DefParseError::BitSizeUnsupported(span.into()).into());
                            }
                        }
                        Rule::def_default_suffix => {
                            let default_inner = suffix.into_inner().next().expect("default inner");
                            let default_val = f64::from_str(default_inner.as_str()).expect("default str to f64");
                            compiled_field_def.default_value.replace(default_val);
                        }
                        _ => unreachable!()
                    }
                }
                Ok(compiled_field_def)
            }
            Rule::def_dummy => {
                unimplemented!()
            }
            _ => {
                unimplemented!()
            }
        }
    } else {
        unreachable!()
    }
}

impl<'i> From<Span<'i>> for ErrSpan {
    fn from(span: Span<'i>) -> Self {
        ErrSpan { start: span.start(), end: span.end() }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs::{File};
    use std::io::{BufReader, Read};
    use crate::{ParamFieldDef, ParamFieldType};
    use crate::deserialize::field_def_parse::parse_param_field_def;

    impl PartialEq for ParamFieldDef {
        fn eq(&self, other: &Self) -> bool {
            self.field_type == other.field_type && self.default_value == other.default_value && self.name == other.name
        }
    }

    #[test]
    fn simple() {
        assert_eq!(
            parse_param_field_def("u32 testingVar").expect("parses"),
            ParamFieldDef {
                name: "testingVar".to_string(),
                default_value: None,
                field_type: ParamFieldType::u32 { bit_size: None },
            }
        )
    }

    #[test]
    fn simple_s() {
        assert_eq!(
            parse_param_field_def("s32 testingVar2").expect("parses"),
            ParamFieldDef {
                name: "testingVar2".to_string(),
                default_value: None,
                field_type: ParamFieldType::s32,
            }
        )
    }

    #[test]
    fn simple_f() {
        assert_eq!(
            parse_param_field_def("f32 ｇradFactor").expect("parses"),
            ParamFieldDef {
                name: "ｇradFactor".to_string(),
                default_value: None,
                field_type: ParamFieldType::f32,
            }
        )
    }

    #[test]
    fn simple_bitsize() {
        assert_eq!(
            parse_param_field_def("u32 testingVar:3").expect("parses"),
            ParamFieldDef {
                name: "testingVar".to_string(),
                default_value: None,
                field_type: ParamFieldType::u32 { bit_size: Some(3) },
            }
        )
    }

    #[test]
    #[should_panic]
    fn simple_bitsize_s() {
        parse_param_field_def("s32 testingVar:3").expect("parses");
    }

    #[test]
    fn simple_default() {
        assert_eq!(
            parse_param_field_def("u32 testingVar = -3.0").expect("parses"),
            ParamFieldDef {
                name: "testingVar".to_string(),
                default_value: Some(-3.0),
                field_type: ParamFieldType::u32 { bit_size: None },
            }
        )
    }

    #[test]
    fn simple_default_s() {
        assert_eq!(
            parse_param_field_def("s32 testingVar3 = -3.0").expect("parses"),
            ParamFieldDef {
                name: "testingVar3".to_string(),
                default_value: Some(-3.0),
                field_type: ParamFieldType::s32,
            }
        )
    }

    #[test]
    fn simple_default_bitsize() {
        assert_eq!(
            parse_param_field_def("u32 testingVar:3 = 0").expect("parses"),
            ParamFieldDef {
                name: "testingVar".to_string(),
                default_value: Some(0.0),
                field_type: ParamFieldType::u32 { bit_size: Some(3) },
            }
        )
    }

    #[test]
    #[should_panic]
    fn simple_default_bitsize_s() {
        parse_param_field_def("s32 testingVar:3 = 0").expect("parses");
    }

    #[test]
    fn test_all_xmls() {
        let mut errors = HashMap::new();
        let walkdir = walkdir::WalkDir::new("Paramdex/ER")
            .into_iter()
            .filter_map(|a| a.ok())
            .filter(|a| a.file_type().is_file())
            .map(|a| a.into_path())
            .filter(|a| a.extension().map(|a| a == "xml").unwrap_or(false));
        for a in walkdir {
            let path = a.to_str().expect("tostr");
            let string =
                {
                    let mut string = String::with_capacity(25000);
                    BufReader::new(File::open(&a).expect("opening file"))
                        .read_to_string(&mut string).expect("reading file");
                    string
                };
            let doc = roxmltree::Document::parse(string.as_str()).expect("parsing xml");
            for def_str in doc.descendants().filter(|a| a.has_tag_name("Field")).map(|a| a.attribute("Def").expect("def")) {
                if !(def_str.contains("fixstr") || def_str.contains("dummy")) {
                    match parse_param_field_def(def_str) {
                        Err(_err) => {
                            if !errors.contains_key(path) {
                                errors.insert(path.to_owned(), Vec::new());
                            }
                            errors.get_mut(path).expect("wewe").push(def_str.to_owned());
                        },
                        _ => {}
                    }

                } else if def_str.contains("fixstrW") {
                    // println!("{}", path);
                }
            }
        }

        for (k,v) in errors {
            println!("{}", k);
            for def in v {
                println!("\t{}", def);
            }
        }

    }
}