pub mod deserialize;

use std::collections::HashMap;

pub struct Paramdex {
    definitions: HashMap<String, ParamDef>,
}

impl Paramdex {
    pub fn insert(&mut self, paramdef: ParamDef) -> Option<ParamDef> {
        self.definitions.insert(paramdef.param_type.clone(), paramdef)
    }

    pub fn get_param_def(&self, key: &String) -> Option<&ParamDef> {
        self.definitions.get(key)
    }
}

pub enum ParamdefFormat {
    UTF16,
    ShiftJIS,
}

pub enum ParamdefEndian {
    Little,
    Big,
}

pub struct ParamDef {
    pub param_type: String,
    pub data_version: u32,
    pub endian: ParamdefEndian,
    pub string_format: ParamdefFormat,
    pub format_version: u32,
    pub fields: Vec<ParamField>
}

#[derive(Debug)]
pub struct ParamFieldDef {
    pub field_type: ParamFieldType,
    pub name: String,
    pub default_value: Option<f64>,
}

pub struct ParamField {
    pub field_def: ParamFieldDef,
    pub display_name: Option<String>,
    pub enum_tdf: Option<String>,
    pub description: Option<String>,
    pub printf_format: Option<String>,
    pub edit_flags: Option<EditFlags>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub increment: Option<f64>,
    pub sort_id: Option<usize>,
}

pub struct EditFlags {
    pub wrap: bool,
    pub lock: bool,
}

#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug)]
pub enum ParamFieldType {
    s8,
    u8 {
        bit_size: Option<u8>
    },
    s16,
    u16 {
        bit_size: Option<u8>
    },
    s32,
    u32 {
        bit_size: Option<u8>
    },
    b32,
    f32,
    a32,
    f64,
    fixstr {
        length: usize,
    },
    fixstrW {
        length: usize,
    },
    dummy8 {
        length: Option<usize>,
        bit_size: Option<u8>,
    },
}

impl ParamFieldType {
    pub fn set_bit_size(&mut self, new_bit_size: u8) {
        match self {
            Self::u8 {bit_size} | Self::u16 {bit_size} | Self::u32 {bit_size} => {
                bit_size.replace(new_bit_size)
            }
            _ => panic!("Bit size not supported"),
        };
    }

    pub fn supports_bit_size(&self) -> bool {
        match self {
            Self::u8 {..} | Self::u16 {..} | Self::u32 {..} => true,
            _ => false,
        }
    }
}
