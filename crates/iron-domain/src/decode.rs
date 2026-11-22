//! Register decoding (`docs/specs/protocol-drivers.md`, "Modbus addressing").

use crate::{TagType, Value};

/// Position of a value's bytes on the wire. `ABCD` is big-endian.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ByteOrder {
    /// Big-endian words, big-endian bytes — the Modbus convention.
    #[default]
    Abcd,
    /// Word order reversed.
    Cdab,
    /// Bytes swapped inside each word.
    Badc,
    /// Little-endian: both reversed.
    Dcba,
}

impl ByteOrder {
    /// Parses `ABCD`, `CDAB`, `BADC`, `DCBA`.
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

/// Why a set of registers could not be decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The number of registers does not match the type.
    RegisterCount {
        /// Registers the type needs.
        expected: usize,
        /// Registers supplied.
        got: usize,
    },
    /// A bit index outside 0–15, or a bit given for a non-`bool` type.
    Bit,
}

/// Decodes registers, given in address order, into a value.
///
/// For `bool`, `bit` selects one bit of the register; without it, any
/// non-zero register is `true`. Decoding follows `order` (REQ-DRV-010); for
/// 64-bit values the same four orders apply to the four words.
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
