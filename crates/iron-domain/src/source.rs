//! Source URI parsing (`docs/specs/protocol-drivers.md`, "Modbus addressing").

use crate::decode::ByteOrder;

/// A Modbus data table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Table {
    /// Coils: read FC01, write FC05.
    Coil,
    /// Discrete inputs: FC02.
    Discrete,
    /// Holding registers: read FC03, write FC06/FC16.
    Holding,
    /// Input registers: FC04.
    Input,
}

/// A parsed `modbus://` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModbusSource {
    /// Connection alias, hostname, or IP address.
    pub host: String,
    /// The data table.
    pub table: Table,
    /// Zero-based protocol address.
    pub address: u16,
    /// Bit of a 16-bit register, for packed `bool` tags.
    pub bit: Option<u8>,
    /// Per-tag byte-order override.
    pub order: Option<ByteOrder>,
}

/// Why a source URI was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// Not a `modbus://` URI.
    Scheme,
    /// Malformed; the message says what was expected.
    Syntax(String),
    /// A datasheet-style address such as `40001` (REQ-DRV-009). `suggested`
    /// is the zero-based protocol address it most likely means.
    DatasheetNotation {
        /// The converted address to offer in the hint.
        suggested: u16,
    },
}

/// Parses `modbus://<host>/<table>/<address>[?bit=<n>][&order=<order>]`.
///
/// Decimal addresses that look like datasheet notation (`4xxxx` on holding
/// registers, `3xxxx` on input registers) are rejected with the converted
/// address; the hexadecimal form is always taken literally.
pub fn parse_modbus(uri: &str) -> Result<ModbusSource, SourceError> {
    let rest = uri.strip_prefix("modbus://").ok_or(SourceError::Scheme)?;
    let (path, query) = match rest.split_once('?') {
        Some((p, q)) => (p, Some(q)),
        None => (rest, None),
    };
    let parts: Vec<&str> = path.split('/').collect();
    let [host, table, address] = parts[..] else {
        return Err(SourceError::Syntax(
            "expected modbus://<host>/<table>/<address>".into(),
        ));
    };
    if host.is_empty() {
        return Err(SourceError::Syntax("missing host".into()));
    }
    let table = match table {
        "coil" => Table::Coil,
        "discrete" => Table::Discrete,
        "holding" => Table::Holding,
        "input" => Table::Input,
        other => {
            return Err(SourceError::Syntax(format!(
                "unknown table '{other}': expected coil, discrete, holding, or input"
            )))
        }
    };
    let address = parse_address(table, address)?;
    let (mut bit, mut order) = (None, None);
    for pair in query.into_iter().flat_map(|q| q.split('&')) {
        match pair.split_once('=') {
            Some(("bit", v)) => {
                let b: u8 = v
                    .parse()
                    .map_err(|_| SourceError::Syntax("bit must be 0–15".into()))?;
                if b > 15 || !matches!(table, Table::Holding | Table::Input) {
                    return Err(SourceError::Syntax(
                        "bit must be 0–15 and applies to holding or input registers".into(),
                    ));
                }
                bit = Some(b);
            }
            Some(("order", v)) => {
                order = Some(ByteOrder::parse(v).ok_or_else(|| {
                    SourceError::Syntax("order must be ABCD, CDAB, BADC, or DCBA".into())
                })?);
            }
            _ => return Err(SourceError::Syntax(format!("unknown parameter '{pair}'"))),
        }
    }
    Ok(ModbusSource {
        host: host.to_string(),
        table,
        address,
        bit,
        order,
    })
}

fn parse_address(table: Table, text: &str) -> Result<u16, SourceError> {
    let bad = || SourceError::Syntax(format!("address '{text}' is not an integer in 0–65535"));
    if let Some(hex) = text.strip_prefix("0x") {
        return u16::from_str_radix(hex, 16).map_err(|_| bad());
    }
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad());
    }
    let n: u32 = text.parse().map_err(|_| bad())?;
    let prefix = match table {
        Table::Holding => Some(4),
        Table::Input => Some(3),
        _ => None,
    };
    if let Some(p) = prefix {
        for base in [p * 10_000, p * 100_000] {
            if text.len() >= 5
                && n > base
                && n <= base + 65_536
                && text.starts_with(char::from(b'0' + p as u8))
            {
                return Err(SourceError::DatasheetNotation {
                    suggested: (n - base - 1) as u16,
                });
            }
        }
    }
    u16::try_from(n).map_err(|_| bad())
}
