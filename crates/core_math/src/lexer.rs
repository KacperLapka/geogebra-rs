//! Character-level tokenizer (AD-2).

use crate::ast::Span;

/// A lexical token with its byte span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Token kinds. Numbers are parsed eagerly to `f64`.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LexError {
    pub span: Span,
    pub message: String,
}

/// Tokenize `input`, skipping whitespace. Spans are byte offsets.
pub fn lex(input: &str) -> Result<Vec<Token>, LexError> {
    let bytes = input.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // `i` always rests on a char boundary; decode one char for span math.
        let c = input[i..].chars().next().unwrap();
        if c.is_whitespace() {
            i += c.len_utf8();
            continue;
        }
        let start = i;
        match c {
            '+' => {
                tokens.push(tok(TokenKind::Plus, start, i + 1));
                i += 1;
            }
            '-' => {
                tokens.push(tok(TokenKind::Minus, start, i + 1));
                i += 1;
            }
            '*' => {
                if bytes.get(i + 1) == Some(&b'*') {
                    tokens.push(tok(TokenKind::Caret, start, i + 2));
                    i += 2;
                } else {
                    tokens.push(tok(TokenKind::Star, start, i + 1));
                    i += 1;
                }
            }
            '/' => {
                tokens.push(tok(TokenKind::Slash, start, i + 1));
                i += 1;
            }
            '^' => {
                tokens.push(tok(TokenKind::Caret, start, i + 1));
                i += 1;
            }
            '(' => {
                tokens.push(tok(TokenKind::LParen, start, i + 1));
                i += 1;
            }
            ')' => {
                tokens.push(tok(TokenKind::RParen, start, i + 1));
                i += 1;
            }
            ',' => {
                tokens.push(tok(TokenKind::Comma, start, i + 1));
                i += 1;
            }
            '0'..='9' | '.' => {
                let (value, end) = lex_number(input, start)?;
                tokens.push(tok(TokenKind::Number(value), start, end));
                i = end;
            }
            _ if c.is_alphabetic() || c == '_' => {
                let mut end = i + c.len_utf8();
                while let Some(ch) = input[end..].chars().next() {
                    if ch.is_alphanumeric() || ch == '_' {
                        end += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                // `end` rests on a char boundary by construction; slicing is safe
                let name = input[start..end].to_string();
                tokens.push(tok(TokenKind::Ident(name), start, end));
                i = end;
            }
            _ => {
                return Err(LexError {
                    span: Span::new(start, start + c.len_utf8()),
                    message: format!("unexpected character '{c}'"),
                });
            }
        }
    }
    Ok(tokens)
}

fn tok(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: Span::new(start, end),
    }
}

fn lex_number(input: &str, start: usize) -> Result<(f64, usize), LexError> {
    let bytes = input.as_bytes();
    let mut end = start;
    let mut seen_digit = false;
    while let Some(&b) = bytes.get(end) {
        if b.is_ascii_digit() {
            seen_digit = true;
            end += 1;
        } else {
            break;
        }
    }
    if bytes.get(end) == Some(&b'.') {
        end += 1;
        while let Some(&b) = bytes.get(end) {
            if b.is_ascii_digit() {
                seen_digit = true;
                end += 1;
            } else {
                break;
            }
        }
    }
    if !seen_digit {
        return Err(LexError {
            span: Span::new(start, end.max(start + 1)),
            message: "expected a number".to_string(),
        });
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut j = end + 1;
        if matches!(bytes.get(j), Some(b'+' | b'-')) {
            j += 1;
        }
        let exp_start = j;
        while let Some(&b) = bytes.get(j) {
            if b.is_ascii_digit() {
                j += 1;
            } else {
                break;
            }
        }
        if j > exp_start {
            end = j;
        }
    }
    let text = &input[start..end];
    text.parse::<f64>().map(|v| (v, end)).map_err(|_| LexError {
        span: Span::new(start, end),
        message: format!("invalid number '{text}'"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexes_expression_with_spans() {
        let toks = lex("a*x^2+1.5").unwrap();
        assert_eq!(toks.len(), 7);
        assert_eq!(toks[0].span, Span::new(0, 1));
        assert!(matches!(toks[1].kind, TokenKind::Star));
        assert!(matches!(toks[3].kind, TokenKind::Caret));
    }

    #[test]
    fn rejects_bad_character_with_span() {
        let err = lex("x$").unwrap_err();
        assert_eq!(err.span, Span::new(1, 2));
    }

    #[test]
    fn lexes_scientific_notation() {
        let toks = lex("2.5e-3").unwrap();
        assert_eq!(toks.len(), 1);
        match toks[0].kind {
            TokenKind::Number(v) => assert!((v - 0.0025).abs() < 1e-15),
            _ => panic!("expected number"),
        }
    }

    #[test]
    fn reports_multibyte_unexpected_character_without_panic() {
        // `€` is 3 bytes in UTF-8 starting at byte offset 2.
        let err = lex("x+€").unwrap_err();
        assert_eq!(err.span, Span::new(2, 5));
        assert!(err.message.contains('€'));
    }

    #[test]
    fn lexes_underscore_and_alphanumeric_idents() {
        let toks = lex("alpha_1 + _b2").unwrap();
        assert_eq!(toks.len(), 3);
        assert!(matches!(&toks[0].kind, TokenKind::Ident(n) if n == "alpha_1"));
        assert!(matches!(&toks[2].kind, TokenKind::Ident(n) if n == "_b2"));
    }

    #[test]
    fn star_star_aliases_caret() {
        let toks = lex("x**2").unwrap();
        assert_eq!(toks.len(), 3);
        assert!(matches!(toks[1].kind, TokenKind::Caret));
        assert_eq!(toks[1].span, Span::new(1, 3));
    }
}
