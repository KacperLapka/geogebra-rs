//! Pratt parser: tokens -> `Expr` (AD-2).
//!
//! Supported forms (tracer): polynomials, rationals, `sin/cos/tan/exp/log/ln/sqrt`,
//! parameters, `+ - * / ^` (`**` aliases `^`), parentheses, unary `+`/`-`.
//! Accepts an optional `f(x) =` / `y =` prefix from the formula bar.

use crate::ast::{BinOp, Expr, Func, ParseError, Span, UnaryOp};
use crate::lexer::{Token, TokenKind, lex};

/// Parse a formula into an AST, or return a span-annotated error.
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let body = strip_prefix(input);
    let offset = input.len() - body.len();
    if body.trim().is_empty() {
        return Err(ParseError::new(
            Span::new(0, input.len()),
            "empty expression",
        ));
    }
    let tokens = lex(body).map_err(|e| {
        ParseError::new(
            Span::new(e.span.start + offset, e.span.end + offset),
            e.message,
        )
    })?;
    let mut p = Parser::new(tokens, offset);
    let expr = p.parse_expression(0)?;
    if let Some(t) = p.peek() {
        return Err(ParseError::new(
            t.span,
            format!("unexpected {}", describe(&t.kind)),
        ));
    }
    Ok(expr)
}

/// Strip an optional `f(x) =` / `y =` head, returning the body slice.
fn strip_prefix(input: &str) -> &str {
    let trimmed = input.trim_start();
    // `name(x) = body`
    if let Some(paren) = trimmed.find('(') {
        let head = &trimmed[..paren];
        if !head.is_empty()
            && head.chars().all(|c| c.is_alphanumeric() || c == '_')
            && let Some(close) = trimmed.find(')')
            && trimmed[paren + 1..close].trim() == "x"
            && let Some(eq) = trimmed[close + 1..].find('=')
        {
            return &trimmed[close + 1 + eq + 1..];
        }
    }
    // `y = body`
    if let Some(eq) = trimmed.find('=')
        && trimmed[..eq].trim() == "y"
    {
        return &trimmed[eq + 1..];
    }
    trimmed
}

