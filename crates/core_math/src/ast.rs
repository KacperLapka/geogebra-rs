//! AST nodes and error shapes (AD-2).
//!
//! Every node and every parse error carries a byte `Span` into the input.

use std::fmt;

/// Byte range into the formula input: `input[start..end]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Smallest span covering both.
    pub fn cover(a: Span, b: Span) -> Self {
        Self {
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {})", self.start, self.end)
    }
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Pos,
}

/// Built-in single-argument functions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Func {
    Sin,
    Cos,
    Tan,
    Exp,
    /// Natural logarithm (`ln`).
    Ln,
    /// Natural logarithm (`log` alias for the tracer; base selection is out of scope).
    Log,
    Sqrt,
}

impl Func {
    pub fn name(self) -> &'static str {
        match self {
            Func::Sin => "sin",
            Func::Cos => "cos",
            Func::Tan => "tan",
            Func::Exp => "exp",
            Func::Ln => "ln",
            Func::Log => "log",
            Func::Sqrt => "sqrt",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "sin" => Some(Func::Sin),
            "cos" => Some(Func::Cos),
            "tan" => Some(Func::Tan),
            "exp" => Some(Func::Exp),
            "ln" => Some(Func::Ln),
            "log" => Some(Func::Log),
            "sqrt" => Some(Func::Sqrt),
            _ => None,
        }
    }

    pub fn apply(self, v: f64) -> f64 {
        match self {
            Func::Sin => v.sin(),
            Func::Cos => v.cos(),
            Func::Tan => v.tan(),
            Func::Exp => v.exp(),
            Func::Ln | Func::Log => v.ln(),
            Func::Sqrt => v.sqrt(),
        }
    }
}

/// Mathematical expression AST.
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Number {
        value: f64,
        span: Span,
    },
    /// The independent variable (conventionally `x`).
    Variable {
        name: String,
        span: Span,
    },
    /// A slider-bound parameter (any identifier that is not `x` or a function).
    Param {
        name: String,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Binary {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    Call {
        func: Func,
        arg: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number { span, .. }
            | Expr::Variable { span, .. }
            | Expr::Param { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Call { span, .. } => *span,
        }
    }

    pub fn number(value: f64, span: Span) -> Self {
        Expr::Number { value, span }
    }

    pub fn zero(span: Span) -> Self {
        Expr::Number { value: 0.0, span }
    }

    pub fn one(span: Span) -> Self {
        Expr::Number { value: 1.0, span }
    }

    /// Collect slider parameter names in first-use order.
    pub fn params_used(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.collect_params(&mut out);
        out
    }

    fn collect_params(&self, out: &mut Vec<String>) {
        match self {
            Expr::Param { name, .. } => {
                if !out.iter().any(|n| n == name) {
                    out.push(name.clone());
                }
            }
            Expr::Unary { expr, .. } => expr.collect_params(out),
            Expr::Binary { left, right, .. } => {
                left.collect_params(out);
                right.collect_params(out);
            }
            Expr::Call { arg, .. } => arg.collect_params(out),
            Expr::Number { .. } | Expr::Variable { .. } => {}
        }
    }
}

/// Syntax error with an exact character span (AD-2, NFR-FUNC-01).
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub span: Span,
    pub message: String,
}

impl ParseError {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at {}", self.message, self.span)
    }
}

impl std::error::Error for ParseError {}
