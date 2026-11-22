//! Spec files → a small tree with source positions; enforces the YAML subset.

use yaml_edit::{TextPosition, YamlFile, YamlKind, YamlNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub span: Span,
    pub kind: Kind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Map(Vec<Entry>),
    Seq(Vec<Node>),
    Scalar { text: String, quoted: bool },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub key: String,
    pub key_span: Span,
    pub value: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Problem {
    pub span: Span,
    pub path: String,
    pub message: String,
    /// A syntax error, as opposed to a construct outside the subset.
    pub syntax: bool,
}

impl Node {
    #[must_use]
    pub fn entries(&self) -> Option<&[Entry]> {
        match &self.kind {
            Kind::Map(e) => Some(e),
            _ => None,
        }
    }

    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Node> {
        self.entries()?.iter().find(|e| e.key == key).map(|e| &e.value)
    }

    #[must_use]
    pub fn items(&self) -> Option<&[Node]> {
        match &self.kind {
            Kind::Seq(items) => Some(items),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Kind::Scalar { text, .. } => Some(text),
            _ => None,
        }
    }

    fn plain(&self) -> Option<&str> {
        match &self.kind {
            Kind::Scalar { text, quoted: false } => Some(text),
            _ => None,
        }
    }

    /// Decimal or `0x` hexadecimal; a quoted `"1"` is text, not a number.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        let text = self.plain()?;
        if let Some(hex) = text.strip_prefix("0x") {
            let digits = hex.bytes().all(|b| b.is_ascii_hexdigit());
            return u64::from_str_radix(hex, 16).ok().filter(|_| digits).map(|v| v as f64);
        }
        let numeric = text.starts_with(|c: char| c.is_ascii_digit() || c == '-' || c == '.');
        text.parse::<f64>().ok().filter(|v| numeric && v.is_finite())
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match self.plain()? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_ms(&self) -> Option<i64> {
        let text = self.as_str()?;
        let split = text.find(|c: char| !c.is_ascii_digit() && c != '.')?;
        let (number, unit) = text.split_at(split);
        let scale = match unit {
            "ms" => 1.0,
            "s" => 1_000.0,
            "m" => 60_000.0,
            "h" => 3_600_000.0,
            _ => return None,
        };
        let value: f64 = number.parse().ok()?;
        value.is_finite().then_some((value * scale) as i64)
    }

    fn scalars(&self, out: &mut Vec<Span>) {
        match &self.kind {
            Kind::Scalar { .. } => out.push(self.span),
            Kind::Seq(items) => items.iter().for_each(|n| n.scalars(out)),
            Kind::Map(entries) => entries.iter().for_each(|e| e.value.scalars(out)),
        }
    }

    fn path_at(&self, offset: usize, prefix: &str) -> String {
        for e in self.entries().unwrap_or_default() {
            if (e.key_span.start..e.value.span.end.max(e.key_span.end)).contains(&offset) {
                let path = if prefix.is_empty() { e.key.clone() } else { format!("{prefix}.{}", e.key) };
                return e.value.path_at(offset, &path);
            }
        }
        prefix.to_string()
    }
}

/// The root is absent for an empty file and for one with a syntax error.
#[must_use]
pub fn parse(src: &str) -> (Option<Node>, Vec<Problem>) {
    let problem = |span, message: &str, syntax| Problem { span, path: String::new(), message: message.into(), syntax };
    let parsed = YamlFile::parse(src);
    let mut problems: Vec<Problem> =
        parsed.positioned_errors().iter().map(|e| problem(span_of(&e.range), &format!("YAML syntax error: {}", e.message), true)).collect();
    problems.extend(lexical(src));
    if problems.iter().any(|p| p.syntax) {
        return (None, problems);
    }
    let file = parsed.tree();
    if file.documents().count() > 1 {
        problems.push(problem(Span::default(), "multi-document files are outside IRON's YAML subset", false));
        return (None, problems);
    }
    let blank = |l: &str| matches!(l.trim(), "" | "---" | "...") || l.trim_start().starts_with('#');
    let Some(root) = file.document().and_then(|d| d.as_mapping()) else {
        if !src.lines().all(blank) {
            problems.push(problem(Span::default(), "the top level of a spec file must be a mapping", true));
        }
        return (None, problems);
    };
    let node = map(src, &root, span_of(&root.byte_range()), "", &mut problems);
    // The scanner reads lines, not YAML: an indicator strictly inside a scalar
    // (after a comma in plain text, on the next line of a long string) is text.
    let mut scalars = Vec::new();
    node.scalars(&mut scalars);
    problems.retain(|p| !scalars.iter().any(|s| s.start < p.span.start && p.span.start < s.end));
    for p in problems.iter_mut().filter(|p| p.path.is_empty()) {
        p.path = node.path_at(p.span.start, "");
    }
    (Some(node), problems)
}

/// What the tree walk cannot see: indicators that only matter where a node
/// starts, so a `&` or `*` inside a URL or a quoted string is never flagged.
fn lexical(src: &str) -> Vec<Problem> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        let body = line.trim_end_matches(['\n', '\r']);
        let indent = body.len() - body.trim_start_matches([' ', '\t']).len();
        let mut flag = |at: usize, message: &str, syntax| {
            let span = Span { start: offset + at, end: offset + at + 1 };
            out.push(Problem { span, path: String::new(), message: message.into(), syntax });
        };
        if body[..indent].contains('\t') {
            flag(0, "YAML syntax error: tabs cannot be used for indentation", true);
        }
        if body.starts_with('%') {
            flag(0, "directives are outside IRON's YAML subset", false);
        }
        let bytes = body.as_bytes();
        let (mut i, mut start, mut quote) = (indent, true, None::<u8>);
        while i < bytes.len() {
            let c = bytes[i];
            let next_blank = bytes.get(i + 1).is_none_or(|n| *n == b' ');
            if let Some(q) = quote {
                if c == b'\\' && q == b'"' {
                    i += 1;
                } else if c == q {
                    quote = None;
                }
            } else if c == b'#' && (i == indent || bytes[i - 1] == b' ') {
                break;
            } else if start && c != b' ' {
                let block = || {
                    let rest = body[i..].split(" #").next().unwrap_or("").trim_end();
                    rest.len() <= 3 && rest[1..].bytes().all(|b| b"+-0123456789".contains(&b))
                };
                match c {
                    b'&' => flag(i, "anchors are outside IRON's YAML subset", false),
                    b'*' => flag(i, "aliases are outside IRON's YAML subset", false),
                    b'!' => flag(i, "tags are outside IRON's YAML subset", false),
                    b'?' if next_blank => flag(i, "complex keys are outside IRON's YAML subset", false),
                    b'|' | b'>' if block() => {
                        flag(i, "block scalars are outside IRON's YAML subset", false);
                    }
                    _ => {}
                }
                start = matches!(c, b'[' | b'{') || (c == b'-' && next_blank);
                if matches!(c, b'"' | b'\'') {
                    quote = Some(c);
                }
            } else if matches!(c, b'[' | b'{' | b',') || (c == b':' && next_blank) {
                start = true;
            }
            i += 1;
        }
        offset += line.len();
    }
    out
}

