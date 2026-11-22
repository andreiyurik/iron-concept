//! Source URI parsing (`docs/specs/protocol-drivers.md`).

use crate::decode::ByteOrder;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Table {
    Coil,
    Discrete,
    Holding,
    Input,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModbusSource {
    pub host: String,
    pub table: Table,
    pub address: u16,
    pub bit: Option<u8>,
    pub order: Option<ByteOrder>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    Scheme,
    Syntax(String),
    DatasheetNotation { suggested: u16 },
}

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
