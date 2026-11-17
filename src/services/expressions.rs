//! A bounded expression parser and evaluator for report formulas.
use super::relations::{Relations, RowContext};
use crate::repository::{DataValue, Error, Result};

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Text(String),
    Name(String),
    Symbol(String),
    End,
}
#[derive(Clone, Debug)]
enum Node {
    Literal(DataValue),
    Field(String),
    Unary(String, Box<Node>),
    Binary(String, Box<Node>, Box<Node>),
    Call(String, Vec<Node>),
}
fn invalid(message: &str) -> Error {
    Error::InvalidReport(format!("expression: {message}"))
}
fn tokens(source: &str) -> Result<Vec<Token>> {
    if source.len() > 2048 {
        return Err(invalid("too long"));
    }
    let chars: Vec<char> = source.chars().collect();
    let mut result = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            i += 1;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let value: String = chars[start..i].iter().collect();
            let n = value
                .parse::<f64>()
                .map_err(|_| invalid("invalid number"))?;
            if !n.is_finite() {
                return Err(invalid("number must be finite"));
            }
            result.push(Token::Number(n));
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while i < chars.len()
                && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                i += 1;
            }
            result.push(Token::Name(chars[start..i].iter().collect()));
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < chars.len() {
                let x = chars[i];
                i += 1;
                if x == quote {
                    closed = true;
                    break;
                }
                if x == '\\' {
                    if i >= chars.len() {
                        break;
                    }
                    value.push(chars[i]);
                    i += 1;
                } else {
                    value.push(x);
                }
            }
            if !closed {
                return Err(invalid("unclosed string"));
            }
            result.push(Token::Text(value));
            continue;
        }
        let three: String = chars[i..(i + 3).min(chars.len())].iter().collect();
        if ["===", "!=="].contains(&three.as_str()) {
            result.push(Token::Symbol(three));
            i += 3;
            continue;
        }
        let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
        if [">=", "<=", "==", "!=", "<>", "&&", "||"].contains(&two.as_str()) {
            result.push(Token::Symbol(two));
            i += 2;
            continue;
        }
        if "+-*/(),=><!".contains(c) {
            result.push(Token::Symbol(c.to_string()));
            i += 1;
            continue;
        }
        return Err(invalid("unsupported symbol"));
    }
    result.push(Token::End);
    Ok(result)
}
struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}
impl Parser {
    fn parse(source: &str) -> Result<Node> {
        let mut p = Self {
            tokens: tokens(source)?,
            pos: 0,
            depth: 0,
        };
        let result = p.expression(0)?;
        if p.tokens[p.pos] != Token::End {
            return Err(invalid("unexpected token"));
        }
        Ok(result)
    }
    fn expression(&mut self, min: u8) -> Result<Node> {
        self.depth += 1;
        if self.depth > 40 {
            return Err(invalid("nesting too deep"));
        }
        let mut node = self.primary()?;
        loop {
            let op = match &self.tokens[self.pos] {
                Token::Symbol(s) => s.as_str(),
                Token::Name(s) if s.eq_ignore_ascii_case("AND") => "&&",
                Token::Name(s) if s.eq_ignore_ascii_case("OR") => "||",
                _ => break,
            };
            let prec = match op {
                "||" => 1,
                "&&" => 2,
                "=" | "==" | "===" | "!=" | "!==" | "<>" | ">" | "<" | ">=" | "<=" => 3,
                "+" | "-" => 4,
                "*" | "/" => 5,
                _ => break,
            };
            if prec < min {
                break;
            }
            let op = op.to_owned();
            self.pos += 1;
            let right = self.expression(prec + 1)?;
            node = Node::Binary(op, Box::new(node), Box::new(right));
        }
        self.depth -= 1;
        Ok(node)
    }
    fn primary(&mut self) -> Result<Node> {
        let token = self
            .tokens
            .get(self.pos)
            .cloned()
            .ok_or_else(|| invalid("unexpected end"))?;
        self.pos += 1;
        match token {
            Token::Number(n) => Ok(Node::Literal(DataValue::Decimal(n))),
            Token::Text(s) => Ok(Node::Literal(DataValue::Text(s))),
            Token::Name(name) if name.eq_ignore_ascii_case("true") => {
                Ok(Node::Literal(DataValue::Boolean(true)))
            }
            Token::Name(name) if name.eq_ignore_ascii_case("false") => {
                Ok(Node::Literal(DataValue::Boolean(false)))
            }
            Token::Name(name) if name.eq_ignore_ascii_case("null") => {
                Ok(Node::Literal(DataValue::Null))
            }
            Token::Name(name) if name.eq_ignore_ascii_case("not") => {
                Ok(Node::Unary("!".into(), Box::new(self.expression(6)?)))
            }
            Token::Name(name) => {
                if self.tokens[self.pos] != Token::Symbol("(".into()) {
                    return Ok(Node::Field(name));
                }
                self.pos += 1;
                let mut args = Vec::new();
                if self.tokens[self.pos] != Token::Symbol(")".into()) {
                    loop {
                        args.push(self.expression(0)?);
                        if self.tokens[self.pos] != Token::Symbol(",".into()) {
                            break;
                        }
                        self.pos += 1;
                    }
                }
                if self.tokens[self.pos] != Token::Symbol(")".into()) {
                    return Err(invalid("missing closing parenthesis"));
                }
                self.pos += 1;
                Ok(Node::Call(name.to_uppercase(), args))
            }
            Token::Symbol(s) if s == "(" => {
                let n = self.expression(0)?;
                if self.tokens[self.pos] != Token::Symbol(")".into()) {
                    return Err(invalid("missing closing parenthesis"));
                }
                self.pos += 1;
                Ok(n)
            }
            Token::Symbol(s) if ["-", "+", "!"].contains(&s.as_str()) => {
                Ok(Node::Unary(s, Box::new(self.expression(6)?)))
            }
            _ => Err(invalid("invalid operand")),
        }
    }
}
pub(super) fn validate(source: &str) -> Result<()> {
    fn check(node: &Node) -> Result<()> {
        match node {
            Node::Call(name, args) => {
                let arity = match name.as_str() {
                    "IF" => 3,
                    "SUM" => 1,
                    _ => return Err(Error::Unsupported(format!("expression function {name}"))),
                };
                if args.len() != arity {
                    return Err(invalid("invalid function argument count"));
                }
                for arg in args {
                    check(arg)?;
                }
            }
            Node::Unary(_, child) => check(child)?,
            Node::Binary(_, left, right) => {
                check(left)?;
                check(right)?;
            }
            _ => {}
        }
        Ok(())
    }
    check(&Parser::parse(source)?)
}
pub(super) fn evaluate(
    source: &str,
    relations: &Relations<'_>,
    context: &RowContext,
    current: Option<&str>,
) -> Result<DataValue> {
    let trimmed = source.trim();
    let source = trimmed
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .or_else(|| trimmed.strip_prefix('{').and_then(|v| v.strip_suffix('}')))
        .unwrap_or(trimmed);
    eval(
        &Parser::parse(source)?,
        relations,
        context,
        current,
        &mut Vec::new(),
        false,
    )
}
fn numeric(value: DataValue) -> Result<f64> {
    match value {
        DataValue::Null => Ok(0.0),
        DataValue::Text(ref s) if s.is_empty() => Ok(0.0),
        DataValue::Text(ref s) => s
            .parse::<f64>()
            .ok()
            .filter(|n| n.is_finite())
            .ok_or_else(|| invalid("expected number")),
        DataValue::Integer(n) => Ok(n as f64),
        DataValue::Decimal(n) => Ok(n),
        _ => Err(invalid("expected number")),
    }
}
fn boolean(value: DataValue) -> Result<bool> {
    if let DataValue::Boolean(b) = value {
        Ok(b)
    } else {
        Err(invalid("expected boolean"))
    }
}
fn eval(
    node: &Node,
    relations: &Relations<'_>,
    context: &RowContext,
    current: Option<&str>,
    stack: &mut Vec<String>,
    aggregating: bool,
) -> Result<DataValue> {
    use DataValue as V;
    match node {
        Node::Literal(v) => Ok(v.clone()),
        Node::Field(name) => {
            if name == "ROW_NUMBER" {
                return Ok(V::Integer(context.row_number.unwrap_or(1) as i64));
            }
            if name == "PAGE_NUMBER" {
                return Ok(V::Integer(context.page_number as i64));
            }
            if name == "PAGE_COUNT" {
                return Ok(V::Integer(context.page_count as i64));
            }
            if let Some(expression) = relations.formula(name) {
                if stack.len() >= 40 || stack.contains(name) {
                    return Err(invalid("circular formula"));
                }
                stack.push(name.clone());
                let value = eval(
                    &Parser::parse(expression)?,
                    relations,
                    context,
                    current,
                    stack,
                    aggregating,
                );
                stack.pop();
                return value;
            }
            let (table, field) = if let Some(parts) = name.split_once('.') {
                parts
            } else {
                (
                    current.ok_or_else(|| invalid("unqualified field outside detail"))?,
                    name.as_str(),
                )
            };
            Ok(relations
                .value(context, table, field)?
                .cloned()
                .unwrap_or(V::Null))
        }
        Node::Unary(op, value) => {
            let v = eval(value, relations, context, current, stack, aggregating)?;
            match op.as_str() {
                "!" => Ok(V::Boolean(!boolean(v)?)),
                "-" => Ok(V::Decimal(-numeric(v)?)),
                "+" => Ok(V::Decimal(numeric(v)?)),
                _ => Err(invalid("unknown unary operator")),
            }
        }
        Node::Binary(op, left, right) => {
            let a = eval(left, relations, context, current, stack, aggregating)?;
            if op == "&&" && !boolean(a.clone())? {
                return Ok(V::Boolean(false));
            }
            if op == "||" && boolean(a.clone())? {
                return Ok(V::Boolean(true));
            }
            let b = eval(right, relations, context, current, stack, aggregating)?;
            match op.as_str() {
                "+" | "-" | "*" | "/" => {
                    let x = numeric(a)?;
                    let y = numeric(b)?;
                    let n = match op.as_str() {
                        "+" => x + y,
                        "-" => x - y,
                        "*" => x * y,
                        _ => x / y,
                    };
                    if !n.is_finite() {
                        return Err(invalid("non-finite arithmetic result"));
                    }
                    let rounded = (n * 1e10 + 0.5).floor() / 1e10;
                    if !rounded.is_finite() {
                        return Err(invalid("non-finite arithmetic result"));
                    }
                    Ok(V::Decimal(rounded))
                }
                "&&" | "||" => Ok(V::Boolean(boolean(b)?)),
                "=" | "==" | "===" | "!=" | "!==" | "<>" | ">" | "<" | ">=" | "<=" => {
                    let ordering = match (&a, &b) {
                        (V::Integer(_) | V::Decimal(_), V::Integer(_) | V::Decimal(_)) => {
                            numeric(a)?.partial_cmp(&numeric(b)?)
                        }
                        (V::Text(x) | V::Date(x), V::Text(y) | V::Date(y)) => Some(x.cmp(y)),
                        (V::Boolean(x), V::Boolean(y)) => Some(x.cmp(y)),
                        (V::Null, V::Null) => Some(std::cmp::Ordering::Equal),
                        _ => None,
                    };
                    let result = match op.as_str() {
                        "=" | "==" | "===" => ordering == Some(std::cmp::Ordering::Equal),
                        "!=" | "!==" | "<>" => ordering != Some(std::cmp::Ordering::Equal),
                        ">" => ordering.is_some_and(|x| x.is_gt()),
                        "<" => ordering.is_some_and(|x| x.is_lt()),
                        ">=" => ordering.is_some_and(|x| x.is_ge()),
                        _ => ordering.is_some_and(|x| x.is_le()),
                    };
                    Ok(V::Boolean(result))
                }
                _ => Err(invalid("unknown operator")),
            }
        }
        Node::Call(name, args) if name == "IF" && args.len() == 3 => {
            let condition = boolean(eval(
                &args[0],
                relations,
                context,
                current,
                stack,
                aggregating,
            )?)?;
            eval(
                &args[if condition { 1 } else { 2 }],
                relations,
                context,
                current,
                stack,
                aggregating,
            )
        }
        Node::Call(name, args) if name == "SUM" && args.len() == 1 => {
            if aggregating {
                return Err(invalid("nested aggregate"));
            }
            let source = current.ok_or_else(|| invalid("SUM outside detail dataset"))?;
            let mut sum = 0.0;
            for row in relations.all_contexts(source, context)? {
                let value = eval(&args[0], relations, &row, current, stack, true)?;
                if value != V::Null {
                    sum += numeric(value)?;
                }
            }
            if !sum.is_finite() {
                return Err(invalid("non-finite sum"));
            }
            Ok(V::Decimal(sum))
        }
        Node::Call(name, _) => Err(Error::Unsupported(format!("expression function {name}"))),
    }
}
