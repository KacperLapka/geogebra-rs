//! Symbolic differentiation (AD-3).
//!
//! Invariant: [`differentiate`] always simplifies before returning, so no
//! caller can observe or re-derive an unsimplified tree.

use crate::ast::{BinOp, Expr, Func, Span, UnaryOp};
use crate::simplify::simplify;

/// Differentiate `expr` with respect to `var`, returning the simplified result.
pub fn differentiate(expr: &Expr, var: &str) -> Expr {
    simplify(&diff_raw(expr, var))
}

fn diff_raw(expr: &Expr, var: &str) -> Expr {
    let span = expr.span();
    match expr {
        Expr::Number { .. } => Expr::zero(span),
        Expr::Variable { name, .. } => {
            if name == var {
                Expr::one(span)
            } else {
                Expr::zero(span)
            }
        }
        Expr::Param { .. } => Expr::zero(span),
        Expr::Unary {
            op, expr: inner, ..
        } => match op {
            UnaryOp::Neg => neg(diff_raw(inner, var), span),
            UnaryOp::Pos => diff_raw(inner, var),
        },
        Expr::Binary {
            op, left, right, ..
        } => match op {
            BinOp::Add => add(diff_raw(left, var), diff_raw(right, var), span),
            BinOp::Sub => sub(diff_raw(left, var), diff_raw(right, var), span),
            BinOp::Mul => {
                // (u v)' = u' v + u v'
                let dl = diff_raw(left, var);
                let dr = diff_raw(right, var);
                add(
                    mul(dl, (**right).clone(), span),
                    mul((**left).clone(), dr, span),
                    span,
                )
            }
            BinOp::Div => {
                // (u/v)' = (u' v - u v') / v^2
                let dl = diff_raw(left, var);
                let dr = diff_raw(right, var);
                div(
                    sub(
                        mul(dl, (**right).clone(), span),
                        mul((**left).clone(), dr, span),
                        span,
                    ),
                    pow((**right).clone(), Expr::number(2.0, span), span),
                    span,
                )
            }
            BinOp::Pow => diff_pow(left, right, diff_raw(left, var), diff_raw(right, var), span),
        },
        Expr::Call { func, arg, .. } => {
            let darg = diff_raw(arg, var);
            let inner = (*arg).clone();
            mul(diff_call(*func, *inner, span), darg, span)
        }
    }
}

/// d/dx `base^exp` given the operand derivatives.
fn diff_pow(base: &Expr, exp: &Expr, dbase: Expr, dexp: Expr, span: Span) -> Expr {
    // Constant exponent: c * base^(c-1) * dbase.
    if let Expr::Number { value: c, .. } = exp {
        if *c == 0.0 {
            return Expr::zero(span);
        }
        return mul(
            mul(
                Expr::number(*c, span),
                pow(base.clone(), Expr::number(c - 1.0, span), span),
                span,
            ),
            dbase,
            span,
        );
    }
    // Constant base: base^exp * ln(base) * dexp.
    if let Expr::Number { value: b, .. } = base {
        return mul(
            mul(
                pow(Expr::number(*b, span), exp.clone(), span),
                call(Func::Ln, Expr::number(*b, span), span),
                span,
            ),
            dexp,
            span,
        );
    }
    // General case via logarithmic differentiation:
    // (u^v)' = u^v * (v' * ln u + v * u' / u).
    let u = base.clone();
    let v = exp.clone();
    mul(
        pow(u.clone(), v.clone(), span),
        add(
            mul(dexp, call(Func::Ln, u.clone(), span), span),
            mul(v, div(dbase, u, span), span),
            span,
        ),
        span,
    )
}

/// d/dx of the outer function applied to the inner expression.
fn diff_call(func: Func, inner: Expr, span: Span) -> Expr {
    match func {
        Func::Sin => call(Func::Cos, inner, span),
        Func::Cos => neg(call(Func::Sin, inner, span), span),
        // sec^2 = 1/cos^2
        Func::Tan => div(
            Expr::one(span),
            pow(
                call(Func::Cos, inner.clone(), span),
                Expr::number(2.0, span),
                span,
            ),
            span,
        ),
        Func::Exp => call(Func::Exp, inner, span),
        Func::Ln | Func::Log => div(Expr::one(span), inner, span),
        Func::Sqrt => div(
            Expr::one(span),
            mul(
                Expr::number(2.0, span),
                call(Func::Sqrt, inner.clone(), span),
                span,
            ),
            span,
        ),
    }
}

fn add(l: Expr, r: Expr, span: Span) -> Expr {
    Expr::Binary {
        op: BinOp::Add,
        left: Box::new(l),
        right: Box::new(r),
        span,
    }
}

fn sub(l: Expr, r: Expr, span: Span) -> Expr {
    Expr::Binary {
        op: BinOp::Sub,
        left: Box::new(l),
        right: Box::new(r),
        span,
    }
}

