
//! Utilities for handling and deserializing a Paramdex for modifying Souls games.
//!
//! Entry points for the library include:
//! - [`Paramdex::deserialize_all`] - For deserializing an entire Paramdex
//! - [`deserialize::deserialize_def`] - For deserializing a single Paramdef from a Paramdex
//! - [`Paramdex::empty`] - For starting with an empty Paramdex to insert defs into.


/// Utilities for deserializing [ParamDef]s from XML. Input should be from
/// [soulsmods/Paramdex](https://github.com/soulsmods/Paramdex).
pub mod deserialize;

use std::collections::HashMap;
use crate::deserialize::ParamdefDeserializeError;

/// A simple mapping from param type to a [ParamDef]
pub struct Paramdex {
    /// internal backing map for [ParamDef]s
    definitions: HashMap<String, ParamDef>,
}

impl Paramdex {
    /// Insert a new [ParamDef] into the Paramdex
    pub fn insert(&mut self, paramdef: ParamDef) -> Option<ParamDef> {
        self.definitions.insert(paramdef.param_type.clone(), paramdef)
    }

    /// Retrieve a [ParamDef] based on a param type (encoded into params)
    /// The relevant definition must first be inserted into the paramdex by using [`Paramdex::insert`]
    /// or by deserializing the Paramdex.
    ///
    /// # See also
    /// [`Paramdex::deserialize_all`]
    pub fn get_param_def(&self, key: &str) -> Option<&ParamDef> {
        self.definitions.get(key)
    }

    /// Deserialize a whole Paramdex from an iterator of &str
    pub fn deserialize_all<I: IntoIterator<Item = S>, S: AsRef<str>>(input_iter: I) -> Result<Paramdex, ParamdefDeserializeError> {
        let mut paramdex = Paramdex { definitions: HashMap::new() };

        for input in input_iter {
            let input = input.as_ref();
            paramdex.insert(deserialize::deserialize_def(input)?);
        }
        Ok(paramdex)
    }

    /// Creates an empty Paramdex.
    pub fn empty() -> Paramdex { Paramdex { definitions: HashMap::new() } }
}

/// The text format for descriptions in the [ParamDef]
pub enum ParamdefFormat {
    UTF16,
    ShiftJIS,
}

/// The endianness of the specific [ParamDef]
pub enum ParamdefEndian {
    Little,
    Big,
}

/// A definition for the format of a param file
pub struct ParamDef {
    /// The internal type key for the parameter
    pub param_type: String,

    /// The data version declared for the param
    pub data_version: u32,

    /// The endianness declared for the param
    pub endian: ParamdefEndian,

    /// The string encoding declared for the param
    pub string_format: ParamdefFormat,

    /// The version of the format for the XML
    pub format_version: u32,

    /// The fields present in the param. Ordered.
    pub fields: Vec<ParamField>
}

/// The data type definition for a parameter field
#[derive(Debug)]
pub struct ParamFieldDef {
    pub field_type: ParamFieldType,
    pub name: String,
    pub default_value: Option<f64>,
}

/// Declared metadata about fields in a param
pub struct ParamField {
    /// The definition of the field, including type and internal name, among others.
    pub field_def: ParamFieldDef,

    /// A user-friends display name.
    pub display_name: Option<String>,

    /// A type of enum declared by a paramdex that can be applied to this field. Unused.
    pub enum_tdf: Option<String>,

    /// A  user-friendly description
    pub description: Option<String>,

    /// A printf(3) compatible format string for printing the data in this field. Unused.
    pub printf_format: Option<String>,

    /// Flags that inform a potential editor how to handle this field. Unused.
    pub edit_flags: Option<EditFlags>,

    /// Minimum value allowed to be input in an editor. Unused.
    pub minimum: Option<f64>,

    /// Maximum value allowed to be input in an editor. Unused.
    pub maximum: Option<f64>,

    /// Increment value allowed to be input in an editor. Unused.
    pub increment: Option<f64>,

    /// Declares sorting for a potential editor. Unused.
    pub sort_id: Option<usize>,
}

/// Flags used in editors to control user input behavior
pub struct EditFlags {
    pub wrap: bool,
    pub lock: bool,
}

/// Type of field present in the param
///
/// \[su\]\(8\|16\|32\) are integer types, signed and unsigned respectively, with the
/// appropriate bit sizes.
#[allow(non_camel_case_types)]
#[derive(PartialEq, Debug)]
pub enum ParamFieldType {
    /// Signed integer with size of 8 bits
    s8,

    /// Unsigned integer with size of 8 bits
    u8 {
        /// Optionally limited to number of bits to be read
        bit_size: Option<u8>
    },

    /// Signed integer with size of 16 bits
    s16,

    /// Unsigned integer with size of 16 bits
    u16 {
        /// Optionally limited to number of bits to be read
        bit_size: Option<u8>
    },

    /// Signed integer with size of 32 bits
    s32,

    /// Unsigned integer with size of 32 bits
    u32 {
        /// Optionally limited to number of bits to be read
        bit_size: Option<u8>
    },

    /// Boolean value represented with 32 bits. 0 == `false`, !0 == `true`.
    b32,

    /// Single-precision floating point
    f32,

    /// Single-precision floating point, but this time references an angle. No real difference to [`ParamFieldType::f32`]
    a32,

    /// Double-precision floating point
    f64,

    /// Fixed-length string encoded in ShiftJIS.
    fixstr {
        /// Length of fixed-length string
        length: usize,
    },

    /// Fixed-length string encoded in UTF16.
    fixstrW {
        /// Length of fixed-length string
        length: usize,
    },

    /// Unused or unknown bytes or bits, likely used for padding
    dummy8 {
        /// Length of dummy data. 1 byte if `None`.
        length: Option<DummyType>
    },
}

/// Enum for type of dummy data
#[derive(PartialEq, Debug)]
pub enum DummyType {
    /// Dummy data is in bytes, with a defined length
    Bytes(usize),

    ///  Dummy data is in bits, with a defined length
    Bits(u8),
}

impl ParamFieldType {
    /// Sets the bit size of a field type, on field types that support variable bit lengths.
    ///
    /// # Panics
    /// Panics when the field type does not support bit size definitions. See [`ParamFieldType::supports_bit_size`]
    pub fn set_bit_size(&mut self, new_bit_size: u8) {
        match self {
            Self::u8 {bit_size} | Self::u16 {bit_size} | Self::u32 {bit_size} => {
                bit_size.replace(new_bit_size)
            }
            _ => panic!("Bit size not supported"),
        };
    }

    /// Whether the given field type supports bit size definitions
    pub fn supports_bit_size(&self) -> bool {
        match self {
            Self::u8 {..} | Self::u16 {..} | Self::u32 {..} => true,
            _ => false,
        }
    }
}
