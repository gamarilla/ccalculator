//! ccalc-core — the evaluation engine behind Console Calculator.
//!
//! High-precision arithmetic, a units converter, base conversion, user
//! variables and functions, and the command set, all exposed through [`Engine`].

pub mod ast;
pub mod engine;
pub mod format;
pub mod lexer;
pub mod number;
pub mod parser;
pub mod theme;
pub mod units;
pub mod value;

pub use engine::{Angle, DefKind, DefRecord, Engine, Eval, InputLayout};
pub use format::{Base, DisplaySettings, SciMode};
pub use theme::{Palette, Theme};
pub use value::Value;

/// Evaluate a single line with a fresh engine and return the display lines.
/// Convenience for quick one-shot use and tests.
pub fn eval_once(line: &str) -> Vec<String> {
    let mut e = Engine::new();
    e.run_line(line)
        .iter()
        .filter_map(|ev| e.format_eval(ev))
        .collect()
}