fn describe(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Number(v) => format!("number '{v}'"),
        TokenKind::Ident(n) => format!("identifier '{n}'"),
        TokenKind::Plus => "'+'".to_string(),
        TokenKind::Minus => "'-'".to_string(),
        TokenKind::Star => "'*'".to_string(),
        TokenKind::Slash => "'/'".to_string(),
        TokenKind::Caret => "'^'".to_string(),
        TokenKind::LParen => "'('".to_string(),
        TokenKind::RParen => "')'".to_string(),
        TokenKind::Comma => "','".to_string(),
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>, _offset: usize) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect_rparen(&mut self) -> Result<Token, ParseError> {
        match self.bump() {
            Some(t) if matches!(t.kind, TokenKind::RParen) => Ok(t),
            Some(t) => Err(ParseError::new(
                t.span,
                format!("expected ')', found {}", describe(&t.kind)),
            )),
            None => Err(ParseError::new(
                self.end_span(),
                "expected ')', found end of input".to_string(),
            )),
        }
    }

    fn end_span(&self) -> Span {
        self.tokens
            .last()
            .map(|t| Span::new(t.span.end, t.span.end))
            .unwrap_or(Span::new(0, 0))
    }

    fn parse_expression(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;
        loop {
            let (op, prec, right_assoc) = match self.peek().map(|t| &t.kind) {
                Some(TokenKind::Plus) => (BinOp::Add, 10, false),
                Some(TokenKind::Minus) => (BinOp::Sub, 10, false),
                Some(TokenKind::Star) => (BinOp::Mul, 20, false),
                Some(TokenKind::Slash) => (BinOp::Div, 20, false),
                Some(TokenKind::Caret) => (BinOp::Pow, 30, true),
                _ => break,
            };
            if prec < min_prec {
                break;
            }
            self.bump();
            let next_min = if right_assoc { prec } else { prec + 1 };
            let right = self.parse_expression(next_min)?;
            let span = Span::cover(left.span(), right.span());
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        let Some(t) = self.bump() else {
            return Err(ParseError::new(self.end_span(), "unexpected end of input"));
        };
        match t.kind {
            TokenKind::Number(v) => Ok(Expr::Number {
                value: v,
                span: t.span,
            }),
            TokenKind::Ident(name) => self.finish_ident(name, t.span),
            TokenKind::Minus => {
                let expr = self.parse_expression(25)?;
                let span = Span::cover(t.span, expr.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::Plus => {
                let expr = self.parse_expression(25)?;
                let span = Span::cover(t.span, expr.span());
                Ok(Expr::Unary {
                    op: UnaryOp::Pos,
                    expr: Box::new(expr),
                    span,
                })
            }
            TokenKind::LParen => {
                let expr = self.parse_expression(0)?;
                let _close = self.expect_rparen()?;
                Ok(expr)
            }
            other => Err(ParseError::new(
                t.span,
                format!("unexpected {}", describe(&other)),
            )),
        }
    }

    fn finish_ident(&mut self, name: String, span: Span) -> Result<Expr, ParseError> {
        let is_call = matches!(self.peek().map(|t| &t.kind), Some(TokenKind::LParen));
        if is_call {
            let Some(func) = Func::from_name(&name) else {
                return Err(ParseError::new(span, format!("unknown function '{name}'")));
            };
            self.bump(); // '('
            if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::RParen)) {
                let close = self.bump().unwrap();
                return Err(ParseError::new(
                    Span::cover(span, close.span),
                    format!("function '{}' needs one argument", func.name()),
                ));
            }
            let arg = self.parse_expression(0)?;
            let close = self.expect_rparen()?;
            if matches!(self.peek().map(|t| &t.kind), Some(TokenKind::Comma)) {
                let comma = self.bump().unwrap();
                return Err(ParseError::new(
                    comma.span,
                    format!("function '{}' takes exactly one argument", func.name()),
                ));
            }
            let span = Span::cover(span, close.span);
            return Ok(Expr::Call {
                func,
                arg: Box::new(arg),
                span,
            });
        }
        if name == "x" {
            Ok(Expr::Variable { name, span })
        } else {
            Ok(Expr::Param { name, span })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn eval_str(formula: &str, x: f64, params: &[(&str, f64)]) -> f64 {
        let expr = parse(formula).unwrap();
        let map: HashMap<String, f64> =
            params.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        expr.eval(x, &map)
    }

    #[test]
    fn parses_polynomial_with_param() {
        assert!((eval_str("a*x^2", 3.0, &[("a", 2.0)]) - 18.0).abs() < 1e-12);
    }

    #[test]
    fn strips_function_prefix() {
        assert!((eval_str("f(x) = a*x^2", 3.0, &[("a", 2.0)]) - 18.0).abs() < 1e-12);
    }

    #[test]
    fn power_is_right_associative() {
        assert!((eval_str("2^3^2", 0.0, &[]) - 512.0).abs() < 1e-12);
    }

    #[test]
    fn parses_trig_exp_log_rational() {
        assert!((eval_str("sin(x)", 0.0, &[]) - 0.0).abs() < 1e-12);
        assert!((eval_str("cos(x)", 0.0, &[]) - 1.0).abs() < 1e-12);
        assert!((eval_str("exp(x)", 0.0, &[]) - 1.0).abs() < 1e-12);
        assert!((eval_str("log(x)", 1.0, &[]) - 0.0).abs() < 1e-12);
        assert!((eval_str("(x+1)/(x-1)", 0.0, &[]) + 1.0).abs() < 1e-12);
    }

    #[test]
    fn reports_span_errors() {
        let err = parse("a**x^").unwrap_err();
        assert!(!err.message.is_empty());
        assert!(err.span.end <= "a**x^".len());
    }

    #[test]
    fn rejects_unknown_function() {
        assert!(parse("frobnicate(x)").is_err());
    }
}
