//! IRON's pure domain logic. No I/O, no async, no dependencies.

pub mod decode;
pub mod expr;
pub mod publish;
pub mod scaling;
pub mod source;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Quality {
    Good,
    Uncertain,
    Bad,
}

impl Quality {
    #[must_use]
    pub fn worst(self, other: Quality) -> Quality {
        self.max(other)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    Number(f64),
    Bool(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    pub value: Value,
    pub quality: Quality,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TagType {
    Bool,
    Int16,
    Int32,
    Uint16,
    Uint32,
    Float32,
    Float64,
}

impl TagType {
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

    #[must_use]
    pub fn register_count(self) -> usize {
        match self {
            TagType::Bool | TagType::Int16 | TagType::Uint16 => 1,
            TagType::Int32 | TagType::Uint32 | TagType::Float32 => 2,
            TagType::Float64 => 4,
        }
    }
}
