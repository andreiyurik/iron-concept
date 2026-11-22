//! Register decoding (`docs/specs/protocol-drivers.md`).

use crate::{TagType, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ByteOrder {
    #[default]
    Abcd,
    Cdab,
    Badc,
    Dcba,
}

impl ByteOrder {
    #[must_use]
    pub fn parse(s: &str) -> Option<ByteOrder> {
        Some(match s {
            "ABCD" => ByteOrder::Abcd,
            "CDAB" => ByteOrder::Cdab,
            "BADC" => ByteOrder::Badc,
            "DCBA" => ByteOrder::Dcba,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    RegisterCount { expected: usize, got: usize },
    Bit,
}

pub fn decode(
    ty: TagType,
    order: ByteOrder,
    registers: &[u16],
    bit: Option<u8>,
) -> Result<Value, DecodeError> {
    let expected = ty.register_count();
    if registers.len() != expected {
        return Err(DecodeError::RegisterCount {
            expected,
            got: registers.len(),
        });
    }
    if ty == TagType::Bool {
        return match bit {
            Some(b) if b > 15 => Err(DecodeError::Bit),
            Some(b) => Ok(Value::Bool(registers[0] >> b & 1 == 1)),
            None => Ok(Value::Bool(registers[0] != 0)),
        };
    }
    if bit.is_some() {
        return Err(DecodeError::Bit);
    }
    // Normalize to big-endian bytes.
    let mut words: Vec<u16> = registers.to_vec();
    if matches!(order, ByteOrder::Cdab | ByteOrder::Dcba) {
        words.reverse();
    }
    if matches!(order, ByteOrder::Badc | ByteOrder::Dcba) {
        for w in &mut words {
            *w = w.swap_bytes();
        }
    }
    let mut bytes = [0u8; 8];
    for (i, w) in words.iter().enumerate() {
        bytes[i * 2..i * 2 + 2].copy_from_slice(&w.to_be_bytes());
    }
    let b2 = [bytes[0], bytes[1]];
    let b4 = [bytes[0], bytes[1], bytes[2], bytes[3]];
    Ok(Value::Number(match ty {
        TagType::Int16 => f64::from(i16::from_be_bytes(b2)),
        TagType::Uint16 => f64::from(u16::from_be_bytes(b2)),
        TagType::Int32 => f64::from(i32::from_be_bytes(b4)),
        TagType::Uint32 => f64::from(u32::from_be_bytes(b4)),
        TagType::Float32 => f64::from(f32::from_be_bytes(b4)),
        TagType::Float64 => f64::from_be_bytes(bytes),
        TagType::Bool => unreachable!("handled above"),
    }))
}
