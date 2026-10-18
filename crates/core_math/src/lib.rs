//! `core_math`: pure math engine with zero external dependencies.
//!
//! Pipeline stage: `Parse -> Differentiate/Simplify -> Evaluate`.
//! Compiles and tests natively with `cargo test` (no WASM toolchain).

pub mod ast;
pub mod diff;
pub mod eval;
pub mod lexer;
pub mod parser;
pub mod simplify;
pub mod solver;

pub use ast::{BinOp, Expr, Func, ParseError, Span, UnaryOp};
pub use diff::differentiate;
pub use parser::parse;
pub use simplify::simplify;
pub use solver::{POLE_THRESHOLD, brent, scan_sign_changes};