fn mul(l: Expr, r: Expr, span: Span) -> Expr {
    Expr::Binary {
        op: BinOp::Mul,
        left: Box::new(l),
        right: Box::new(r),
        span,
    }
}

fn div(l: Expr, r: Expr, span: Span) -> Expr {
    Expr::Binary {
        op: BinOp::Div,
        left: Box::new(l),
        right: Box::new(r),
        span,
    }
}

fn pow(l: Expr, r: Expr, span: Span) -> Expr {
    Expr::Binary {
        op: BinOp::Pow,
        left: Box::new(l),
        right: Box::new(r),
        span,
    }
}

fn neg(e: Expr, span: Span) -> Expr {
    Expr::Unary {
        op: UnaryOp::Neg,
        expr: Box::new(e),
        span,
    }
}

fn call(func: Func, arg: Expr, span: Span) -> Expr {
    Expr::Call {
        func,
        arg: Box::new(arg),
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use std::collections::HashMap;

    fn deriv(formula: &str) -> Expr {
        differentiate(&parse(formula).unwrap(), "x")
    }

    fn eval_at(expr: &Expr, x: f64) -> f64 {
        expr.eval(x, &HashMap::new())
    }

    #[test]
    fn differentiates_parabola() {
        // d/dx a*x^2 = 2*a*x (param treated as constant)
        let d = deriv("a*x^2");
        let mut p = HashMap::new();
        p.insert("a".to_string(), 3.0);
        assert!((d.eval(2.0, &p) - 12.0).abs() < 1e-9);
    }

    #[test]
    fn differentiates_trig_and_exp() {
        assert!((eval_at(&deriv("sin(x)"), 0.0) - 1.0).abs() < 1e-9);
        assert!((eval_at(&deriv("exp(x)"), 0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn result_is_simplified() {
        // d/dx (x + 0) must not carry the dead constant
        let d = deriv("x+0");
        assert!(matches!(d, Expr::Number { value, .. } if value == 1.0));
    }

    #[test]
    fn differentiates_transcendental_compositions_with_chain_rule() {
        // sin(x^2)' = 2x·cos(x^2); exp(sin x)' = cos x·exp(sin x);
        // ln(x^2+1)' = 2x/(x^2+1); cos(x)' = -sin(x).
        let cases: &[(&str, &[f64], fn(f64) -> f64)] = &[
            ("sin(x^2)", &[0.0, 0.5, -1.3, 2.0], |x| 2.0 * x * (x * x).cos()),
            (
                "exp(sin(x))",
                &[0.0, 0.7, -1.1, 2.0],
                |x| x.cos() * x.sin().exp(),
            ),
            (
                "ln(x^2+1)",
                &[0.0, 1.3, -2.0],
                |x| 2.0 * x / (x * x + 1.0),
            ),
            ("cos(x)", &[0.3, -0.8], |x| -x.sin()),
            (
                "sin(cos(x))",
                &[0.9],
                |x| -x.sin() * x.cos().cos(),
            ),
        ];
        for (formula, xs, want) in cases {
            let d = deriv(formula);
            for x in *xs {
                assert!(
                    (eval_at(&d, *x) - want(*x)).abs() < 1e-12,
                    "{formula} x={x}: got {} want {}",
                    eval_at(&d, *x),
                    want(*x)
                );
            }
        }
    }

    #[test]
    fn differentiates_transcendental_second_derivatives() {
        // sin'' = -sin; exp'' = exp; ln(x)'' = -1/x^2 (x > 0).
        let f = parse("sin(x)").unwrap();
        let f2 = differentiate(&differentiate(&f, "x"), "x");
        let g = parse("exp(x)").unwrap();
        let g2 = differentiate(&differentiate(&g, "x"), "x");
        let h = parse("ln(x)").unwrap();
        let h2 = differentiate(&differentiate(&h, "x"), "x");
        for x in [0.0, 0.7, -1.1] {
            assert!(
                (eval_at(&f2, x) - -x.sin()).abs() < 1e-12,
                "sin'' x={x}"
            );
            assert!(
                (eval_at(&g2, x) - x.exp()).abs() < 1e-12,
                "exp'' x={x}"
            );
        }
        for x in [0.5, 2.0] {
            assert!(
                (eval_at(&h2, x) - -1.0 / (x * x)).abs() < 1e-12,
                "ln'' x={x}"
            );
        }
    }

    #[test]
    fn transcendental_derivatives_are_canonically_simplified() {
        // exp(0) folds to 1, so d/dx (exp(0)*x) is exactly 1.
        let d = deriv("exp(0)*x");
        assert!(
            matches!(d, Expr::Number { value, .. } if value == 1.0),
            "canonical: got {d:?}"
        );
        // ln(1) folds to 0, so d/dx (ln(1)+cos(x)) is -sin(x).
        let e = deriv("ln(1)+cos(x)");
        for x in [0.0, 0.3, -0.8] {
            assert!(
                (eval_at(&e, x) - -x.sin()).abs() < 1e-12,
                "x={x}: got {}",
                eval_at(&e, x)
            );
        }
    }
}
