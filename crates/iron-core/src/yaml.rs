//! Spec files → a small tree with source positions; enforces the YAML subset.

use std::str::FromStr;
use yaml_edit::{Document, YamlFile, YamlKind, YamlNode};

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
        self.entries()?
            .iter()
            .find(|e| e.key == key)
            .map(|e| &e.value)
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match &self.kind {
            Kind::Scalar { text, .. } => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        let Kind::Scalar {
            text,
            quoted: false,
        } = &self.kind
        else {
            return None;
        };
        if let Some(hex) = text.strip_prefix("0x") {
            return u64::from_str_radix(hex, 16).ok().map(|v| v as f64);
        }
        let numeric = text
            .bytes()
            .next()
            .is_some_and(|b| b.is_ascii_digit() || b == b'-' || b == b'.');
        text.parse::<f64>()
            .ok()
            .filter(|v| numeric && v.is_finite())
    }

    #[must_use]
    pub fn as_bool(&self) -> Option<bool> {
        match &self.kind {
            Kind::Scalar {
                text,
                quoted: false,
            } if text == "true" => Some(true),
            Kind::Scalar {
                text,
                quoted: false,
            } if text == "false" => Some(false),
            _ => None,
        }
    }

    #[must_use]
    pub fn items(&self) -> Option<&[Node]> {
        match &self.kind {
            Kind::Seq(items) => Some(items),
            _ => None,
        }
    }
}

#[must_use]
pub fn parse(src: &str) -> (Option<Node>, Vec<Problem>) {
    let mut problems = Vec::new();
    let file_problem = |message: &str, syntax| Problem {
        span: Span::default(),
        path: String::new(),
        message: message.to_string(),
        syntax,
    };
    if let Ok(file) = YamlFile::from_str(src) {
        if file.documents().count() > 1 {
            problems.push(file_problem(
                "multi-document files are outside IRON's YAML subset",
                false,
            ));
            return (None, problems);
        }
    }
    let doc = match Document::from_str(src) {
        Ok(doc) => doc,
        Err(e) => {
            problems.push(file_problem(&format!("YAML syntax error: {e}"), true));
            return (None, problems);
        }
    };
    let Some(root) = doc.as_mapping() else {
        if src
            .lines()
            .all(|l| l.trim().is_empty() || l.trim_start().starts_with('#'))
        {
            return (None, problems);
        }
        problems.push(file_problem(
            "the top level of a spec file must be a mapping",
            true,
        ));
        return (None, problems);
    };
    let span = span_of(&root.byte_range());
    let node = map(src, &root, span, "", &mut problems);
    (Some(node), problems)
}

fn span_of(r: &yaml_edit::TextPosition) -> Span {
    Span {
        start: r.start as usize,
        end: r.end as usize,
    }
}

fn map(
    src: &str,
    m: &yaml_edit::Mapping,
    span: Span,
    path: &str,
    problems: &mut Vec<Problem>,
) -> Node {
    let mut entries: Vec<Entry> = Vec::new();
    if m.is_flow_style()
        && src
            .get(span.start..span.end)
            .is_some_and(|t| t.contains('\n'))
    {
        violation(problems, span, path, "a flow mapping must fit on one line");
    }
    for (k, v) in m.iter() {
        let key_span = span_of(&k.byte_range());
        let key = match k.as_scalar() {
            Some(s) => s.as_string(),
            None => {
                violation(problems, key_span, path, "a key must be a plain scalar");
                continue;
            }
        };
        let child = if path.is_empty() {
            key.clone()
        } else {
            format!("{path}.{key}")
        };
        if key == "<<" {
            violation(
                problems,
                key_span,
                path,
                "merge keys are outside IRON's YAML subset",
            );
            continue;
        }
        if entries.iter().any(|e| e.key == key) {
            violation(problems, key_span, &child, "duplicate key");
            continue;
        }
        let value_span = span_of(&v.byte_range());
        // An anchor sits between the colon and the value it names.
        let before = src.get(key_span.end..value_span.end).unwrap_or("");
        if before.trim_start_matches([':', ' ', '\t']).starts_with('&') {
            violation(
                problems,
                key_span,
                &child,
                "anchors are outside IRON's YAML subset",
            );
        }
        let value = node(src, &v, value_span, &child, problems);
        entries.push(Entry {
            key,
            key_span,
            value,
        });
    }
    Node {
        span,
        kind: Kind::Map(entries),
    }
}

