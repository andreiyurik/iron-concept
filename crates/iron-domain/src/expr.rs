//! The one expression language (`docs/specs/expressions.md`).

use crate::{Quality, Sample, Value};
use std::fmt;

pub const MAX_DEPTH: usize = 64;
/// Bounds the height of an operator chain, which nesting depth does not.
pub const MAX_TOKENS: usize = 512;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num { value: f64, int: bool },
    Bool(bool),
    Ref(String),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(Func, Vec<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Abs,
    Min,
    Max,
    Clamp,
    Round,
    Floor,
    Ceil,
    Sqrt,
    Pow,
    Exp,
    Ln,
    Log10,
    If,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Type {
    Number,
    Bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    TooDeep,
    TooLong,
    UnknownReference,
    TypeMismatch,
    IfBranches,
    UnknownFunction,
    Arity,
    NoValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprError {
    pub kind: ErrorKind,
    pub pos: usize,
    pub message: String,
}

impl fmt::Display for ExprError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (at {})", self.message, self.pos)
    }
}

impl std::error::Error for ExprError {}

fn err<T>(kind: ErrorKind, pos: usize, message: impl Into<String>) -> Result<T, ExprError> {
    Err(ExprError { kind, pos, message: message.into() })
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64, bool),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
}

const OPS: [&str; 15] = ["||", "&&", "==", "!=", "<=", ">=", "<", ">", "+", "-", "*", "/", "%", "!", "="];

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, ExprError> {
    let b = src.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() {
            if c == b'0' && b.get(i + 1) == Some(&b'x') {
                i += 2;
                let h = i;
                while i < b.len() && b[i].is_ascii_hexdigit() {
                    i += 1;
                }
                match u64::from_str_radix(&src[h..i], 16) {
                    Ok(v) if i > h => out.push((Tok::Num(v as f64, true), start)),
                    _ => return err(ErrorKind::Syntax, start, "malformed hexadecimal number"),
                }
                continue;
            }
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            let mut int = true;
            if b.get(i) == Some(&b'.') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
                int = false;
                i += 1;
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            match src[start..i].parse::<f64>() {
                Ok(v) if v.is_finite() => out.push((Tok::Num(v, int), start)),
                _ => return err(ErrorKind::Syntax, start, "malformed number"),
            }
            continue;
        }
        if c.is_ascii_alphabetic() || c == b'_' {
            loop {
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let next_is_name = b.get(i + 1).is_some_and(|n| n.is_ascii_alphabetic() || *n == b'_');
                if b.get(i) == Some(&b'.') && next_is_name {
                    i += 1;
                } else {
                    break;
                }
            }
            out.push((Tok::Ident(src[start..i].to_string()), start));
            continue;
        }
        match c {
            b'(' => out.push((Tok::LParen, start)),
            b')' => out.push((Tok::RParen, start)),
            b',' => out.push((Tok::Comma, start)),
            _ => match OPS.iter().find(|op| src[i..].starts_with(**op)) {
                Some(&"=") | None => {
                    return err(ErrorKind::Syntax, start, "unexpected character");
                }
                Some(op) => {
                    out.push((Tok::Op(op), start));
                    i += op.len();
                    continue;
                }
            },
        }
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    at: usize,
    depth: usize,
    end: usize,
}

impl Parser {
    fn pos(&self) -> usize {
        self.toks.get(self.at).map_or(self.end, |t| t.1)
    }

