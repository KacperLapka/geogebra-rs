//! Recursive tree-walk evaluator: `expr.eval(x, &params) -> f64` (AD-2).
//!
//! IEEE 754 double precision throughout; unknown parameters and domain
//! errors surface as `NaN`/infinities per IEEE semantics, never panics.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, UnaryOp};

impl Expr {
    /// Evaluate at `x` with slider parameters `params`.
    pub fn eval(&self, x: f64, params: &HashMap<String, f64>) -> f64 {
        match self {
            Expr::Number { value, .. } => *value,
            Expr::Variable { .. } => x,
            Expr::Param { name, .. } => params.get(name).copied().unwrap_or(f64::NAN),
            Expr::Unary { op, expr, .. } => {
                let v = expr.eval(x, params);
                match op {
                    UnaryOp::Neg => -v,
                    UnaryOp::Pos => v,
                }
            }
            Expr::Binary { op, left, right, .. } => {
                let l = left.eval(x, params);
                let r = right.eval(x, params);
                match op {
                    BinOp::Add => l + r,
                    BinOp::Sub => l - r,
                    BinOp::Mul => l * r,
                    BinOp::Div => l / r,
                    BinOp::Pow => l.powf(r),
                }
            }
            Expr::Call { func, arg, .. } => func.apply(arg.eval(x, params)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn params(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn evaluates_parabola_with_slider() {
        let expr = parse("a*x^2").unwrap();
        assert!((expr.eval(4.0, &params(&[("a", 0.5)])) - 8.0).abs() < 1e-12);
    }

    #[test]
    fn missing_param_is_nan_not_panic() {
        let expr = parse("a*x").unwrap();
        assert!(expr.eval(1.0, &params(&[])).is_nan());
    }

    #[test]
    fn division_by_zero_follows_ieee() {
        let expr = parse("1/(x-1)").unwrap();
        assert!(expr.eval(1.0, &params(&[])).is_infinite());
    }

    #[test]
    fn matches_f64_reference_within_tolerance() {
        let expr = parse("sin(x)+cos(x)*exp(x)").unwrap();
        for i in -10..=10 {
            let x = i as f64 * 0.25;
            let got = expr.eval(x, &params(&[]));
            let want = x.sin() + x.cos() * x.exp();
            assert!((got - want).abs() < 1e-12, "x={x}: {got} vs {want}");
        }
    }

    #[test]
    fn evaluates_full_form_family_with_params() {
        let cases: &[(&str, f64, &[(&str, f64)], f64)] = &[
            ("a*x^2+b*x+c", 2.0, &[("a", 1.0), ("b", 2.0), ("c", 3.0)], 11.0),
            ("(x+1)/(x-1)", 0.0, &[], -1.0),
            ("tan(x)", 0.0, &[], 0.0),
            ("ln(x+1)", 0.0, &[], 0.0),
            ("sqrt(a^2+x^2)", 4.0, &[("a", 3.0)], 5.0),
        ];
        for (formula, x, ps, want) in cases {
            let expr = parse(formula).unwrap();
            let got = expr.eval(*x, &params(ps));
            assert!((got - want).abs() < 1e-12, "{formula} x={x}: {got} vs {want}");
        }
    }

    #[test]
    fn domain_errors_are_nan_not_panic() {
        let expr = parse("log(x)+sqrt(x)").unwrap();
        assert!(expr.eval(-1.0, &params(&[])).is_nan());
    }
}
