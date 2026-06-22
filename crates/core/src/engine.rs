//! The calculator engine: holds state (variables, functions, units, settings),
//! evaluates expressions, and runs commands.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, Stmt};
use crate::format::{format_value, Base, DisplaySettings, SciMode};
use crate::number::{Ctx, Num};
use crate::parser::parse_stmt;
use crate::theme::{self, Palette};
use crate::units::build_units;
use crate::value::Value;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Angle {
    Rad,
    Deg,
}

/// Where the input prompt lives in the terminal UI.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputLayout {
    /// A fixed input box pinned to the bottom of the screen.
    Bottom,
    /// An inline prompt at the end of the scrollback (original-console style).
    Inline,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DefKind {
    Var,
    Func,
    Unit,
}

/// A replayable definition, used to persist user state between sessions.
#[derive(Clone, Debug)]
pub struct DefRecord {
    pub kind: DefKind,
    pub name: String,
    pub text: String,
}

/// The result of evaluating one statement.
#[derive(Clone, Debug)]
pub enum Eval {
    /// An anonymous expression result, shown as `ans = ...`.
    Result(Value),
    /// A named result (assignment / disp), shown as `name = ...`.
    Named(String, Value),
    /// A plain text message (command output).
    Message(String),
    /// Clear the screen.
    Clear,
    /// Exit the calculator.
    Exit,
    /// Nothing to display (suppressed by semicolon).
    Quiet,
}

pub struct Engine {
    pub ctx: Ctx,
    pub vars: HashMap<String, Value>,
    pub funcs: HashMap<String, (Vec<String>, Expr)>,
    pub units: HashMap<String, Value>,
    /// Names of units the user added (for persistence).
    pub custom_units: Vec<String>,
    pub settings: DisplaySettings,
    pub angle: Angle,
    pub european: bool,
    /// Name of the active UI color theme.
    pub theme: String,
    /// Where the input prompt is drawn in the terminal UI.
    pub input_layout: InputLayout,
    /// Replayable record of user definitions (variables, functions, units).
    pub def_records: Vec<DefRecord>,
    rng: u64,
}

impl Default for Engine {
    fn default() -> Self {
        Engine::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        let mut ctx = Ctx::new();
        let units = build_units(&mut ctx);
        Engine {
            ctx,
            vars: HashMap::new(),
            funcs: HashMap::new(),
            units,
            custom_units: Vec::new(),
            settings: DisplaySettings::default(),
            angle: Angle::Rad,
            european: false,
            theme: theme::DEFAULT_THEME.to_string(),
            input_layout: InputLayout::Bottom,
            def_records: Vec::new(),
            rng: 0x9E3779B97F4A7C15,
        }
    }

    /// The color palette of the active theme (falls back to the default).
    pub fn palette(&self) -> Palette {
        theme::find(&self.theme)
            .unwrap_or_else(theme::default_theme)
            .palette
    }

    fn record_def(&mut self, kind: DefKind, name: &str, text: String) {
        self.def_records
            .retain(|d| !(d.kind == kind && d.name == name));
        self.def_records.push(DefRecord {
            kind,
            name: name.to_string(),
            text,
        });
    }

