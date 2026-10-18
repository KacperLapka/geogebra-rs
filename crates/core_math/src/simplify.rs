//! Constant folding, identity elimination, sign normalization (AD-3).

use crate::ast::{BinOp, Expr, Func, Span, UnaryOp};

/// Simplify an expression: fold constants, drop identities, normalize signs.
///
/// Terminates: every rewrite strictly shrinks the tree or moves constants
/// toward the leaves.
pub fn simplify(expr: &Expr) -> Expr {
    match expr {
        Expr::Number { .. } | Expr::Variable { .. } | Expr::Param { .. } => expr.clone(),
        Expr::Unary { op, expr: inner, span } => {
            let e = simplify(inner);
            simplify_unary(*op, e, *span)
        }
        Expr::Binary { op, left, right, span } => {
            let l = simplify(left);
            let r = simplify(right);
            simplify_binary(*op, l, r, *span)
        }
        Expr::Call { func, arg, span } => {
            let a = simplify(arg);
            if let Expr::Number { value, .. } = a {
                return Expr::Number {
                    value: func.apply(value),
                    span: *span,
                };
            }
            Expr::Call {
                func: *func,
                arg: Box::new(a),
                span: *span,
            }
        }
    }
}

fn simplify_unary(op: UnaryOp, e: Expr, span: Span) -> Expr {
    match (op, e) {
        // constant folding
        (UnaryOp::Neg, Expr::Number { value, .. }) => Expr::number(-value, span),
        (UnaryOp::Pos, Expr::Number { value, .. }) => Expr::number(value, span),
        // sign normalization: --x -> x, +-x -> -x
        (UnaryOp::Neg, Expr::Unary { op: UnaryOp::Neg, expr, .. }) => *expr,
        (UnaryOp::Pos, inner) => inner,
        (op, inner) => Expr::Unary {
            op,
            expr: Box::new(inner),
            span,
        },
    }
}

fn num(value: f64, span: Span) -> Expr {
    Expr::number(value, span)
}

fn is_zero(e: &Expr) -> bool {
    matches!(e, Expr::Number { value, .. } if *value == 0.0)
}

fn is_one(e: &Expr) -> bool {
    matches!(e, Expr::Number { value, .. } if *value == 1.0)
}

fn as_num(e: &Expr) -> Option<f64> {
    match e {
        Expr::Number { value, .. } => Some(*value),
        _ => None,
    }
}

fn simplify_binary(op: BinOp, l: Expr, r: Expr, span: Span) -> Expr {
    // constant folding
    if let (Some(a), Some(b)) = (as_num(&l), as_num(&r)) {
        let value = match op {
            BinOp::Add => a + b,
            BinOp::Sub => a - b,
            BinOp::Mul => a * b,
            BinOp::Div => a / b,
            BinOp::Pow => a.powf(b),
        };
        return num(value, span);
    }
    match op {
        BinOp::Add => {
            if is_zero(&l) {
                return r;
            }
            if is_zero(&r) {
                return l;
            }
            binary(op, l, r, span)
        }
        BinOp::Sub => {
            if is_zero(&r) {
                return l;
            }
            if is_zero(&l) {
                return Expr::Unary {
                    op: UnaryOp::Neg,
                    expr: Box::new(r),
                    span,
                };
            }
            binary(op, l, r, span)
        }
        BinOp::Mul => {
            if is_zero(&l) || is_zero(&r) {
                return num(0.0, span);
            }
            if is_one(&l) {
                return r;
            }
            if is_one(&r) {
                return l;
            }
            binary(op, l, r, span)
        }
        BinOp::Div => {
            if is_zero(&l) {
                return num(0.0, span);
            }
            if is_one(&r) {
                return l;
            }
            binary(op, l, r, span)
        }
        BinOp::Pow => {
            if let Some(b) = as_num(&r) {
                if b == 0.0 {
                    return num(1.0, span);
                }
                if b == 1.0 {
                    return l;
                }
            }
            if let Some(a) = as_num(&l) {
                if a == 0.0 || a == 1.0 {
                    return num(a, span);
                }
            }
            binary(op, l, r, span)
        }
    }
}

fn binary(op: BinOp, left: Expr, right: Expr, span: Span) -> Expr {
    Expr::Binary {
        op,
        left: Box::new(left),
        right: Box::new(right),
        span,
    }
}

/// Placeholder for `Func` so rustdoc links resolve in diff rules.
#[allow(unused)]
fn _func_link(_: Func) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn simp(s: &str) -> Expr {
        simplify(&parse(s).unwrap())
    }

    #[test]
    fn folds_constants() {
        assert_eq!(simp("2+3*4"), Expr::number(14.0, simp("2+3*4").span()));
    }

    #[test]
    fn eliminates_identities() {
        let e = simp("x*1+0");
        assert!(matches!(e, Expr::Variable { .. }));
        let e = simp("x^1");
        assert!(matches!(e, Expr::Variable { .. }));
        assert!(is_zero(&simp("0*x+0")));
    }

    #[test]
    fn normalizes_double_negation() {
        let e = simp("--x");
        assert!(matches!(e, Expr::Variable { .. }));
    }
}