fn violation(problems: &mut Vec<Problem>, span: Span, path: &str, message: &str) {
    problems.push(Problem {
        span,
        path: path.to_string(),
        message: message.to_string(),
        syntax: false,
    });
}

fn node(src: &str, n: &YamlNode, span: Span, path: &str, problems: &mut Vec<Problem>) -> Node {
    match n.kind() {
        YamlKind::Mapping => match n.as_mapping() {
            Some(m) => map(src, m, span, path, problems),
            None => Node {
                span,
                kind: Kind::Map(Vec::new()),
            },
        },
        YamlKind::Sequence => {
            let mut items = Vec::new();
            if let Some(seq) = n.as_sequence() {
                if seq.is_flow_style()
                    && src
                        .get(span.start..span.end)
                        .is_some_and(|t| t.contains('\n'))
                {
                    violation(problems, span, path, "a flow sequence must fit on one line");
                }
                for (i, item) in seq.values().enumerate() {
                    let s = span_of(&item.byte_range());
                    items.push(node(src, &item, s, &format!("{path}[{i}]"), problems));
                }
            }
            Node {
                span,
                kind: Kind::Seq(items),
            }
        }
        YamlKind::Scalar => {
            let (text, quoted) = match n.as_scalar() {
                Some(s) => (s.as_string(), s.is_quoted()),
                None => (String::new(), false),
            };
            Node {
                span,
                kind: Kind::Scalar { text, quoted },
            }
        }
        YamlKind::Alias => {
            violation(
                problems,
                span,
                path,
                "aliases are outside IRON's YAML subset",
            );
            Node {
                span,
                kind: Kind::Scalar {
                    text: String::new(),
                    quoted: true,
                },
            }
        }
        _ => {
            violation(problems, span, path, "tags are outside IRON's YAML subset");
            Node {
                span,
                kind: Kind::Scalar {
                    text: String::new(),
                    quoted: true,
                },
            }
        }
    }
}

#[must_use]
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let before = &src[..offset.min(src.len())];
    let line = before.bytes().filter(|b| *b == b'\n').count() + 1;
    let col = before
        .rfind('\n')
        .map_or(before.len(), |i| before.len() - i - 1)
        + 1;
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_values_with_positions() {
        let src = "a:\n  x: { limit: 21.5, on: true }  # c\n  r: [0, 0x10]\n  s: \"1\"\n";
        let (root, problems) = parse(src);
        assert_eq!(problems, vec![]);
        let a = root.unwrap();
        let a = a.get("a").unwrap();
        assert_eq!(
            a.get("x").unwrap().get("limit").unwrap().as_f64(),
            Some(21.5)
        );
        assert_eq!(a.get("x").unwrap().get("on").unwrap().as_bool(), Some(true));
        let r = a.get("r").unwrap().items().unwrap();
        assert_eq!((r[0].as_f64(), r[1].as_f64()), (Some(0.0), Some(16.0)));
        assert_eq!(
            a.get("s").unwrap().as_f64(),
            None,
            "a quoted number is text"
        );
        assert_eq!(line_col(src, a.get("r").unwrap().span.start), (3, 6));
    }

    // REQ-CFG-005.
    #[test]
    fn rejects_constructs_outside_the_subset() {
        let cases = [
            ("a:\n  x: &b { t: 1 }\n  y: 2\n", "anchors"),
            ("a:\n  x: { t: 1 }\n  y: *b\n", "aliases"),
            ("a:\n  y: { <<: 1 }\n", "merge keys"),
            ("a:\n  x: 1\n  x: 2\n", "duplicate key"),
            ("a: 1\n---\nb: 2\n", "multi-document"),
            ("a:\n  x: { t: 1,\n       u: 2 }\n", "one line"),
        ];
        for (src, needle) in cases {
            let (_, problems) = parse(src);
            assert!(
                problems.iter().any(|p| p.message.contains(needle)),
                "{src:?}: expected '{needle}', got {problems:?}"
            );
        }
    }
}