    /// Emit a replayable script reconstructing all user definitions.
    pub fn definitions_script(&self) -> String {
        self.def_records
            .iter()
            .map(|d| d.text.clone())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Seed the pseudo-random generator (frontends may seed from the clock).
    pub fn seed_rng(&mut self, seed: u64) {
        self.rng = seed | 1;
    }

    /// Format an [`Eval`] into the display line, if any.
    pub fn format_eval(&self, e: &Eval) -> Option<String> {
        match e {
            Eval::Result(v) => Some(format!("ans = {}", format_value(v, &self.settings))),
            Eval::Named(n, v) => Some(format!("{n} = {}", format_value(v, &self.settings))),
            Eval::Message(m) => Some(m.clone()),
            Eval::Clear | Eval::Exit | Eval::Quiet => None,
        }
    }

    /// Run a full input line (possibly several `;`-separated statements).
    pub fn run_line(&mut self, line: &str) -> Vec<Eval> {
        let mut out = Vec::new();
        for (stmt, suppressed) in split_statements(line) {
            if stmt.trim().is_empty() {
                continue;
            }
            let prepared = self.apply_ans_shortcut(stmt.trim());
            let res = match self.eval_statement(&prepared) {
                Ok(e) => e,
                Err(msg) => Eval::Message(format!("Error: {msg}")),
            };
            if suppressed {
                // keep side effects, drop visible output (except control signals)
                match res {
                    Eval::Clear => out.push(Eval::Clear),
                    Eval::Exit => out.push(Eval::Exit),
                    _ => out.push(Eval::Quiet),
                }
            } else {
                out.push(res);
            }
        }
        out
    }

    /// `+`, `*`, `/` … at the start of a line become `ans +`; `--` becomes `ans -`.
    fn apply_ans_shortcut(&self, s: &str) -> String {
        if let Some(rest) = s.strip_prefix("--") {
            return format!("ans -{rest}");
        }
        let first = s.chars().next();
        if let Some(c) = first {
            if matches!(c, '*' | '/' | '+' | '^' | '%' | '&' | '|' | '@' | '<' | '>') {
                return format!("ans {s}");
            }
        }
        s.to_string()
    }

    fn eval_statement(&mut self, s: &str) -> Result<Eval, String> {
        if let Some(cmd) = self.try_command(s)? {
            return Ok(cmd);
        }
        match parse_stmt(s)? {
            Stmt::Expr(e) => {
                let v = self.eval(&e)?;
                self.vars.insert("ans".to_string(), v.clone());
                Ok(Eval::Result(v))
            }
            Stmt::Assign(name, e) => {
                let v = self.eval(&e)?;
                self.vars.insert(name.clone(), v.clone());
                self.vars.insert("ans".to_string(), v.clone());
                self.record_def(DefKind::Var, &name, format!("{name} = {e}"));
                Ok(Eval::Named(name, v))
            }
            Stmt::FuncDef(name, params, body) => {
                let text = format!("{name}({}) = {body}", params.join(","));
                self.funcs.insert(name.clone(), (params, body));
                self.record_def(DefKind::Func, &name, text);
                Ok(Eval::Message(format!("function '{name}' defined")))
            }
        }
    }

    // ----------------------- expression evaluation -----------------------

    pub fn eval(&mut self, e: &Expr) -> Result<Value, String> {
        match e {
            Expr::Num {
                mantissa,
                radix,
                si_exp,
            } => {
                let mut n = Num::parse(mantissa, *radix, &mut self.ctx)?;
                if *si_exp != 0 {
                    let ten = Num::from_i64(10);
                    let exp = Num::from_i64(si_exp.unsigned_abs() as i64);
                    let scale = ten.pow(&exp, &mut self.ctx);
                    n = if *si_exp > 0 {
                        n.mul(&scale, &self.ctx)
                    } else {
                        n.div(&scale, &self.ctx)
                    };
                }
                Ok(Value::scalar(n))
            }
            Expr::Ident(name) => self.resolve_ident(name),
            Expr::Call(name, args) => self.eval_call(name, args),
            Expr::Neg(inner) => Ok(self.eval(inner)?.neg()),
            Expr::Fact(inner) => {
                let v = self.eval(inner)?;
                self.factorial(&v)
            }
            Expr::Bin(op, a, b) => {
                let lhs = self.eval(a)?;
                let rhs = self.eval(b)?;
                self.eval_binop(op, lhs, rhs)
            }
            Expr::Convert(a, b) => {
                let lhs = self.eval(a)?;
                let rhs = self.eval(b)?;
                if lhs.dim != rhs.dim {
                    return Err(format!(
                        "cannot convert [{}] to [{}]",
                        lhs.dim.describe(),
                        rhs.dim.describe()
                    ));
                }
                Ok(Value::scalar(lhs.num.div(&rhs.num, &self.ctx)))
            }
        }
    }

    fn resolve_ident(&mut self, name: &str) -> Result<Value, String> {
        if let Some(v) = self.vars.get(name) {
            return Ok(v.clone());
        }
        match name {
            "pi" => return Ok(Value::scalar(self.ctx.pi())),
            "e" => return Ok(Value::scalar(self.ctx.e())),
            "ans" => return Ok(Value::from_i64(0)),
            _ => {}
        }
        if let Some(u) = self.units.get(name) {
            return Ok(u.clone());
        }
        Err(format!("undefined name '{name}'"))
    }

    fn eval_call(&mut self, name: &str, args: &[Expr]) -> Result<Value, String> {
        // user-defined function?
        if let Some((params, body)) = self.funcs.get(name).cloned() {
            if params.len() != args.len() {
                return Err(format!(
                    "function '{name}' expects {} argument(s), got {}",
                    params.len(),
                    args.len()
                ));
            }
            let mut argv = Vec::new();
            for a in args {
                argv.push(self.eval(a)?);
            }
            // bind params in a temporary scope
            let saved: Vec<(String, Option<Value>)> = params
                .iter()
                .map(|p| (p.clone(), self.vars.get(p).cloned()))
                .collect();
            for (p, v) in params.iter().zip(argv) {
                self.vars.insert(p.clone(), v);
            }
            let result = self.eval(&body);
            for (p, old) in saved {
                match old {
                    Some(v) => {
                        self.vars.insert(p, v);
                    }
                    None => {
                        self.vars.remove(&p);
                    }
                }
            }
            return result;
        }
        // built-in math function?
        if let Some(r) = self.call_builtin(name, args)? {
            return Ok(r);
        }
        // implicit multiplication: `name(arg)` where name is a value (var/unit)
        if args.len() == 1 {
            if let Ok(v) = self.resolve_ident(name) {
                let a = self.eval(&args[0])?;
                return Ok(v.mul(&a, &self.ctx));
            }
        }
        Err(format!("unknown function '{name}'"))
    }

    fn call_builtin(&mut self, name: &str, args: &[Expr]) -> Result<Option<Value>, String> {
        // log with explicit base, e.g. log2, log8, log16
        if let Some(rest) = name.strip_prefix("log") {
            if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
                let base: i64 = rest.parse().map_err(|_| "bad log base")?;
                let x = self.one_scalar(name, args)?;
                let b = Num::from_i64(base);
                return Ok(Some(Value::scalar(x.log_base(&b, &mut self.ctx))));
            }
        }

        let unary = |slf: &mut Self,
                     f: fn(&Num, &mut Ctx) -> Num|
         -> Result<Option<Value>, String> {
            let x = slf.one_scalar(name, args)?;
            Ok(Some(Value::scalar(f(&x, &mut slf.ctx))))
        };

        let r = match name {
            "sqrt" => {
                let x = self.one_scalar(name, args)?;
                Some(Value::scalar(x.sqrt(&self.ctx)))
            }
            "cbrt" => {
                let x = self.one_scalar(name, args)?;
                let third = Num::one().div(&Num::from_i64(3), &self.ctx);
                Some(Value::scalar(x.pow(&third, &mut self.ctx)))
            }
            "exp" => return unary(self, Num::exp),
            "ln" => return unary(self, Num::ln),
            "log" | "log10" => return unary(self, Num::log10),
            "log2" => return unary(self, Num::log2),
            "abs" => {
                let v = self.one_value(name, args)?;
                Some(Value { num: v.num.abs(), dim: v.dim })
            }
            "floor" => {
                let x = self.one_scalar(name, args)?;
                Some(Value::scalar(x.floor()))
            }
            "ceil" => {
                let x = self.one_scalar(name, args)?;
                Some(Value::scalar(x.ceil()))
            }
            "round" => {
                let x = self.one_scalar(name, args)?;
                Some(Value::scalar(x.round()))
            }
            "trunc" | "int" => {
                let x = self.one_scalar(name, args)?;
                Some(Value::scalar(x.trunc()))
            }
            "sin" => Some(Value::scalar(self.trig(name, args, Num::sin, true)?)),
            "cos" => Some(Value::scalar(self.trig(name, args, Num::cos, true)?)),
            "tan" => Some(Value::scalar(self.trig(name, args, Num::tan, true)?)),
            "asin" => Some(Value::scalar(self.trig(name, args, Num::asin, false)?)),
            "acos" => Some(Value::scalar(self.trig(name, args, Num::acos, false)?)),
            "atan" => Some(Value::scalar(self.trig(name, args, Num::atan, false)?)),
            "sinh" => return unary(self, Num::sinh),
            "cosh" => return unary(self, Num::cosh),
            "tanh" => return unary(self, Num::tanh),
            "mod" => {
                let (a, b) = self.two_scalar(name, args)?;
                Some(Value::scalar(self.num_mod(&a, &b)))
            }
            "max" => {
                let (a, b) = self.two_scalar(name, args)?;
                let pick = if a.cmp(&b) == Some(std::cmp::Ordering::Less) { b } else { a };
                Some(Value::scalar(pick))
            }
            "min" => {
                let (a, b) = self.two_scalar(name, args)?;
                let pick = if a.cmp(&b) == Some(std::cmp::Ordering::Greater) { b } else { a };
                Some(Value::scalar(pick))
            }
            "hypot" => {
                let (a, b) = self.two_scalar(name, args)?;
                let s = a.mul(&a, &self.ctx).add(&b.mul(&b, &self.ctx), &self.ctx);
                Some(Value::scalar(s.sqrt(&self.ctx)))
            }
            "rand" => {
                let maxv = if args.is_empty() {
                    Num::one()
                } else {
                    self.one_scalar(name, args)?
                };
                Some(Value::scalar(self.rand(&maxv)))
            }
            _ => None,
        };
        Ok(r)
    }

