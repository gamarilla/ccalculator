//! High-precision number type, a thin ergonomic wrapper over `astro_float::BigFloat`.
//!
//! All arithmetic flows through a [`Ctx`] which owns the constants cache and the
//! working precision (in bits). The original Console Calculator advertised "352
//! bits of arithmetic accuracy"; we default to 384 bits of working precision so
//! results round cleanly at that level.

use astro_float::{BigFloat, Consts, Radix, RoundingMode, Sign};

/// Working precision, in bits. ~115 decimal digits.
pub const WORKING_PREC: usize = 384;
/// Rounding mode used for every operation.
pub const RM: RoundingMode = RoundingMode::ToEven;

/// Evaluation context: owns the (mutable) constants cache and precision.
pub struct Ctx {
    pub consts: Consts,
    pub prec: usize,
}

impl Default for Ctx {
    fn default() -> Self {
        Ctx {
            consts: Consts::new().expect("failed to init constants cache"),
            prec: WORKING_PREC,
        }
    }
}

impl Ctx {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pi(&mut self) -> Num {
        Num(self.consts.pi(self.prec, RM))
    }

    pub fn e(&mut self) -> Num {
        Num(self.consts.e(self.prec, RM))
    }
}

/// A high-precision real number.
#[derive(Clone, Debug)]
pub struct Num(pub BigFloat);

impl Num {
    pub fn raw(&self) -> &BigFloat {
        &self.0
    }

    pub fn from_f64(f: f64) -> Num {
        Num(BigFloat::from_f64(f, WORKING_PREC))
    }

    pub fn from_i64(i: i64) -> Num {
        Num(BigFloat::from_i64(i, WORKING_PREC))
    }

    pub fn from_i128(i: i128) -> Num {
        Num(BigFloat::from_i128(i, WORKING_PREC))
    }

    pub fn zero() -> Num {
        Num::from_i64(0)
    }

    pub fn one() -> Num {
        Num::from_i64(1)
    }

    /// Parse a numeric literal in the given radix (10, 16, or 2).
    pub fn parse(s: &str, radix: u32, ctx: &mut Ctx) -> Result<Num, String> {
        let rdx = match radix {
            10 => Radix::Dec,
            16 => Radix::Hex,
            2 => Radix::Bin,
            8 => Radix::Oct,
            _ => return Err(format!("unsupported radix {radix}")),
        };
        let bf = BigFloat::parse(s, rdx, ctx.prec, RM, &mut ctx.consts);
        if bf.is_nan() {
            return Err(format!("invalid number literal: {s}"));
        }
        Ok(Num(bf))
    }

    pub fn is_nan(&self) -> bool {
        self.0.is_nan()
    }

    pub fn is_inf(&self) -> bool {
        self.0.is_inf()
    }

    pub fn is_zero(&self) -> bool {
        self.0.is_zero()
    }

    pub fn is_negative(&self) -> bool {
        matches!(self.0.sign(), Some(Sign::Neg))
    }