fn span_of(r: &TextPosition) -> Span {
    Span { start: r.start as usize, end: r.end as usize }
}

fn violation(problems: &mut Vec<Problem>, span: Span, path: &str, message: &str) {
    problems.push(Problem { span, path: path.into(), message: message.into(), syntax: false });
}

fn one_line(src: &str, flow: bool, span: Span, path: &str, problems: &mut Vec<Problem>) {
    if flow && src.get(span.start..span.end).is_some_and(|t| t.contains('\n')) {
        violation(problems, span, path, "a flow collection must fit on one line");
    }
}

fn map(src: &str, m: &yaml_edit::Mapping, span: Span, path: &str, problems: &mut Vec<Problem>) -> Node {
    one_line(src, m.is_flow_style(), span, path, problems);
    let mut entries: Vec<Entry> = Vec::new();
    for (k, v) in m.iter() {
        let key_span = span_of(&k.byte_range());
        let Some(key) = k.as_scalar().map(yaml_edit::Scalar::as_string) else {
            violation(problems, key_span, path, "a key must be a plain scalar");
            continue;
        };
        let child = if path.is_empty() { key.clone() } else { format!("{path}.{key}") };
        if key == "<<" {
            violation(problems, key_span, path, "merge keys are outside IRON's YAML subset");
        } else if entries.iter().any(|e| e.key == key) {
            violation(problems, key_span, &child, "duplicate key");
        } else {
            let value = node(src, &v, &child, problems);
            entries.push(Entry { key, key_span, value });
        }
    }
    Node { span, kind: Kind::Map(entries) }
}

fn node(src: &str, n: &YamlNode, path: &str, problems: &mut Vec<Problem>) -> Node {
    let span = span_of(&n.byte_range());
    let kind = match (n.kind(), n.as_mapping(), n.as_sequence(), n.as_scalar()) {
        (YamlKind::Mapping, Some(m), ..) => return map(src, m, span, path, problems),
        (YamlKind::Sequence, _, Some(seq), _) => {
            one_line(src, seq.is_flow_style(), span, path, problems);
            let item = |(i, item): (usize, YamlNode)| node(src, &item, &format!("{path}[{i}]"), problems);
            Kind::Seq(seq.values().enumerate().map(item).collect())
        }
        (YamlKind::Scalar, .., Some(s)) => Kind::Scalar { text: s.as_string(), quoted: s.is_quoted() },
        // Aliases and tags are reported by `lexical`; quoted so the
        // placeholder never reads as a number or a bool.
        _ => Kind::Scalar { text: String::new(), quoted: true },
    };
    Node { span, kind }
}

