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
        return Err(SourceError::Syntax("expected modbus://<host>/<table>/<address>".into()));
    };
    if host.is_empty() {
        return Err(SourceError::Syntax("missing host".into()));
    }
    let table = match table {
        "coil" => Table::Coil,
        "discrete" => Table::Discrete,
        "holding" => Table::Holding,
        "input" => Table::Input,
        other => return Err(SourceError::Syntax(format!("unknown table '{other}': expected coil, discrete, holding, or input"))),
    };
    let address = parse_address(table, address)?;
    let (mut bit, mut order) = (None, None);
    for pair in query.into_iter().flat_map(|q| q.split('&')) {
        let syntax = |m: &str| SourceError::Syntax(m.into());
        match pair.split_once('=') {
            Some(("bit", v)) if bit.is_none() => {
                let b = v.parse::<u8>().ok().filter(|b| *b <= 15 && v.bytes().all(|c| c.is_ascii_digit()));
                if !matches!(table, Table::Holding | Table::Input) {
                    return Err(syntax("bit applies to holding or input registers"));
                }
                bit = Some(b.ok_or_else(|| syntax("bit must be 0–15"))?);
            }
            Some(("order", v)) if order.is_none() => {
                order = Some(ByteOrder::parse(v).ok_or_else(|| SourceError::Syntax(format!("order must be {}", ByteOrder::NAMES)))?);
            }
            _ => return Err(SourceError::Syntax(format!("unknown or repeated parameter '{pair}'"))),
        }
    }
    Ok(ModbusSource { host: host.to_string(), table, address, bit, order })
}

fn parse_address(table: Table, text: &str) -> Result<u16, SourceError> {
    let bad = || SourceError::Syntax(format!("address '{text}' is not an integer in 0–65535"));
    let digits = |t: &str| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit());
    if let Some(hex) = text.strip_prefix("0x") {
        let valid = hex.bytes().all(|b| b.is_ascii_hexdigit());
        return u16::from_str_radix(hex, 16).ok().filter(|_| valid).ok_or_else(bad);
    }
    // Modicon notation written with an x: 4x0001 (REQ-DRV-009).
    if let Some((_, n)) = text.split_once('x').filter(|(p, n)| matches!(*p, "3" | "4") && digits(n)) {
        let suggested = n.parse::<u32>().ok().and_then(|n| u16::try_from(n.checked_sub(1)?).ok());
        return suggested.map_or_else(|| Err(bad()), |suggested| Err(SourceError::DatasheetNotation { suggested }));
    }
    if !digits(text) {
        return Err(bad());
    }
    let n: u32 = text.parse().map_err(|_| bad())?;
    let prefix = match table {
        Table::Holding => Some(4),
        Table::Input => Some(3),
        _ => None,
    };
    // Modicon datasheet notation: 4xxxx/4xxxxx holding, 3xxxx/3xxxxx input (REQ-DRV-009).
    if let Some(p) = prefix {
        for base in [p * 10_000, p * 100_000] {
            if n > base && n <= base + 65_536 && text.starts_with(char::from(b'0' + p as u8)) {
                return Err(SourceError::DatasheetNotation { suggested: (n - base - 1) as u16 });
            }
        }
    }
    u16::try_from(n).map_err(|_| bad())
}