    fn one_value(&mut self, name: &str, args: &[Expr]) -> Result<Value, String> {
        if args.len() != 1 {
            return Err(format!("function '{name}' expects 1 argument"));
        }
        self.eval(&args[0])
    }

    fn one_scalar(&mut self, name: &str, args: &[Expr]) -> Result<Num, String> {
        let v = self.one_value(name, args)?;
        v.require_scalar(name).cloned()
    }

    fn two_scalar(&mut self, name: &str, args: &[Expr]) -> Result<(Num, Num), String> {
        if args.len() != 2 {
            return Err(format!("function '{name}' expects 2 arguments"));
        }
        let a = self.eval(&args[0])?.require_scalar(name)?.clone();
        let b = self.eval(&args[1])?.require_scalar(name)?.clone();
        Ok((a, b))
    }

    fn trig(
        &mut self,
        name: &str,
        args: &[Expr],
        f: fn(&Num, &mut Ctx) -> Num,
        input_angle: bool,
    ) -> Result<Num, String> {
        let x = self.one_scalar(name, args)?;
        if self.angle == Angle::Deg && input_angle {
            // convert degrees -> radians
            let rad = x.mul(&self.deg_to_rad(), &self.ctx);
            Ok(f(&rad, &mut self.ctx))
        } else if self.angle == Angle::Deg && !input_angle {
            // result radians -> degrees
            let r = f(&x, &mut self.ctx);
            Ok(r.div(&self.deg_to_rad(), &self.ctx))
        } else {
            Ok(f(&x, &mut self.ctx))
        }
    }

