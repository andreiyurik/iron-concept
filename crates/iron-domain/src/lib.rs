//! IRON's pure domain logic. No I/O, no async, no dependencies.
//!
//! Every function here is deterministic and total: the same code runs in the
//! validator, the simulator, the edge agent, and the server
//! (`docs/decisions/0009-rust-for-the-server.md`).

pub mod decode;
pub mod expr;
pub mod publish;
pub mod scaling;
pub mod source;

/// Per-sample trust marker. Part of the value, not metadata
/// (`docs/specs/tag-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    /// A successful read of a plausible value.
    Good,
    /// The value exists but should not be trusted fully.
    Uncertain,
    /// There is no usable value.
    Bad,
}

impl Quality {
    /// The worse of two qualities.
    #[must_use]
    pub fn worst(self, other: Quality) -> Quality {
        self.max(other)
    }
}

/// A tag value: a number or a boolean. There are no other value types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// Every numeric tag type, carried as a 64-bit float.
    Number(f64),
    /// A `bool` tag.
    Bool(bool),
}

/// A value together with its quality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// The value.
    pub value: Value,
    /// Its quality.
    pub quality: Quality,
}

/// The declared `type` of a tag (`docs/specs/tag-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagType {
    /// One bit.
    Bool,
    /// Signed 16-bit integer.
    Int16,
    /// Signed 32-bit integer.
    Int32,
    /// Unsigned 16-bit integer.
    Uint16,
    /// Unsigned 32-bit integer.
    Uint32,
    /// IEEE 754 single precision.
    Float32,
    /// IEEE 754 double precision.
    Float64,
}

impl TagType {
    /// Parses the spelling used in spec files.
    #[must_use]
    pub fn parse(s: &str) -> Option<TagType> {
        Some(match s {
            "bool" => TagType::Bool,
            "int16" => TagType::Int16,
            "int32" => TagType::Int32,
            "uint16" => TagType::Uint16,
            "uint32" => TagType::Uint32,
            "float32" => TagType::Float32,
            "float64" => TagType::Float64,
            _ => return None,
        })
    }

    /// How many 16-bit registers hold one value of this type.
    #[must_use]
    pub fn register_count(self) -> usize {
        match self {
            TagType::Bool | TagType::Int16 | TagType::Uint16 => 1,
            TagType::Int32 | TagType::Uint32 | TagType::Float32 => 2,
            TagType::Float64 => 4,
        }
    }
}