    fn peek_op(&self) -> Option<&'static str> {
        match self.toks.get(self.at) {
            Some((Tok::Op(op), _)) => Some(op),
            _ => None,
        }
    }

    fn eat_op(&mut self, ops: &[&'static str]) -> Option<&'static str> {
        let op = self.peek_op().filter(|op| ops.contains(op))?;
        self.at += 1;
        Some(op)
    }

    fn expr(&mut self) -> Result<Expr, ExprError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH + 1 {
            return err(ErrorKind::TooDeep, self.pos(), "expression is nested too deeply");
        }
        let mut left = self.and()?;
        while self.eat_op(&["||"]).is_some() {
            left = Expr::Binary(BinaryOp::Or, Box::new(left), Box::new(self.and()?));
        }
        self.depth -= 1;
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.cmp()?;
        while self.eat_op(&["&&"]).is_some() {
            left = Expr::Binary(BinaryOp::And, Box::new(left), Box::new(self.cmp()?));
        }
        Ok(left)
    }

    fn cmp(&mut self) -> Result<Expr, ExprError> {
        const CMP: [&str; 6] = ["==", "!=", "<=", ">=", "<", ">"];
        let left = self.sum()?;
        let Some(op) = self.eat_op(&CMP) else {
            return Ok(left);
        };
        let right = self.sum()?;
        if self.peek_op().is_some_and(|o| CMP.contains(&o)) {
            return err(ErrorKind::Syntax, self.pos(), "comparisons cannot be chained");
        }
        let op = match op {
            "==" => BinaryOp::Eq,
            "!=" => BinaryOp::Ne,
            "<=" => BinaryOp::Le,
            ">=" => BinaryOp::Ge,
            "<" => BinaryOp::Lt,
            _ => BinaryOp::Gt,
        };
        Ok(Expr::Binary(op, Box::new(left), Box::new(right)))
    }

    fn sum(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.term()?;
        while let Some(op) = self.eat_op(&["+", "-"]) {
            let op = if op == "+" { BinaryOp::Add } else { BinaryOp::Sub };
            left = Expr::Binary(op, Box::new(left), Box::new(self.term()?));
        }
        Ok(left)
    }

    fn term(&mut self) -> Result<Expr, ExprError> {
        let mut left = self.unary()?;
        while let Some(op) = self.eat_op(&["*", "/", "%"]) {
            let op = match op {
                "*" => BinaryOp::Mul,
                "/" => BinaryOp::Div,
                _ => BinaryOp::Rem,
            };
            left = Expr::Binary(op, Box::new(left), Box::new(self.unary()?));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, ExprError> {
        match self.eat_op(&["-", "!"]) {
            Some("-") => Ok(Expr::Unary(UnaryOp::Neg, Box::new(self.primary()?))),
            Some(_) => Ok(Expr::Unary(UnaryOp::Not, Box::new(self.primary()?))),
            None => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, ExprError> {
        let pos = self.pos();
        let Some((tok, _)) = self.toks.get(self.at).cloned() else {
            return err(ErrorKind::Syntax, pos, "unexpected end of expression");
        };
        self.at += 1;
        match tok {
            Tok::Num(value, int) => Ok(Expr::Num { value, int }),
            Tok::LParen => {
                let inner = self.expr()?;
                self.close(pos)?;
                Ok(inner)
            }
            Tok::Ident(name) => {
                if !matches!(self.toks.get(self.at), Some((Tok::LParen, _))) {
                    return Ok(match name.as_str() {
                        "true" => Expr::Bool(true),
                        "false" => Expr::Bool(false),
                        _ => Expr::Ref(name),
                    });
                }
                self.at += 1;
                let func = match name.as_str() {
                    "abs" => Func::Abs,
                    "min" => Func::Min,
                    "max" => Func::Max,
                    "clamp" => Func::Clamp,
                    "round" => Func::Round,
                    "floor" => Func::Floor,
                    "ceil" => Func::Ceil,
                    "sqrt" => Func::Sqrt,
                    "pow" => Func::Pow,
                    "exp" => Func::Exp,
                    "ln" => Func::Ln,
                    "log10" => Func::Log10,
                    "if" => Func::If,
                    _ => return err(ErrorKind::UnknownFunction, pos, format!("unknown function '{name}'")),
                };
                let mut args = vec![self.expr()?];
                while matches!(self.toks.get(self.at), Some((Tok::Comma, _))) {
                    self.at += 1;
                    args.push(self.expr()?);
                }
                self.close(pos)?;
                let ok = match func {
                    Func::Min | Func::Max => !args.is_empty(),
                    Func::Clamp | Func::If => args.len() == 3,
                    Func::Pow => args.len() == 2,
                    _ => args.len() == 1,
                };
                if !ok {
                    return err(ErrorKind::Arity, pos, format!("wrong number of arguments for '{name}'"));
                }
                Ok(Expr::Call(func, args))
            }
            _ => err(ErrorKind::Syntax, pos, "expected a value"),
        }
    }

    fn close(&mut self, opened: usize) -> Result<(), ExprError> {
        if matches!(self.toks.get(self.at), Some((Tok::RParen, _))) {
            self.at += 1;
            Ok(())
        } else {
            err(ErrorKind::Syntax, opened, "missing closing parenthesis")
        }
    }
}

pub fn parse(src: &str) -> Result<Expr, ExprError> {
    let toks = lex(src)?;
    if toks.len() > MAX_TOKENS {
        return err(ErrorKind::TooLong, toks[MAX_TOKENS].1, "expression is too long");
    }
    let mut p = Parser { toks, at: 0, depth: 0, end: src.len() };
    let expr = p.expr()?;
    if p.at != p.toks.len() {
        return err(ErrorKind::Syntax, p.pos(), "unexpected token");
    }
    Ok(expr)
}

impl Expr {
    #[must_use]
    pub fn refs(&self) -> Vec<&str> {
        fn walk<'a>(e: &'a Expr, out: &mut Vec<&'a str>) {
            match e {
                Expr::Ref(name) if !out.contains(&name.as_str()) => out.push(name),
                Expr::Unary(_, a) => walk(a, out),
                Expr::Binary(_, a, b) => {
                    walk(a, out);
                    walk(b, out);
                }
                Expr::Call(_, args) => args.iter().for_each(|a| walk(a, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        walk(self, &mut out);
        out
    }

    pub fn check(&self, types: &dyn Fn(&str) -> Option<Type>) -> Result<Type, ExprError> {
        use BinaryOp::*;
        let want = |e: &Expr, t: Type| -> Result<(), ExprError> {
            if e.check(types)? == t {
                Ok(())
            } else {
                let name = if t == Type::Number { "a number" } else { "a boolean" };
                err(ErrorKind::TypeMismatch, 0, format!("expected {name}"))
            }
        };
        match self {
            Expr::Num { .. } => Ok(Type::Number),
            Expr::Bool(_) => Ok(Type::Bool),
            Expr::Ref(name) => types(name).ok_or_else(|| ExprError {
                kind: ErrorKind::UnknownReference,
                pos: 0,
                message: format!("unknown reference '{name}'"),
            }),
            Expr::Unary(UnaryOp::Neg, a) => want(a, Type::Number).map(|()| Type::Number),
            Expr::Unary(UnaryOp::Not, a) => want(a, Type::Bool).map(|()| Type::Bool),
            Expr::Binary(op, a, b) => match op {
                Add | Sub | Mul | Div | Rem => {
                    want(a, Type::Number)?;
                    want(b, Type::Number).map(|()| Type::Number)
                }
                Lt | Le | Gt | Ge => {
                    want(a, Type::Number)?;
                    want(b, Type::Number).map(|()| Type::Bool)
                }
                And | Or => {
                    want(a, Type::Bool)?;
                    want(b, Type::Bool).map(|()| Type::Bool)
                }
                Eq | Ne => {
                    let t = a.check(types)?;
                    want(b, t).map(|()| Type::Bool)
                }
            },
            Expr::Call(Func::If, args) => {
                want(&args[0], Type::Bool)?;
                let t = args[1].check(types)?;
                if args[2].check(types)? != t {
                    return err(ErrorKind::IfBranches, 0, "both branches of if() must have the same type");
                }
                Ok(t)
            }
            Expr::Call(_, args) => {
                args.iter().try_for_each(|a| want(a, Type::Number))?;
                Ok(Type::Number)
            }
        }
    }

    // REQ-EXP-006: None instead of a panic or a non-finite number.
    #[must_use]
    pub fn value(&self, get: &dyn Fn(&str) -> Option<Value>) -> Option<Value> {
        use BinaryOp::*;
        let num = |e: &Expr| match e.value(get)? {
            Value::Number(n) => Some(n),
            Value::Bool(_) => None,
        };
        let boolean = |e: &Expr| match e.value(get)? {
            Value::Bool(b) => Some(b),
            Value::Number(_) => None,
        };
        let finite = |n: f64| n.is_finite().then_some(Value::Number(n));
        match self {
            Expr::Num { value, .. } => Some(Value::Number(*value)),
            Expr::Bool(b) => Some(Value::Bool(*b)),
            Expr::Ref(name) => get(name).filter(|v| !matches!(v, Value::Number(n) if !n.is_finite())),
            Expr::Unary(UnaryOp::Neg, a) => finite(-num(a)?),
            Expr::Unary(UnaryOp::Not, a) => Some(Value::Bool(!boolean(a)?)),
            Expr::Binary(op, a, b) => match op {
                Add => finite(num(a)? + num(b)?),
                Sub => finite(num(a)? - num(b)?),
                Mul => finite(num(a)? * num(b)?),
                Div => finite(num(a)? / num(b)?),
                Rem => finite(num(a)? % num(b)?),
                Lt => Some(Value::Bool(num(a)? < num(b)?)),
                Le => Some(Value::Bool(num(a)? <= num(b)?)),
                Gt => Some(Value::Bool(num(a)? > num(b)?)),
                Ge => Some(Value::Bool(num(a)? >= num(b)?)),
                And => Some(Value::Bool(boolean(a)? && boolean(b)?)),
                Or => Some(Value::Bool(boolean(a)? || boolean(b)?)),
                Eq | Ne => {
                    let same = match (a.value(get)?, b.value(get)?) {
                        (Value::Number(x), Value::Number(y)) => x == y,
                        (Value::Bool(x), Value::Bool(y)) => x == y,
                        _ => return None,
                    };
                    Some(Value::Bool(same == (*op == Eq)))
                }
            },
            Expr::Call(Func::If, args) => {
                if boolean(&args[0])? {
                    args[1].value(get)
                } else {
                    args[2].value(get)
                }
            }
            Expr::Call(func, args) => {
                let a: Vec<f64> = args.iter().map(num).collect::<Option<_>>()?;
                finite(match func {
                    Func::Abs => a[0].abs(),
                    Func::Min => a.iter().copied().fold(f64::INFINITY, f64::min),
                    Func::Max => a.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    Func::Clamp if a[1] <= a[2] => a[0].clamp(a[1], a[2]),
                    Func::Clamp => f64::NAN,
                    Func::Round => a[0].round(),
                    Func::Floor => a[0].floor(),
                    Func::Ceil => a[0].ceil(),
                    Func::Sqrt => a[0].sqrt(),
                    Func::Pow => a[0].powf(a[1]),
                    Func::Exp => a[0].exp(),
                    Func::Ln => a[0].ln(),
                    Func::Log10 => a[0].log10(),
                    Func::If => return None,
                })
            }
        }
    }

    #[must_use]
    pub fn evaluate(&self, inputs: &dyn Fn(&str) -> Option<Sample>) -> Evaluated {
        let quality = self.refs().iter().map(|name| inputs(name).map_or(Quality::Bad, |s| s.quality)).fold(Quality::Good, Quality::worst);
        if quality == Quality::Bad {
            return Evaluated { value: None, quality };
        }
        match self.value(&|name| inputs(name).map(|s| s.value)) {
            Some(value) => Evaluated { value: Some(value), quality },
            None => Evaluated { value: None, quality: Quality::Bad },
        }
    }

    // REQ-EXP-008: fails toward annunciation.
    #[must_use]
    pub fn suppresses(&self, inputs: &dyn Fn(&str) -> Option<Sample>) -> bool {
        let e = self.evaluate(inputs);
        e.quality == Quality::Good && e.value == Some(Value::Bool(true))
    }

    fn is_int(&self, param_int: &dyn Fn(&str) -> bool) -> bool {
        use BinaryOp::*;
        match self {
            Expr::Num { int, .. } => *int,
            Expr::Bool(_) => false,
            Expr::Ref(name) => param_int(name),
            Expr::Unary(UnaryOp::Neg, a) => a.is_int(param_int),
            Expr::Unary(UnaryOp::Not, _) => false,
            Expr::Binary(Add | Sub | Mul | Rem, a, b) => a.is_int(param_int) && b.is_int(param_int),
            Expr::Binary(..) => false,
            Expr::Call(Func::Round | Func::Floor | Func::Ceil, _) => true,
            Expr::Call(Func::Abs | Func::Min | Func::Max | Func::Clamp, args) => args.iter().all(|a| a.is_int(param_int)),
            Expr::Call(Func::If, args) => args[1].is_int(param_int) && args[2].is_int(param_int),
            Expr::Call(..) => false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evaluated {
    pub value: Option<Value>,
    pub quality: Quality,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Param {
    /// `int`: written without a fraction, so it prints as `272`, not `272.0`.
    Number {
        value: f64,
        int: bool,
    },
    Text(String),
}

/// Expands `{…}` groups. A field that is one group producing a number stays
/// a number; anything else becomes text.
pub fn expand(template: &str, params: &dyn Fn(&str) -> Option<Param>) -> Result<Param, ExprError> {
    let number = |name: &str| match params(name) {
        Some(Param::Number { value, int }) => Some((value, int)),
        _ => None,
    };
    let mut out = String::new();
    let mut sole = None;
    let (mut rest, mut offset, mut groups) = (template, 0, 0);
    while let Some(open) = rest.find('{') {
        let Some(len) = rest[open..].find('}') else {
            return err(ErrorKind::Syntax, offset + open, "missing closing brace");
        };
        let at = offset + open + 1;
        let fail = |kind, message: String| ExprError { kind, pos: at, message };
        let expr = parse(&rest[open + 1..open + len]).map_err(|e| ExprError { pos: e.pos + at, ..e })?;
        out.push_str(&rest[..open]);
        groups += 1;
        offset += open + len + 1;
        rest = &rest[open + len + 1..];
        if let Expr::Ref(name) = &expr {
            if let Some(Param::Text(text)) = params(name) {
                out.push_str(&text);
                continue;
            }
        }
        for name in expr.refs() {
            match params(name) {
                Some(Param::Number { .. }) => {}
                Some(Param::Text(_)) => {
                    return Err(fail(ErrorKind::TypeMismatch, format!("parameter '{name}' is text and cannot be computed with")))
                }
                None => return Err(fail(ErrorKind::UnknownReference, format!("unknown parameter '{name}'"))),
            }
        }
        expr.check(&|name| number(name).map(|_| Type::Number)).map_err(|e| fail(e.kind, e.message))?;
        let Some(Value::Number(value)) = expr.value(&|name| number(name).map(|(v, _)| Value::Number(v))) else {
            return Err(fail(ErrorKind::NoValue, "the expansion has no finite numeric value".into()));
        };
        let int = expr.is_int(&|name| number(name).is_some_and(|(_, i)| i)) && value.fract() == 0.0;
        // Display, not Debug: the lexer has no exponent syntax. Parentheses
        // keep `-{k}` valid when k is negative.
        let mut text = format!("{value}");
        if !int && !text.contains('.') {
            text.push_str(".0");
        }
        out.push_str(&if value.is_sign_negative() { format!("({text})") } else { text });
        sole = Some(Param::Number { value, int });
    }
    out.push_str(rest);
    let whole = template.trim();
    match sole {
        Some(n) if groups == 1 && whole.starts_with('{') && whole.find('}') == Some(whole.len() - 1) => Ok(n),
        _ => Ok(Param::Text(out)),
    }
}