    fn deg_to_rad(&mut self) -> Num {
        let pi = self.ctx.pi();
        pi.div(&Num::from_i64(180), &self.ctx)
    }

    fn factorial(&mut self, v: &Value) -> Result<Value, String> {
        let n = v.require_scalar("!")?;
        let i = n
            .to_i128()
            .ok_or_else(|| "factorial requires a non-negative integer".to_string())?;
        if i < 0 {
            return Err("factorial of a negative number".into());
        }
        if i > 50_000 {
            return Err("factorial argument too large".into());
        }
        let mut acc = Num::one();
        for k in 2..=i {
            acc = acc.mul(&Num::from_i128(k), &self.ctx);
        }
        Ok(Value::scalar(acc))
    }

    fn num_mod(&mut self, a: &Num, b: &Num) -> Num {
        let q = a.div(b, &self.ctx).trunc();
        a.sub(&b.mul(&q, &self.ctx), &self.ctx)
    }

    fn rand(&mut self, maxv: &Num) -> Num {
        // xorshift64
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.rng = x;
        let frac = (x >> 11) as f64 / (1u64 << 53) as f64; // [0,1)
        Num::from_f64(frac).mul(maxv, &self.ctx)
    }

    fn eval_binop(&mut self, op: &BinOp, lhs: Value, rhs: Value) -> Result<Value, String> {
        use BinOp::*;
        match op {
            Add => lhs.add(&rhs, &self.ctx),
            Sub => lhs.sub(&rhs, &self.ctx),
            Mul => Ok(lhs.mul(&rhs, &self.ctx)),
            Div => Ok(lhs.div(&rhs, &self.ctx)),
            Mod => {
                let a = lhs.require_scalar("%")?.clone();
                let b = rhs.require_scalar("%")?.clone();
                Ok(Value::scalar(self.num_mod(&a, &b)))
            }
            Pow => self.eval_pow(lhs, rhs),
            Shl | Shr | And | Or | Xor => {
                let a = lhs
                    .require_scalar("bitwise")?
                    .to_i128()
                    .ok_or("bitwise operators require integers")?;
                let b = rhs
                    .require_scalar("bitwise")?
                    .to_i128()
                    .ok_or("bitwise operators require integers")?;
                let r = match op {
                    Shl => a << b,
                    Shr => a >> b,
                    And => a & b,
                    Or => a | b,
                    Xor => a ^ b,
                    _ => unreachable!(),
                };
                Ok(Value::scalar(Num::from_i128(r)))
            }
            Lt | Le | Gt | Ge | Eq | Ne => {
                if lhs.dim != rhs.dim {
                    return Err("cannot compare incompatible quantities".into());
                }
                let ord = lhs.num.cmp(&rhs.num);
                let truth = match (op, ord) {
                    (Lt, Some(o)) => o == std::cmp::Ordering::Less,
                    (Le, Some(o)) => o != std::cmp::Ordering::Greater,
                    (Gt, Some(o)) => o == std::cmp::Ordering::Greater,
                    (Ge, Some(o)) => o != std::cmp::Ordering::Less,
                    (Eq, Some(o)) => o == std::cmp::Ordering::Equal,
                    (Ne, Some(o)) => o != std::cmp::Ordering::Equal,
                    _ => false,
                };
                Ok(Value::from_i64(if truth { 1 } else { 0 }))
            }
            LogAnd => {
                let a = !lhs.require_scalar("&&")?.is_zero();
                let b = !rhs.require_scalar("&&")?.is_zero();
                Ok(Value::from_i64(if a && b { 1 } else { 0 }))
            }
            LogOr => {
                let a = !lhs.require_scalar("||")?.is_zero();
                let b = !rhs.require_scalar("||")?.is_zero();
                Ok(Value::from_i64(if a || b { 1 } else { 0 }))
            }
        }
    }