    /// Convert to f64 (lossy) — used for small-integer extraction and tests.
    pub fn to_f64(&self) -> f64 {
        if self.0.is_nan() {
            return f64::NAN;
        }
        if self.0.is_inf() {
            return if self.is_negative() {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        let mut cc = match Consts::new() {
            Ok(c) => c,
            Err(_) => return f64::NAN,
        };
        match self.0.format(Radix::Dec, RM, &mut cc) {
            Ok(s) => parse_af_decimal_f64(&s),
            Err(_) => f64::NAN,
        }
    }

    /// Try to interpret as an exact i128 (returns None if not integral / out of range).
    pub fn to_i128(&self) -> Option<i128> {
        if self.0.is_nan() || self.0.is_inf() {
            return None;
        }
        let truncated = self.0.floor();
        // must be integral
        if self.0.sub(&truncated, WORKING_PREC, RM).is_zero() {
            // round() returns nearest; for an integral value this is exact
            let s = self
                .0
                .format(Radix::Dec, RM, &mut Consts::new().ok()?)
                .ok()?;
            // strip any exponent / fraction
            parse_decimal_to_i128(&s)
        } else {
            None
        }
    }

    // ----- arithmetic -----
    pub fn add(&self, o: &Num, ctx: &Ctx) -> Num {
        Num(self.0.add(&o.0, ctx.prec, RM))
    }
    pub fn sub(&self, o: &Num, ctx: &Ctx) -> Num {
        Num(self.0.sub(&o.0, ctx.prec, RM))
    }
    pub fn mul(&self, o: &Num, ctx: &Ctx) -> Num {
        Num(self.0.mul(&o.0, ctx.prec, RM))
    }
    pub fn div(&self, o: &Num, ctx: &Ctx) -> Num {
        Num(self.0.div(&o.0, ctx.prec, RM))
    }
    pub fn neg(&self) -> Num {
        Num(self.0.neg())
    }
    pub fn abs(&self) -> Num {
        Num(self.0.abs())
    }
    pub fn pow(&self, o: &Num, ctx: &mut Ctx) -> Num {
        Num(self.0.pow(&o.0, ctx.prec, RM, &mut ctx.consts))
    }
    pub fn sqrt(&self, ctx: &Ctx) -> Num {
        Num(self.0.sqrt(ctx.prec, RM))
    }
    pub fn floor(&self) -> Num {
        Num(self.0.floor())
    }
    pub fn ceil(&self) -> Num {
        Num(self.0.ceil())
    }
    pub fn round(&self) -> Num {
        // round to 0 fractional digits, nearest
        Num(self.0.round(0, RoundingMode::ToEven))
    }

    /// Truncate toward zero.
    pub fn trunc(&self) -> Num {
        if self.is_negative() {
            Num(self.0.ceil())
        } else {
            Num(self.0.floor())
        }
    }

    // ----- transcendental -----
    pub fn sin(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.sin(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn cos(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.cos(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn tan(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.tan(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn asin(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.asin(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn acos(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.acos(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn atan(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.atan(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn sinh(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.sinh(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn cosh(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.cosh(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn tanh(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.tanh(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn ln(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.ln(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn log2(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.log2(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn log10(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.log10(ctx.prec, RM, &mut ctx.consts))
    }
    pub fn exp(&self, ctx: &mut Ctx) -> Num {
        Num(self.0.exp(ctx.prec, RM, &mut ctx.consts))
    }

    /// log base `b` of self = ln(self)/ln(b)
    pub fn log_base(&self, b: &Num, ctx: &mut Ctx) -> Num {
        let num = self.ln(ctx);
        let den = b.ln(ctx);
        num.div(&den, ctx)
    }

    /// Compare two numbers. Returns Ordering via i128 sign of cmp.
    #[allow(clippy::should_implement_trait)]
    pub fn cmp(&self, o: &Num) -> Option<std::cmp::Ordering> {
        self.0.cmp(&o.0).map(|c| c.cmp(&0))
    }

    /// Decompose into a sign, a string of significant decimal digits (no point,
    /// no leading zeros), and a power-of-ten exponent such that
    /// `value = (neg ? -1 : 1) * digits * 10^exp10`.
    pub fn decompose(&self) -> Option<DecParts> {
        if self.0.is_nan() || self.0.is_inf() {
            return None;
        }
        let mut cc = Consts::new().ok()?;
        let s = self.0.format(Radix::Dec, RM, &mut cc).ok()?;
        DecParts::parse_astro(&s)
    }
}

/// Decimal decomposition: `value = sign * digits * 10^exp10`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecParts {
    pub neg: bool,
    pub digits: String,
    pub exp10: i64,
}

impl DecParts {
    pub fn is_zero(&self) -> bool {
        self.digits.bytes().all(|b| b == b'0')
    }

    /// Parse astro-float's normalized decimal form, e.g. "1.23456789e+8" or "2.e+0".
    pub fn parse_astro(s: &str) -> Option<DecParts> {
        let s = s.trim();
        let neg = s.starts_with('-');
        let s = s.trim_start_matches(['+', '-']);
        let (mant, exp) = match s.split_once(['e', 'E']) {
            Some((m, e)) => (m, e.replace('+', "").parse::<i64>().ok()?),
            None => (s, 0i64),
        };
        let (before, after) = match mant.split_once('.') {
            Some((b, a)) => (b, a),
            None => (mant, ""),
        };
        let mut digits = String::new();
        digits.push_str(before);
        digits.push_str(after);
        // exponent of the leading digit is `exp`; trailing digits are fractional
        let frac_len = after.len() as i64;
        let mut exp10 = exp - frac_len;
        // strip trailing zeros (fold into exp10)
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
            exp10 += 1;
        }
        // strip leading zeros
        let trimmed = digits.trim_start_matches('0');
        let digits = if trimmed.is_empty() {
            "0".to_string()
        } else {
            trimmed.to_string()
        };
        if digits == "0" {
            return Some(DecParts {
                neg: false,
                digits,
                exp10: 0,
            });
        }
        Some(DecParts {
            neg,
            digits,
            exp10,
        })
    }

    /// Round to at most `sig` significant digits (in place semantics, returns new).
    pub fn round_sig(&self, sig: usize) -> DecParts {
        if sig == 0 || self.digits.len() <= sig || self.is_zero() {
            return self.clone();
        }
        let bytes = self.digits.as_bytes();
        let keep = &bytes[..sig];
        let round_up = bytes[sig] >= b'5';
        let dropped = self.digits.len() - sig;
        let mut exp10 = self.exp10 + dropped as i64;
        let mut kept: Vec<u8> = keep.to_vec();
        if round_up {
            let mut i = sig as isize - 1;
            loop {
                if i < 0 {
                    kept.insert(0, b'1');
                    exp10 += 1;
                    // drop the now-extra least significant digit to keep `sig` length
                    kept.pop();
                    exp10 += 1;
                    break;
                }
                if kept[i as usize] == b'9' {
                    kept[i as usize] = b'0';
                    i -= 1;
                } else {
                    kept[i as usize] += 1;
                    break;
                }
            }
        }
        let mut digits = String::from_utf8(kept).unwrap();
        // re-strip trailing zeros
        while digits.len() > 1 && digits.ends_with('0') {
            digits.pop();
            exp10 += 1;
        }
        DecParts {
            neg: self.neg,
            digits,
            exp10,
        }
    }
}

/// Parse an astro-float decimal format string into f64, tolerating its exponent
/// notation. astro-float prints decimals like "6.283e+0"; normalize and parse.
pub fn parse_af_decimal_f64(s: &str) -> f64 {
    let t = s.trim();
    if let Ok(v) = t.parse::<f64>() {
        return v;
    }
    // Normalize possible separators ('_' used for hex exponents) and stray spaces.
    let cleaned: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    cleaned.parse::<f64>().unwrap_or(f64::NAN)
}

/// Parse a plain decimal string (possibly with exponent) into i128 if it is integral.
fn parse_decimal_to_i128(s: &str) -> Option<i128> {
    let s = s.trim();
    // astro-float dec format may look like "1.23e+2" or "123" or "-0.0"
    let (mantissa, exp) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m.to_string(), e.parse::<i64>().ok()?),
        None => (s.to_string(), 0i64),
    };
    let neg = mantissa.starts_with('-');
    let mantissa = mantissa.trim_start_matches(['+', '-']);
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((i, f)) => (i.to_string(), f.to_string()),
        None => (mantissa.to_string(), String::new()),
    };
    // assemble digits and effective exponent
    let digits: String = format!("{int_part}{frac_part}");
    let point_from_right = frac_part.len() as i64 - exp;
    if point_from_right > 0 {
        // there is a fractional component; must be all zeros to be integral
        let cut = digits.len() as i64 - point_from_right;
        if cut < 0 {
            return None;
        }
        let (keep, frac) = digits.split_at(cut as usize);
        if frac.bytes().any(|b| b != b'0') {
            return None;
        }
        let v: i128 = keep.parse().ok().or(Some(0))?;
        return Some(if neg { -v } else { v });
    }
    // need to append zeros
    let mut d = digits;
    for _ in 0..(-point_from_right) {
        d.push('0');
    }
    let v: i128 = d.parse().ok()?;
    Some(if neg { -v } else { v })
}
