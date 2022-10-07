use std::collections::HashMap;
use std::num::{ParseFloatError, ParseIntError};
use std::str::{FromStr, ParseBoolError};
use roxmltree::Node;
use thiserror::Error;
use crate::{EditFlags, ParamDef, ParamdefEndian, ParamdefFormat, ParamField, ParamFieldDef};

mod field_def_parse;

pub use field_def_parse::DefParseError;

const PARAM_DEF_ROOT: &'static str = "PARAMDEF";

impl FromStr for ParamDef {
    type Err = ParamdefDeserializeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        deserialize_def(s)
    }
}

pub fn deserialize_def<S: AsRef<str>>(input: S) -> Result<ParamDef, ParamdefDeserializeError> {
    let input = input.as_ref();

    let doc = roxmltree::Document::parse(input)?;

    let root = doc.root_element();
    if root.tag_name().name() != PARAM_DEF_ROOT {
        return Err(ParamdefDeserializeError::MissingParamData("Invalid root element".into()));
    }

    let mut root_config: HashMap<String, String> = HashMap::new();

    let mut fields: Option<Node> = None;

    for child in root.children() {
        match child.tag_name().name() {
            "Fields" => {
                fields.replace(child);
            }
            name => {
                root_config.insert(name.into(), child.text().ok_or(ParamdefDeserializeError::XmlBlankElement(name.into()))?.into());
            }
        }
    }

    let fields_node = fields.ok_or(ParamdefDeserializeError::MissingParamData("Fields".into()))?;

    let mut paramdef = ParamDef {
        param_type: get_or_error(&root_config, "ParamType").cloned()?,
        data_version: u32::from_str(get_or_error(&root_config, "DataVersion")?)?,
        endian: ParamdefEndian::from_str(get_or_error(&root_config, "BigEndian")?)?,
        string_format: ParamdefFormat::from_str(get_or_error(&root_config, "BigEndian")?)?,
        format_version: u32::from_str(get_or_error(&root_config, "FormatVersion")?)?,
        fields: Vec::new()
    };

    let fields = &mut paramdef.fields;

    for node in fields_node.children() {
        fields.push(parse_field_node(node)?);
    }

    Ok(paramdef)
}

fn get_or_error<'a>(map: &'a HashMap<String, String>, key: &str) -> Result<&'a String, ParamdefDeserializeError> {
    map.get(key).ok_or(ParamdefDeserializeError::MissingParamData(format!("{}", key)))
}

fn parse_field_node(field_node: Node) -> Result<ParamField, ParamdefDeserializeError> {
    let attr = field_node.attribute("Def").ok_or(ParamdefDeserializeError::MissingParamData("Field Def".into()))?;

    let mut field_config: HashMap<String, String> = HashMap::new();

    for child in field_node.children() {
        if let Some(text) = child.text() {
            field_config.insert(child.tag_name().name().into(), text.into());
        }
    }


    Ok(ParamField {

        field_def: ParamFieldDef::from_str(attr)?,

        display_name: field_config.get("DisplayName").cloned(),
        enum_tdf: field_config.get("Enum").cloned(),
        description: field_config.get("Description").cloned(),
        printf_format: field_config.get("DisplayFormat").cloned(),

        edit_flags: field_config.get("EditFlags").map(|a| EditFlags::from_str(a)).swap()?, // TODO

        minimum: field_config.get("Minimum").map(|a| f64::from_str(a.as_str())).swap()?,
        maximum: field_config.get("Maximum").map(|a| f64::from_str(a.as_str())).swap()?,
        increment: field_config.get("Increment").map(|a| f64::from_str(a.as_str())).swap()?,
        sort_id: field_config.get("SortID").map(|a| usize::from_str(a.as_str())).swap()?,
    })

}

impl FromStr for EditFlags {
    type Err = ParamdefDeserializeError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Ok(EditFlags {
            wrap: text.contains("Wrap"),
            lock: text.contains("Lock"),
        })
    }
}

impl FromStr for ParamFieldDef {
    type Err = DefParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        field_def_parse::parse_param_field_def(s)
    }
}

trait OptionResultExt<V, E> {
    fn swap(self) -> Result<Option<V>, E>;
}

impl<V, E> OptionResultExt<V, E> for Option<Result<V, E>> {
    fn swap(self) -> Result<Option<V>, E> {
        match self {
            Some(Ok(v)) => Ok(Some(v)),
            Some(Err(e)) => Err(e),
            None => Ok(None)
        }
    }
}

impl FromStr for ParamdefEndian {
    type Err = ParseBoolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
          bool::from_str(s).map(|a| a.into())
    }
}

impl From<bool> for ParamdefEndian {
    fn from(a: bool) -> Self {
        if a {
            Self::Big
        } else {
            Self::Little
        }
    }
}

impl FromStr for ParamdefFormat {
    type Err = ParseBoolError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        bool::from_str(s).map(|a| a.into())
    }
}

impl From<bool> for ParamdefFormat {
    fn from(a: bool) -> Self {
        if a {
            Self::UTF16
        } else {
            Self::ShiftJIS
        }
    }
}

#[derive(Error, Debug)]
pub enum ParamdefDeserializeError {
    #[error("XML parsing failed")]
    XmlParsing(#[from] roxmltree::Error),

    #[error("XML blank element")]
    XmlBlankElement(String),

    #[error("Parsing number from XML")]
    XmlParsingNumber(#[from] ParseIntError),

    #[error("Parsing bool from XML")]
    XmlParsingBool(#[from] ParseBoolError),

    #[error("Parsing float from XML")]
    XmlParsingFloat(#[from] ParseFloatError),

    #[error("A required field in the XML was missing")]
    MissingParamData(String),

    #[error("Failed to parse field def string")]
    ParsingDefString(#[from] DefParseError)
}