    fn eval_pow(&mut self, base: Value, exp: Value) -> Result<Value, String> {
        let e = exp.require_scalar("^")?.clone();
        if base.is_scalar() {
            return Ok(Value::scalar(base.num.pow(&e, &mut self.ctx)));
        }
        // dimensioned base: exponent must be an integer
        let ei = e
            .to_i128()
            .ok_or("a dimensioned quantity may only be raised to an integer power")?;
        if ei < i8::MIN as i128 || ei > i8::MAX as i128 {
            return Err("exponent out of range for a dimensioned quantity".into());
        }
        Ok(Value {
            num: base.num.pow(&e, &mut self.ctx),
            dim: base.dim.powi(ei as i8),
        })
    }

    // ----------------------------- commands ------------------------------

    fn try_command(&mut self, s: &str) -> Result<Option<Eval>, String> {
        let trimmed = s.trim();
        let mut it = trimmed.splitn(2, char::is_whitespace);
        let head = it.next().unwrap_or("");
        let rest = it.next().unwrap_or("").trim();
        let lower = head.to_lowercase();
        let e = match lower.as_str() {
            "clear" | "cls" => Eval::Clear,
            "exit" | "quit" => Eval::Exit,
            "echo" => Eval::Message(rest.to_string()),
            "gospel" => Eval::Message(
                "For God so loved the world, that he gave his only Son, that whoever \
                 believes in him should not perish but have eternal life. (John 3:16)"
                    .to_string(),
            ),
            "help" => Eval::Message(
                "Commands: clear, del X, disp X, display [dec|hex|bin], echo, european [on|off], \
                 exit, list, mode [rad|deg], scimode [auto|never|always|eng|prefix|finance], \
                 sigfigs X, theme [name|list], prompt [inline|bottom], unit name = expr"
                    .to_string(),
            ),
            "list" | "vars" => Eval::Message(self.list_definitions()),
            "del" | "rem" => {
                self.cmd_del(rest)?;
                Eval::Message(if rest == "all" {
                    "all variables deleted".into()
                } else {
                    format!("deleted '{rest}'")
                })
            }
            "disp" => {
                let v = self
                    .vars
                    .get(rest)
                    .cloned()
                    .or_else(|| self.units.get(rest).cloned())
                    .ok_or_else(|| format!("'{rest}' is not defined"))?;
                Eval::Named(rest.to_string(), v)
            }
            "display" => {
                self.settings.base = match rest.to_lowercase().as_str() {
                    "dec" | "decimal" => Base::Dec,
                    "hex" | "hexadecimal" => Base::Hex,
                    "bin" | "binary" => Base::Bin,
                    other => return Err(format!("unknown base '{other}'")),
                };
                Eval::Message(format!("display base set to {rest}"))
            }
            "mode" => {
                self.angle = match rest.to_lowercase().as_str() {
                    "rad" | "radian" | "radians" => Angle::Rad,
                    "deg" | "degree" | "degrees" => Angle::Deg,
                    other => return Err(format!("unknown mode '{other}'")),
                };
                Eval::Message(format!("angle mode set to {rest}"))
            }
            "scimode" => {
                self.settings.sci = match rest.to_lowercase().as_str() {
                    "auto" => SciMode::Auto,
                    "never" => SciMode::Never,
                    "always" => SciMode::Always,
                    "eng" => SciMode::Eng,
                    "prefix" => SciMode::Prefix,
                    "finance" => SciMode::Finance,
                    other => return Err(format!("unknown scimode '{other}'")),
                };
                Eval::Message(format!("scientific mode set to {rest}"))
            }
            "sigfigs" => {
                let n: usize = rest.parse().map_err(|_| "sigfigs needs a number")?;
                self.settings.sigfigs = n;
                Eval::Message(format!("significant figures set to {n}"))
            }
            "european" => {
                let on = matches!(rest.to_lowercase().as_str(), "on" | "true" | "1");
                self.set_european(on);
                Eval::Message(format!("european delimiters {}", if on { "on" } else { "off" }))
            }
            "theme" => self.cmd_theme(rest),
            "prompt" | "layout" => self.cmd_prompt(rest),
            "unit" => self.cmd_define_unit(rest)?,
            _ => return Ok(None),
        };
        Ok(Some(e))
    }