/// 1-based line and column, the column counted in characters.
#[must_use]
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let mut end = offset.min(src.len());
    while !src.is_char_boundary(end) {
        end -= 1;
    }
    let before = &src[..end];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (before.matches('\n').count() + 1, before[line_start..].chars().count() + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_with_positions() {
        let src = "a:\n  x: { limit: 21.5, on: true }  # c\n  r: [0, 0x10]\n  s: \"1\"\n  é: 5s\n";
        let (root, problems) = parse(src);
        assert_eq!(problems, vec![]);
        let root = root.unwrap();
        let a = root.get("a").unwrap();
        assert_eq!(a.get("x").unwrap().get("limit").unwrap().as_f64(), Some(21.5));
        assert_eq!(a.get("x").unwrap().get("on").unwrap().as_bool(), Some(true));
        let r = a.get("r").unwrap().items().unwrap();
        assert_eq!((r[0].as_f64(), r[1].as_f64()), (Some(0.0), Some(16.0)));
        assert_eq!(a.get("s").unwrap().as_f64(), None, "a quoted number is text");
        assert_eq!(a.get("é").unwrap().as_ms(), Some(5000));
        assert_eq!(line_col(src, a.get("r").unwrap().span.start), (3, 6));
        assert_eq!(line_col(src, a.get("é").unwrap().span.start), (5, 6), "columns count characters");
    }

    #[test]
    fn comment_only_and_headed_files_are_fine() {
        for src in ["", "# only\n# comments\n", "\n# c\n\n"] {
            assert_eq!(parse(src), (None, vec![]), "{src:?}");
        }
        let (root, problems) = parse("# header\n---\na:\n  x: 1\n");
        assert_eq!(problems, vec![]);
        assert!(root.unwrap().get("a").is_some());
    }

    #[test]
    fn syntax_errors_carry_their_position() {
        let src = "z:\n  t:\n    unit: [a, b\n";
        let (root, problems) = parse(src);
        assert!(root.is_none());
        assert!(problems.iter().all(|p| p.syntax) && line_col(src, problems[0].span.start).0 > 1, "{problems:?}");
        let (_, tabs) = parse("z:\n\tt: 1\n");
        assert!(tabs.iter().any(|p| p.syntax && p.message.contains("tabs")), "{tabs:?}");
    }

    // REQ-CFG-005
    #[test]
    fn rejects_constructs_outside_the_subset() {
        let cases = [
            ("a:\n  x: &b { t: 1 }\n  y: 2\n", "anchors", "a.x"),
            ("a:\n  r: [&b 1, 2]\n", "anchors", "a.r"),
            ("a:\n  &b x: 1\n", "anchors", "a"),
            ("a:\n  x: # c\n    &b\n    t: 1\n", "anchors", "a.x"),
            ("a:\n  x: { t: 1 }\n  y: *b\n", "aliases", "a.y"),
            ("a:\n  y: { <<: 1 }\n", "merge keys", "a.y"),
            ("a:\n  x: !!str 1\n", "tags", "a.x"),
            ("a:\n  x: 1\n  x: 2\n", "duplicate key", "a.x"),
            ("a: 1\n---\nb: 2\n", "multi-document", ""),
            ("a:\n  x: { t: 1,\n       u: 2 }\n", "one line", "a.x"),
            ("a:\n  d: |\n    multi\n    line\n", "block scalars", "a.d"),
            ("a:\n  d: >-\n    folded\n", "block scalars", "a.d"),
            ("? a\n: 1\n", "complex keys", ""),
        ];
        for (src, needle, path) in cases {
            let (_, problems) = parse(src);
            let hit = problems.iter().find(|p| p.message.contains(needle));
            assert_eq!(hit.map(|p| p.path.as_str()), Some(path), "{src:?}: {problems:?}");
        }
        let ok = "a:\n  u: \"modbus://h/x?a=1&b=2\"\n  m: \"*not an alias*\"\n  n: a > b\n  k: 5 # &c\n  d: hello, !world\n  e: see [1], *note\n  f: a {b}, &c\n  g: \"Tank high\n    & check *now*\"\n  h: Tank high\n    *now* & check\n";
        assert_eq!(parse(ok).1, vec![]);
    }
}
