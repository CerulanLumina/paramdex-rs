use std::str::FromStr;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "deserialize/fielddef.pest"]
struct DefParser;

use pest::iterators::{Pair, Pairs};
use pest::{Parser, Span};
use pest::error::ErrorVariant;
use crate::{DummyType, ParamFieldDef, ParamFieldType};

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
    #[error("Failed to parse def line: {0}")]
    ParseError(#[from] pest::error::Error<Rule>),
}

use thiserror::Error;

pub fn parse_param_field_def<S: AsRef<str>>(input_str: S) -> Result<ParamFieldDef, DefParseError> {
    let tokenized = tokenize(input_str.as_ref())?.next().expect("First pair");
    assert_eq!(tokenized.as_rule(), Rule::def, "Rule is not def");
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

            let field_name = get_field_name(&mut inner);

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
                            return Err(
                                pest::error::Error::new_from_span(
                                    ErrorVariant::CustomError {
                                        message: String::from("Bit size not supported on this type")
                                    },
                                    span,
                                ).into());
                        }
                    }
                    Rule::def_default_suffix => {
                        let default_val = get_default(suffix);
                        compiled_field_def.default_value.replace(default_val);
                    }
                    _ => unreachable!()
                }
            }
            Ok(compiled_field_def)
        }
        Rule::def_dummy => {
            let mut inner = inner.into_inner();
            assert_eq!(inner.next().expect("after dummy").as_rule(), Rule::dummy_field_type);
            let field_name = get_field_name(&mut inner);
            let mut dummy_length: Option<DummyType> = None;
            let mut default: Option<f64> = None;
            for suffix in inner {
                match suffix.as_rule() {
                    Rule::suffix_array => {
                        dummy_length.replace(
                            DummyType::Bytes(get_array_size(suffix))
                        );
                    }
                    Rule::suffix_bitsize => {
                        dummy_length.replace(
                            DummyType::Bits(get_bitsize(suffix))
                        );
                    }
                    Rule::def_default_suffix => {
                        default.replace(get_default(suffix));
                    }
                    _ => unreachable!()
                }
            }
            Ok(ParamFieldDef {
                name: field_name,
                default_value: default,
                field_type: ParamFieldType::dummy8 { length: dummy_length },
            })
        },
        Rule::def_fixstr => {
            let mut inner = inner.into_inner();
            let fixstr_type = inner.next().expect("fixstr_type");
            let name = inner.next().expect("field_name").as_str().into();
            let suffix_array = inner.next().expect("suffix_array");
            let array_len = get_array_size(suffix_array);
            let field_type = match fixstr_type.as_str() {
                "fixstr" => ParamFieldType::fixstr { length: array_len },
                "fixstrW" => ParamFieldType::fixstrW { length: array_len },
                _ => unreachable!()
            };
            Ok(ParamFieldDef { name, field_type, default_value: None })
        }
        Rule::def_unrecog => {
            let inner = inner.into_inner().next().expect("field type");
            Err(DefParseError::ParseError(pest::error::Error::new_from_span(ErrorVariant::CustomError { message: "Unrecognized type".into() },inner.as_span())))
        }
        _ => unreachable!()
    }
}

fn get_default(inner: Pair<Rule>) -> f64 {
    assert_eq!(inner.as_rule(), Rule::def_default_suffix, "Rule is not default");
    let default_inner = inner.into_inner().next().expect("default inner");
    f64::from_str(default_inner.as_str()).expect("default str to f64")
}

fn parse_pair<T: FromStr>(inner: Pair<Rule>) -> T where <T as FromStr>::Err: std::fmt::Debug {
    let text = inner.as_str();
    T::from_str(text).expect("parse invalid")
}

fn get_array_size(inner: Pair<Rule>) -> usize {
    assert_eq!(inner.as_rule(), Rule::suffix_array, "Rule is not suffix_array");
    let num = inner.into_inner().next().expect("getting number");
    parse_pair(num)
}

fn get_bitsize(inner: Pair<Rule>) -> u8 {
    assert_eq!(inner.as_rule(), Rule::suffix_bitsize, "Rule is not suffix_bitsize");
    let num = inner.into_inner().next().expect("getting number");
    parse_pair(num)
}

fn get_field_name(inner: &mut Pairs<Rule>) -> String {
    let inner = inner.next().expect("getting field name");
    assert_eq!(inner.as_rule(), Rule::field_name);
    inner.as_str().to_owned()
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
    use crate::{DummyType, ParamFieldDef, ParamFieldType};
    use crate::deserialize::field_def_parse::parse_param_field_def;

    impl PartialEq for ParamFieldDef {
        fn eq(&self, other: &Self) -> bool {
            self.field_type == other.field_type && self.default_value == other.default_value && self.name == other.name
        }
    }

    #[test]
    fn dummy_parse_array() {
        let def = "dummy8 reserve_last[32]";
        assert_eq!(
            parse_param_field_def(def).expect("parses"),
            ParamFieldDef {
                name: "reserve_last".to_string(),
                default_value: None,
                field_type: ParamFieldType::dummy8 { length: Some(DummyType::Bytes(32)) },
            }
        )
    }

    #[test]
    fn dummy_parse_bitsize() {
        let def = "dummy8 disableParamReserve1:7";
        assert_eq!(
            parse_param_field_def(def).expect("parses"),
            ParamFieldDef {
                name: "disableParamReserve1".to_string(),
                default_value: None,
                field_type: ParamFieldType::dummy8 { length: Some(DummyType::Bits(7)) },
            }
        )
    }

    #[test]
    fn dummy_with_array_and_default() {
        let def = "dummy8 pad_3[16] = -1";
        assert_eq!(
            parse_param_field_def(def).expect("parses"),
            ParamFieldDef {
                name: "pad_3".to_string(),
                default_value: Some(-1.0),
                field_type: ParamFieldType::dummy8 { length: Some(DummyType::Bytes(16)) },
            }
        )
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
    fn fixstr() {
        assert_eq!(
            parse_param_field_def("fixstr texName_00[16]").expect("parses"),
            ParamFieldDef {
                name: "texName_00".into(),
                default_value: None,
                field_type: ParamFieldType::fixstr { length: 16 }
            }
        )
    }

    #[test]
    fn fixstrw() {
        assert_eq!(
            parse_param_field_def("fixstrW texName_00[16]").expect("parses"),
            ParamFieldDef {
                name: "texName_00".into(),
                default_value: None,
                field_type: ParamFieldType::fixstrW { length: 16 }
            }
        )
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
        let mut string = String::with_capacity(25000);
        for a in walkdir {
            let path = a.to_str().expect("tostr");
            let string =
                {
                    string.clear();
                    BufReader::new(File::open(&a).expect("opening file"))
                        .read_to_string(&mut string).expect("reading file");
                    &string
                };
            let doc = roxmltree::Document::parse(string.as_str()).expect("parsing xml");
            for def_str in doc.descendants().filter(|a| a.has_tag_name("Field")).map(|a| a.attribute("Def").expect("def")) {
                match parse_param_field_def(def_str) {
                    Err(err) => {
                        if !errors.contains_key(path) {
                            errors.insert(path.to_owned(), Vec::new());
                        }
                        errors.get_mut(path).expect("wewe").push((def_str.to_owned(), err));
                    }
                    _ => {}
                }
            }
        }

        for (k, v) in &errors {
            println!("{}", k);
            for def in v {
                let out = format!("{}", &def.1);
                out.lines().for_each(|a| println!("\t{}", a))
            }
        }

        assert!(errors.is_empty(), "Errors occurred in parsing, check above list");
    }
}