    fn cmd_theme(&mut self, rest: &str) -> Eval {
        let arg = rest.trim();
        if arg.is_empty() {
            return Eval::Message(format!(
                "current theme: {}  (use 'theme list' to see all)",
                self.theme
            ));
        }
        if arg.eq_ignore_ascii_case("list") {
            let mut lines = vec!["Available themes:".to_string()];
            for t in theme::THEMES {
                let kind = if t.dark { "dark" } else { "light" };
                let marker = if t.name == self.theme { "*" } else { " " };
                lines.push(format!("  {marker} {} ({kind})", t.name));
            }
            return Eval::Message(lines.join("\n"));
        }
        match theme::find(arg) {
            Some(t) => {
                self.theme = t.name.to_string();
                Eval::Message(format!("theme set to {}", t.name))
            }
            None => Eval::Message(format!(
                "Error: unknown theme '{arg}' (try 'theme list')"
            )),
        }
    }

    fn cmd_prompt(&mut self, rest: &str) -> Eval {
        let arg = rest.trim().to_lowercase();
        match arg.as_str() {
            "" => {
                let cur = match self.input_layout {
                    InputLayout::Bottom => "bottom",
                    InputLayout::Inline => "inline",
                };
                Eval::Message(format!(
                    "prompt layout: {cur}  (use 'prompt inline' or 'prompt bottom')"
                ))
            }
            "inline" => {
                self.input_layout = InputLayout::Inline;
                Eval::Message("prompt layout set to inline".to_string())
            }
            "bottom" => {
                self.input_layout = InputLayout::Bottom;
                Eval::Message("prompt layout set to bottom".to_string())
            }
            other => Eval::Message(format!(
                "Error: unknown prompt layout '{other}' (use 'inline' or 'bottom')"
            )),
        }
    }

    pub fn set_european(&mut self, on: bool) {
        self.european = on;
        if on {
            self.settings.decimal_char = ',';
            if self.settings.thousands == Some(',') {
                self.settings.thousands = Some('.');
            }
        } else {
            self.settings.decimal_char = '.';
            if self.settings.thousands == Some('.') {
                self.settings.thousands = Some(',');
            }
        }
    }

    fn cmd_del(&mut self, target: &str) -> Result<(), String> {
        if target == "all" {
            self.vars.clear();
            self.def_records.retain(|d| d.kind != DefKind::Var);
            return Ok(());
        }
        self.def_records.retain(|d| d.name != target);
        let removed = self.vars.remove(target).is_some()
            | self.funcs.remove(target).is_some()
            | (self.custom_units.contains(&target.to_string()) && {
                self.units.remove(target);
                self.custom_units.retain(|u| u != target);
                true
            });
        if removed {
            Ok(())
        } else {
            Err(format!("'{target}' is not defined"))
        }
    }

    fn cmd_define_unit(&mut self, rest: &str) -> Result<Eval, String> {
        // syntax: unit name = expr
        let (name, expr) = rest
            .split_once('=')
            .ok_or("usage: unit name = expression")?;
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("unit name is required".into());
        }
        let expr = expr.trim();
        let stmt = parse_stmt(expr)?;
        let value = match stmt {
            Stmt::Expr(e) => self.eval(&e)?,
            _ => return Err("unit definition must be an expression".into()),
        };
        self.units.insert(name.clone(), value);
        if !self.custom_units.contains(&name) {
            self.custom_units.push(name.clone());
        }
        self.record_def(DefKind::Unit, &name, format!("unit {rest}"));
        Ok(Eval::Message("new unit created!".to_string()))
    }

    fn list_definitions(&self) -> String {
        let mut lines = Vec::new();
        let mut vars: Vec<_> = self.vars.iter().filter(|(k, _)| k.as_str() != "ans").collect();
        vars.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in vars {
            lines.push(format!("  {k} = {}", format_value(v, &self.settings)));
        }
        let mut funcs: Vec<_> = self.funcs.keys().cloned().collect();
        funcs.sort();
        for f in funcs {
            let (params, _) = &self.funcs[&f];
            lines.push(format!("  {f}({}) [function]", params.join(",")));
        }
        let mut cu = self.custom_units.clone();
        cu.sort();
        for u in cu {
            lines.push(format!("  {u} [unit]"));
        }
        if lines.is_empty() {
            "(no user definitions)".to_string()
        } else {
            lines.join("\n")
        }
    }
}

/// Split a line into `(statement, suppressed_by_semicolon)` pairs.
fn split_statements(line: &str) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for ch in line.chars() {
        if ch == ';' {
            out.push((cur.clone(), true));
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    if !cur.trim().is_empty() {
        out.push((cur, false));
    }
    out
}

